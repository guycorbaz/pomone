# Pomone

Logiciel libre de gestion de cultures, du maraîchage à l'agroforesterie.

Pomone est une réécriture en Rust de [Qrop](https://qrop.readthedocs.io/) (C++/Qt),
avec un modèle de données refondu pour prendre en charge dès le départ les
cultures **annuelles** et **pluriannuelles** : maraîchage, petits fruits, vergers,
agroforesterie.

- **Site du projet** : <https://guycorbaz.github.io/pomone>
- **Documentation technique (PDF)** : [pomone.pdf](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf) — rolling release, recompilée à chaque push sur `main`.

## État du projet

**Phase 6 — UI en cours.** Le socle est en place : modèle métier (`pomone-domain`),
persistence SQLite et MariaDB derrière un trait `Repository` (`pomone-db`), services
applicatifs (`pomone-app`), i18n fr/en. L'UI Slint expose déjà les écrans Plantings,
Cultures + Variétés et Locations ; l'écran Calendrier reste à faire.

Voir la [feuille de route détaillée](https://guycorbaz.github.io/pomone/roadmap) sur le site.

## Tech

- **Langage** : Rust (édition 2021, MSRV 1.80)
- **UI** : [Slint](https://slint.dev/) (natif desktop, pas de WebView)
- **Données** : [sqlx](https://github.com/launchbadge/sqlx) — SQLite et MariaDB au choix
- **i18n** : [Project Fluent](https://projectfluent.org/) (français, anglais)
- **Plateformes v1** : Linux, macOS, Windows

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
│   ├── sqlite/
│   └── mariadb/
├── doc-latex/           # sources LaTeX → PDF (release docs-latest)
└── docs/                # site GitHub Pages (Jekyll + Just the Docs)
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

## Licence

GPL v3 ou ultérieure. Voir [LICENSE](LICENSE).

Pomone reprend l'ambition libriste de Qrop, créé par André Hoarau et
[L'Atelier paysan](https://www.latelierpaysan.org/), qui reste la référence
pour le projet d'origine.
