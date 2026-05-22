---
title: Installation & build
nav_order: 4
description: "Prérequis, compilation et configuration de Pomone."
---

# Installation & build
{: .no_toc }

<details open markdown="block">
  <summary>Sommaire</summary>
  {: .text-delta }
- TOC
{:toc}
</details>

## Prérequis

- **Rust** édition 2021, MSRV **1.80** (installation via [rustup](https://rustup.rs/)).
- Outils Slint : dépendances système pour le rendu natif (cf. [Slint platform requirements](https://slint.dev/)).
- Optionnel : **MariaDB ≥ 10.6** si vous voulez utiliser ce backend (SQLite ne nécessite rien).

## Cloner

```sh
git clone https://github.com/guycorbaz/pomone.git
cd pomone
```

## Compiler

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

Le binaire desktop est `target/release/pomone` (crate `pomone-ui`). Un binaire admin/debug `pomone-cli` est aussi produit (crate `pomone-cli`).

## Lancer

```sh
cargo run --bin pomone
```

## Choix du backend

Pomone choisit son backend selon la chaîne de connexion (`DATABASE_URL` ou configuration applicative) :

- `sqlite:./pomone.db` — fichier local.
- `mariadb://user:pass@host:3306/pomone` — serveur MariaDB.

Les migrations sont versionnées dans `migrations/sqlite/` et `migrations/mariadb/` et appliquées au démarrage.

## Emplacement par défaut de la base SQLite

Lancé sans configuration explicite, Pomone crée son fichier `pomone.sqlite` à l'emplacement standard de la plateforme :

| Système | Chemin |
|---------|--------|
| Linux   | `~/.local/share/pomone/pomone.sqlite` |
| Windows | `%APPDATA%\pomone\pomone.sqlite` |
| macOS   | `~/Library/Application Support/pomone/pomone.sqlite` |

Le chemin se modifie depuis l'écran **Paramètres** de l'application (champ *Chemin SQLite*). Pour déplacer une base existante vers un autre chemin ou vers MariaDB, utilisez le bouton **Enregistrer et migrer**.

C'est aussi le fichier à sauvegarder régulièrement : il contient toutes vos données.

## Construire la documentation PDF

Les sources LaTeX vivent dans `doc-latex/`. Pour compiler localement :

```sh
cd doc-latex
latexmk -pdf pomone.tex
```

À chaque push sur `main`, GitHub Actions compile la version officielle et la publie en [release rolling `docs-latest`](https://github.com/guycorbaz/pomone/releases/tag/docs-latest).
