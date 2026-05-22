# Pomone

Logiciel libre de gestion de cultures : maraîchage, grandes cultures,
arboriculture et agroforesterie.

Pomone est une réécriture en Rust de [Qrop](https://qrop.readthedocs.io/) (C++/Qt),
avec un modèle de données refondu pour prendre en charge dès le départ les
cultures **annuelles** et **pluriannuelles** : maraîchage diversifié, grandes
cultures (céréales, pomme de terre, betterave, maïs…), arboriculture fruitière,
petits fruits et agroforesterie.

- **Site du projet** : <https://guycorbaz.github.io/pomone>
- **Documentation technique (PDF)** : [pomone.pdf](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf) — rolling release, recompilée à chaque push sur `main`.

## État du projet

**Phase 10 — Parité fonctionnelle Qrop pour v1.0.0, en cours.**

Le socle est en place et la plupart des fonctionnalités essentielles sont
livrées : modèle métier (`pomone-domain`), persistance SQLite ou MariaDB
derrière un même trait `Repository` (`pomone-db`), services applicatifs
(`pomone-app`), i18n FR/EN avec Fluent, UI desktop native (Slint).

L'application expose aujourd'hui les écrans **Accueil** (vue Gantt
résumée de la saison), **Plantations** (liste + vue Gantt multi-segments
serre/champ/récolte + formulaire annuel/pluriannuel), **Cultures et
variétés**, **Lieux**, **Strates**, **Calendrier mensuel**, **Détail de
plantation** avec récoltes annuelles pour les pluriannuelles, et
**Paramètres** (basculement SQLite ↔ MariaDB avec migration des données
à la volée).

Le packaging Linux est livré (`.deb` + AppImage avec manuel PDF
embarqué, accessible depuis l'application via la touche `F1`). Le
backend des **tâches/opérations** (table + repository) est en place ;
l'auto-génération depuis les plantations, l'écran Calendrier des tâches
et la superposition Gantt sont en cours (PRs E et F).

Voir la [feuille de route détaillée](https://guycorbaz.github.io/pomone/roadmap) sur le site et le [manuel utilisateur PDF](https://github.com/guycorbaz/pomone/releases/download/docs-latest/manuel.pdf).

## Tech

- **Langage** : Rust (édition 2021, MSRV 1.80)
- **UI** : [Slint](https://slint.dev/) (natif desktop, pas de WebView)
- **Données** : [sqlx](https://github.com/launchbadge/sqlx) — SQLite ou MariaDB derrière le même trait `Repository`
- **i18n** : [Project Fluent](https://projectfluent.org/) (français, anglais)
- **Manuel** : LaTeX (xelatex + TeX Gyre Heros), PDF embarqué dans l'app
- **Plateformes** : Linux (`.deb` + AppImage livrés), Windows et macOS prévus post-v1.0.0

## Structure

```
pomone/
├── crates/
│   ├── pomone-domain/   # types métier purs, sans I/O
│   ├── pomone-db/       # trait Repository + impls SQLite/MariaDB
│   ├── pomone-app/      # services, use cases, état applicatif
│   ├── pomone-ui/       # binaire desktop (Slint) — `pomone`
│   └── pomone-cli/      # binaire admin/debug — `pomone-cli`
├── migrations/
│   ├── sqlite/          # schéma SQLite (un seul 0001_initial.sql pré-v1.0.0)
│   └── mariadb/         # idem pour MariaDB
├── doc-latex/           # doc technique LaTeX → pomone.pdf (release docs-latest)
└── docs/                # site GitHub Pages (Jekyll + Just the Docs)
    └── manual/          # manuel utilisateur LaTeX → manuel.pdf (embarqué dans l'app)
```

## Build

```sh
# Compilation
cargo build --release

# Tests
cargo test --workspace

# Lints
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# Couverture (cible ≥ 80 %)
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --html
```

## Packages

```sh
# Linux : .deb + AppImage (x86_64)
cargo install cargo-packager --locked
cargo packager --release -p pomone-ui --formats deb,appimage
# → target/release/pomone_0.1.0_amd64.deb
# → target/release/pomone_0.1.0_x86_64.AppImage
```

Le `.deb` installe le binaire dans `/usr/bin/pomone` avec icônes hicolor et entrée
`.desktop`. L'AppImage est autonome (lance directement, pas d'installation requise).
Packaging Windows (`.msi`) et macOS (`.dmg`) à venir.

## Licence

GPL v3 ou ultérieure. Voir [LICENSE](LICENSE).

Pomone reprend l'ambition libriste de Qrop, créé par André Hoarau et
[L'Atelier paysan](https://www.latelierpaysan.org/), qui reste la référence
pour le projet d'origine.
