//! Mise en page et génération du PDF A4 des badges (88,9 × 50,8 mm, 10 par page).

use crate::excel::Person;
use anyhow::{anyhow, ensure, Context, Result};
use image::{DynamicImage, GenericImageView};
use printpdf::{
    path::{PaintMode, WindingOrder},
    Color, Image, ImageTransform, ImageXObject, Line, LineDashPattern, Mm, PdfDocument,
    PdfLayerReference, Point, Rect, Rgb,
};
use std::io::Cursor;

/// Logo RATP Groupe, toujours présent à gauche (fixe).
pub const RATP_LOGO_PNG: &[u8] = include_bytes!("../assets/ratp_logo.png");
/// Logo « Club des CSP » par défaut, à droite (remplaçable dans l'interface).
pub const CLUB_LOGO_PNG: &[u8] = include_bytes!("../assets/club_csp_logo.png");
/// Police Liberation Sans Bold (licence SIL OFL 1.1).
const FONT_BOLD: &[u8] = include_bytes!("../assets/LiberationSans-Bold.ttf");

const PT_PER_MM: f32 = 72.0 / 25.4;
const PAGE_W: f32 = 210.0 * PT_PER_MM;
const PAGE_H: f32 = 297.0 * PT_PER_MM;
const BADGE_W: f32 = 88.9 * PT_PER_MM;
const BADGE_H: f32 = 50.8 * PT_PER_MM;

const NAME_MAX_PT: f32 = 17.5;
const LABEL_MAX_PT: f32 = 10.0;
const NAME_PAD: f32 = 14.0;
const LABEL_PAD: f32 = 8.0;
const BAND_H: f32 = 22.0;
const SEPARATOR_H: f32 = 2.5;
const LOGO_PAD: f32 = 10.0;
const RATP_LOGO_H: f32 = 30.0;
const CLUB_LOGO_H: f32 = 32.0;
/// Zone réservée aux logos en haut du badge ; le nom est centré sous cette zone.
const LOGO_ZONE_H: f32 = 36.0;
const LOGO_MAX_PX: u32 = 200;
const IMAGE_DPI: f32 = 300.0;

const BLUE: (f32, f32, f32) = (0.0, 79.0 / 255.0, 163.0 / 255.0);
const TEAL: (f32, f32, f32) = (75.0 / 255.0, 192.0 / 255.0, 173.0 / 255.0);
const DARK: (f32, f32, f32) = (0.12, 0.12, 0.15);
const GREY: (f32, f32, f32) = (0.55, 0.55, 0.58);
const WHITE: (f32, f32, f32) = (1.0, 1.0, 1.0);

/// Nombre de badges par page et position de chaque badge (coin bas-gauche, en points).
pub fn grid() -> (usize, usize, Vec<(f32, f32)>) {
    let cols = (PAGE_W / BADGE_W).floor() as usize;
    let rows = (PAGE_H / BADGE_H).floor() as usize;
    let x0 = (PAGE_W - cols as f32 * BADGE_W) / 2.0;
    let top = PAGE_H - (PAGE_H - rows as f32 * BADGE_H) / 2.0;
    let mut slots = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let x = x0 + col as f32 * BADGE_W;
            let y = top - (row as f32 + 1.0) * BADGE_H;
            slots.push((x, y));
        }
    }
    (cols, rows, slots)
}

/// Largeur d'un texte en points, pour une taille de police donnée.
fn text_width(face: &ttf_parser::Face, text: &str, size: f32) -> f32 {
    let upem = face.units_per_em() as f32;
    let units: f32 = text
        .chars()
        .map(|ch| {
            face.glyph_index(ch)
                .and_then(|g| face.glyph_hor_advance(g))
                .unwrap_or(0) as f32
        })
        .sum();
    units * size / upem
}

/// Taille unique qui fait tenir tous les textes dans la largeur donnée (plafonnée).
fn fit_size(face: &ttf_parser::Face, texts: &[&str], max_pt: f32, max_w: f32) -> f32 {
    texts.iter().fold(max_pt, |size, t| {
        let w1 = text_width(face, t, 1.0);
        if w1 <= 0.0 {
            size
        } else {
            size.min(max_w / w1)
        }
    })
}

fn mm(pt: f32) -> Mm {
    Mm(pt / PT_PER_MM)
}

fn rgb((r, g, b): (f32, f32, f32)) -> Color {
    Color::Rgb(Rgb::new(r, g, b, None))
}

fn rect(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32, fill: (f32, f32, f32)) {
    layer.set_fill_color(rgb(fill));
    layer.add_rect(Rect {
        ll: Point::new(mm(x), mm(y)),
        ur: Point::new(mm(x + w), mm(y + h)),
        mode: PaintMode::Fill,
        winding: WindingOrder::NonZero,
    });
}

fn cut_line(layer: &PdfLayerReference, x: f32, y: f32, w: f32, h: f32) {
    layer.set_outline_color(rgb(GREY));
    layer.set_outline_thickness(0.5);
    layer.set_line_dash_pattern(LineDashPattern {
        dash_1: Some(3),
        gap_1: Some(3),
        ..Default::default()
    });
    layer.add_line(Line {
        points: vec![
            (Point::new(mm(x), mm(y)), false),
            (Point::new(mm(x + w), mm(y)), false),
            (Point::new(mm(x + w), mm(y + h)), false),
            (Point::new(mm(x), mm(y + h)), false),
        ],
        is_closed: true,
    });
    layer.set_line_dash_pattern(LineDashPattern::default());
}

