# RATPBadgeGenerator

Application de bureau (Rust, interface graphique) qui génère un PDF A4 de badges nominatifs
au format **88,9 × 50,8 mm**, 10 badges par page, prêts à imprimer et à découper.

Chaque badge affiche :
- en haut à gauche : le **logo RATP Groupe** (fixe, toujours le même) ;
- en haut à droite : le **logo « Club des CSP »** par défaut, remplaçable depuis l'interface ;
- au centre : le **nom** de la personne, en police unique (taille identique sur tous les badges) ;
- en bas : une barre bleue avec l'**intitulé du service** (la colonne du fichier source) ;
- un contour en pointillés pour la découpe.

## Utilisation

1. Ouvrir l'application.
2. **Fichier Excel…** : choisir le fichier source (`.xlsx`, `.xlsm`, `.xls`, `.ods`). Si le classeur contient plusieurs feuilles, choisir la feuille dans la liste.
3. Vérifier la liste des personnes et des services affichée.
4. (Optionnel) **Logo à droite…** : choisir un autre logo (PNG ou JPEG). Le logo RATP à gauche ne change pas.
5. **Générer le PDF…** : choisir où enregistrer le fichier.

Imprimer en A4 à **100 %** (désactiver « Ajuster à la page »).

## Format attendu du fichier Excel

- **Ligne 1** : l'intitulé du service de chaque colonne (ex. `DSP transverse`, `CSP RH`).
- **Lignes suivantes** : une personne par cellule, sous son service. Les colonnes vides sont ignorées.
- **Ligne « INTERVENANTS »** (dans la première colonne) : sépare le bloc des intervenants.
  La ligne non vide qui suit est l'entête de ce bloc (ex. `DELOITTE`, `SANOFI`), puis les intervenants sous chacun.

Les corrections de noms se font dans le fichier Excel source (l'application ne les modifie pas).

## Compilation

Prérequis : [Rust](https://rustup.rs/) (version stable récente).

```bash
cargo run --release          # lance l'interface
cargo test                   # tests de lecture du fichier Excel
cargo build --release        # binaire dans target/release/
```

Sous Linux, l'interface et la boîte de dialogue de fichiers peuvent demander les bibliothèques GTK3 (`libgtk-3-dev`).

## Structure

```
src/
  main.rs      interface graphique (egui / eframe)
  lib.rs       point d'entrée de la bibliothèque
  excel.rs     lecture du classeur, extraction des personnes et services
  badges.rs    mise en page et génération du PDF (printpdf)
assets/
  ratp_logo.png          logo RATP Groupe (fixe)
  club_csp_logo.png      logo Club des CSP par défaut
  LiberationSans-Bold.ttf  police (SIL Open Font License 1.1)
```

Les dimensions, couleurs et tailles de police sont des constantes en tête de `src/badges.rs`.

## Notes

- Les logos RATP sont des marques : garder ce dépôt privé.
- Les couleurs (bleu `#004FA3`, turquoise `#4BC0AD`) sont celles du logo RATP ; elles sont à valider sur la charte officielle.
