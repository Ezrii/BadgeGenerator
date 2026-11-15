use eframe::egui;
use ratpbadge::{badges, excel};
use std::path::PathBuf;

struct App {
    excel_path: Option<PathBuf>,
    sheets: Vec<String>,
    sheet: String,
    people: Vec<excel::Person>,
    club_logo_path: Option<PathBuf>,
    club_logo: Option<image::DynamicImage>,
    status: String,
    status_is_error: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            excel_path: None,
            sheets: Vec::new(),
            sheet: String::new(),
            people: Vec::new(),
            club_logo_path: None,
            club_logo: None,
            status: "Sélectionnez le fichier Excel source.".into(),
            status_is_error: false,
        }
    }
}

impl App {
    fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        self.status = msg.into();
        self.status_is_error = error;
    }

    fn open_excel(&mut self, path: PathBuf) {
        match excel::sheet_names(&path) {
            Ok(sheets) => {
                self.sheet = sheets.first().cloned().unwrap_or_default();
                self.sheets = sheets;
                self.excel_path = Some(path);
                self.reload_sheet();
            }
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }

    fn reload_sheet(&mut self) {
        let Some(path) = self.excel_path.clone() else { return };
        match excel::load_people(&path, &self.sheet) {
            Ok(people) => {
                let msg = format!("{} personnes trouvées.", people.len());
                self.people = people;
                self.set_status(msg, false);
            }
            Err(e) => {
                self.people.clear();
                self.set_status(format!("{e:#}"), true);
            }
        }
    }

    fn open_logo(&mut self, path: PathBuf) {
        match badges::load_logo(&path) {
            Ok(img) => {
                self.club_logo = Some(img);
                self.club_logo_path = Some(path);
                self.set_status("Logo remplacé.", false);
            }
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }

    fn export_pdf(&mut self) {
        let Some(out) = rfd::FileDialog::new()
            .set_file_name("badges.pdf")
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            return;
        };
        match badges::generate_pdf(&self.people, self.club_logo.as_ref()) {
            Ok(bytes) => match std::fs::write(&out, bytes) {
                Ok(()) => self.set_status(
                    format!("PDF créé : {} ({} badges).", out.display(), self.people.len()),
                    false,
                ),
                Err(e) => self.set_status(format!("Écriture impossible : {e}"), true),
            },
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Générateur de badges RATP");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("1. Fichier Excel…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Excel", &["xlsx", "xlsm", "xls", "ods"])
                        .pick_file()
                    {
                        self.open_excel(p);
                    }
                }
                match &self.excel_path {
                    Some(p) => ui.label(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                    None => ui.weak("aucun fichier"),
                };
            });

            if self.sheets.len() > 1 {
                let before = self.sheet.clone();
                egui::ComboBox::from_label("Feuille")
                    .selected_text(self.sheet.clone())
                    .show_ui(ui, |ui| {
                        for s in &self.sheets {
                            ui.selectable_value(&mut self.sheet, s.clone(), s);
                        }
                    });
                if self.sheet != before {
                    self.reload_sheet();
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("2. Logo à droite…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.open_logo(p);
                    }
                }
                if self.club_logo.is_some() && ui.button("Logo par défaut").clicked() {
                    self.club_logo = None;
                    self.club_logo_path = None;
                    self.set_status("Logo par défaut restauré.", false);
                }
                match &self.club_logo_path {
                    Some(p) => ui.label(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                    None => ui.weak("logo Club des CSP par défaut"),
                };
            });
            ui.weak("Le logo RATP à gauche reste toujours le même.");

            ui.add_space(8.0);
            ui.separator();
            ui.label(format!("Personnes : {}", self.people.len()));
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                egui::Grid::new("people").striped(true).show(ui, |ui| {
                    ui.strong("Service");
                    ui.strong("Nom");
                    ui.end_row();
                    for p in &self.people {
                        ui.label(&p.service);
                        ui.label(&p.name);
                        ui.end_row();
                    }
                });
            });

            ui.separator();
            ui.add_space(4.0);
            let can_export = !self.people.is_empty();
            if ui
                .add_enabled(can_export, egui::Button::new("3. Générer le PDF…"))
                .clicked()
            {
                self.export_pdf();
            }

            ui.add_space(6.0);
            let color = if self.status_is_error {
                egui::Color32::from_rgb(200, 40, 40)
            } else {
                ui.visuals().text_color()
            };
            ui.colored_label(color, &self.status);
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([620.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Générateur de badges RATP",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