fn text(
    layer: &PdfLayerReference,
    font: &printpdf::IndirectFontRef,
    s: &str,
    size: f32,
    x: f32,
    baseline: f32,
    color: (f32, f32, f32),
) {
    layer.set_fill_color(rgb(color));
    layer.use_text(s, size, mm(x), mm(baseline), font);
}

/// Place une image dont la hauteur finale est `h`, bas-gauche en (x, y). Retourne sa largeur.
fn place_image(layer: &PdfLayerReference, img: &ImageXObject, px: (u32, u32), x: f32, y: f32, h: f32) -> f32 {
    let natural_h = px.1 as f32 * 72.0 / IMAGE_DPI;
    let natural_w = px.0 as f32 * 72.0 / IMAGE_DPI;
    let scale = h / natural_h;
    Image::from(img.clone()).add_to_layer(
        layer.clone(),
        ImageTransform {
            translate_x: Some(mm(x)),
            translate_y: Some(mm(y)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            dpi: Some(IMAGE_DPI),
            ..Default::default()
        },
    );
    natural_w * scale
}

/// Décode un logo et le réduit pour garder un PDF léger.
pub fn prepare_logo(img: &DynamicImage) -> DynamicImage {
    img.thumbnail(LOGO_MAX_PX, LOGO_MAX_PX)
}

/// Charge un logo depuis un fichier (PNG, JPEG) et le prépare.
pub fn load_logo(path: &std::path::Path) -> Result<DynamicImage> {
    let img = image::open(path).with_context(|| format!("Logo illisible : {}", path.display()))?;
    Ok(prepare_logo(&img))
}

/// Génère le PDF complet. `club_logo` remplace le logo « Club des CSP » par défaut.
pub fn generate_pdf(people: &[Person], club_logo: Option<&DynamicImage>) -> Result<Vec<u8>> {
    ensure!(!people.is_empty(), "Aucune personne à imprimer dans ce fichier");

    let face = ttf_parser::Face::parse(FONT_BOLD, 0).context("Police intégrée illisible")?;
    let names: Vec<&str> = people.iter().map(|p| p.name.as_str()).collect();
    let labels: Vec<&str> = people.iter().map(|p| p.service.as_str()).collect();
    let name_size = fit_size(&face, &names, NAME_MAX_PT, BADGE_W - 2.0 * NAME_PAD);
    let label_size = fit_size(&face, &labels, LABEL_MAX_PT, BADGE_W - 2.0 * LABEL_PAD);

    let ratp = prepare_logo(&image::load_from_memory(RATP_LOGO_PNG)?);
    let club = match club_logo {
        Some(img) => prepare_logo(img),
        None => prepare_logo(&image::load_from_memory(CLUB_LOGO_PNG)?),
    };

    let (doc, first_page, first_layer) =
        PdfDocument::new("Badges", Mm(210.0), Mm(297.0), "Badges");
    let font = doc
        .add_external_font(Cursor::new(FONT_BOLD))
        .map_err(|e| anyhow!("Impossible d'intégrer la police : {e:?}"))?;

    let (_cols, _rows, slots) = grid();
    let per_page = slots.len();
    let mut page = first_page;
    let mut layer_ref = first_layer;

    for (page_index, chunk) in people.chunks(per_page).enumerate() {
        if page_index > 0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Badges");
            page = p;
            layer_ref = l;
        }
        let layer = doc.get_page(page).get_layer(layer_ref);
        let ratp_img = ImageXObject::from_dynamic_image(&ratp);
        let club_img = ImageXObject::from_dynamic_image(&club);
        let ratp_px = ratp.dimensions();
        let club_px = club.dimensions();

        for (slot, person) in slots.iter().zip(chunk.iter()) {
            let (x, y) = *slot;

            cut_line(&layer, x, y, BADGE_W, BADGE_H);

            // Barre basse bleue : intitulé de la colonne.
            rect(&layer, x, y, BADGE_W, BAND_H, BLUE);
            rect(&layer, x, y + BAND_H, BADGE_W, SEPARATOR_H, TEAL);
            let lw = text_width(&face, &person.service, label_size);
            text(
                &layer,
                &font,
                &person.service,
                label_size,
                x + (BADGE_W - lw) / 2.0,
                y + BAND_H / 2.0 - label_size * 0.35,
                WHITE,
            );

            // Logos : RATP à gauche (fixe), club à droite (remplaçable).
            place_image(
                &layer,
                &ratp_img,
                ratp_px,
                x + LOGO_PAD,
                y + BADGE_H - RATP_LOGO_H - 6.0,
                RATP_LOGO_H,
            );
            let club_w = club_px.0 as f32 * CLUB_LOGO_H / club_px.1 as f32;
            place_image(
                &layer,
                &club_img,
                club_px,
                x + BADGE_W - club_w - LOGO_PAD,
                y + BADGE_H - CLUB_LOGO_H - 4.0,
                CLUB_LOGO_H,
            );

            // Nom centré dans la zone sous les logos.
            let top = y + BADGE_H - LOGO_ZONE_H;
            let bottom = y + BAND_H + SEPARATOR_H;
            let cy = (top + bottom) / 2.0;
            let nw = text_width(&face, &person.name, name_size);
            text(
                &layer,
                &font,
                &person.name,
                name_size,
                x + (BADGE_W - nw) / 2.0,
                cy - name_size * 0.35,
                DARK,
            );
        }
    }

    doc.save_to_bytes()
        .map_err(|e| anyhow!("Écriture du PDF impossible : {e:?}"))
}
