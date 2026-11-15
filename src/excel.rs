//! Lecture du fichier Excel source des participants et intervenants.
//!
//! Format attendu (par feuille) :
//! - la ligne 1 contient l'intitulé (service) de chaque colonne ;
//! - les lignes suivantes contiennent les personnes de chaque colonne ;
//! - une ligne « INTERVENANTS » sépare le bloc des intervenants, dont la ligne
//!   d'entête (entreprises / services) est la première ligne non vide qui suit.

use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

/// Marqueur qui sépare les participants des intervenants.
pub const INTERVENANTS_MARKER: &str = "INTERVENANTS";

/// Une personne à badger, avec l'intitulé de son service ou de sa colonne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub name: String,
    pub service: String,
}

/// Noms des feuilles du classeur.
pub fn sheet_names(path: &Path) -> Result<Vec<String>> {
    let wb = open_workbook_auto(path)
        .with_context(|| format!("Impossible d'ouvrir {}", path.display()))?;
    Ok(wb.sheet_names().to_vec())
}

/// Lit une feuille et renvoie ses lignes sous forme de texte.
pub fn read_rows(path: &Path, sheet: &str) -> Result<Vec<Vec<String>>> {
    let mut wb = open_workbook_auto(path)
        .with_context(|| format!("Impossible d'ouvrir {}", path.display()))?;
    let range = wb
        .worksheet_range(sheet)
        .map_err(|e| anyhow!("Feuille « {sheet} » illisible : {e}"))?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
}

/// Lit une feuille et extrait les personnes avec leur service.
pub fn load_people(path: &Path, sheet: &str) -> Result<Vec<Person>> {
    let rows = read_rows(path, sheet)?;
    Ok(parse_people(&rows))
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// Normalise un nom : espaces multiples et bords supprimés.
pub fn normalize_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extrait les personnes d'une feuille déjà lue.
pub fn parse_people(rows: &[Vec<String>]) -> Vec<Person> {
    let mut people = Vec::new();
    if rows.is_empty() {
        return people;
    }

    let marker = rows.iter().position(|row| {
        row.first()
            .map(|c| c.trim().eq_ignore_ascii_case(INTERVENANTS_MARKER))
            .unwrap_or(false)
    });

    // Bloc principal : entête en ligne 1, personnes en dessous jusqu'au marqueur.
    let main_end = marker.unwrap_or(rows.len());
    collect_block(&rows[0], &rows[1..main_end], &mut people);

    // Bloc intervenants : entête = première ligne non vide après le marqueur.
    if let Some(m) = marker {
        if let Some(h) = (m + 1..rows.len()).find(|&i| rows[i].iter().any(|c| !c.is_empty())) {
            collect_block(&rows[h], &rows[h + 1..], &mut people);
        }
    }

    people
}

fn collect_block(header: &[String], data: &[Vec<String>], out: &mut Vec<Person>) {
    for (j, service) in header.iter().enumerate() {
        if service.is_empty() {
            continue;
        }
        for row in data {
            let name = row.get(j).map(|c| normalize_name(c)).unwrap_or_default();
            if !name.is_empty() {
                out.push(Person {
                    name,
                    service: service.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_columns_and_speakers() {
        let rows = vec![
            row(&["DSP", "CSP RH"]),
            row(&["Alice Martin", "Bob  Dupont"]),
            row(&["Chloé Durand", ""]),
            row(&["INTERVENANTS", ""]),
            row(&[]),
            row(&["DELOITTE", "AMAZON"]),
            row(&["Carl Ray", "Dana Lee"]),
        ];
        let people = parse_people(&rows);
        assert_eq!(people.len(), 5);
        assert_eq!(people[0], Person { name: "Alice Martin".into(), service: "DSP".into() });
        assert_eq!(people[2], Person { name: "Bob Dupont".into(), service: "CSP RH".into() });
        assert_eq!(people[4], Person { name: "Dana Lee".into(), service: "AMAZON".into() });
    }

    #[test]
    fn empty_sheet_gives_no_people() {
        assert!(parse_people(&[]).is_empty());
    }
}
