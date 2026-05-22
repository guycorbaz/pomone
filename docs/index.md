---
title: Accueil
layout: home
nav_order: 1
description: "Pomone — logiciel libre de gestion de cultures : maraîchage, grandes cultures, arboriculture et agroforesterie. Réécriture en Rust de Qrop avec support natif des cultures pluriannuelles."
permalink: /
---

# Pomone
{: .fs-9 }

Logiciel libre de gestion de cultures : maraîchage, grandes cultures,
arboriculture et agroforesterie.
{: .fs-6 .fw-300 }

[Voir le code]({{ '/install' | relative_url }}){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[GitHub](https://github.com/guycorbaz/pomone){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Pourquoi Pomone

Pomone est une **réécriture en Rust** de [Qrop](https://qrop.readthedocs.io/) (C++/Qt), avec un modèle de données refondu pour prendre en charge dès le départ les cultures **annuelles** et **pluriannuelles** : maraîchage diversifié, grandes cultures (céréales, pomme de terre, betterave, maïs…), arboriculture fruitière, petits fruits et agroforesterie.

Le projet reprend l'ambition libriste de Qrop, créé par André Hoarau et [L'Atelier paysan](https://www.latelierpaysan.org/), qui reste la référence pour le projet d'origine.

## Caractéristiques

- **Annuelles & pluriannuelles** unifiées dans le modèle de domaine (`Crop.Lifespan`).
- **Persistance au choix** : SQLite (local mono-poste) ou MariaDB (multi-utilisateurs) derrière le même trait `Repository`.
- **UI native desktop** avec [Slint](https://slint.dev/) — pas de WebView.
- **Multilingue** (français, anglais) via [Project Fluent](https://projectfluent.org/).
- **Plateformes v1** : Linux, macOS, Windows.

## Aller plus loin

- [Feuille de route]({{ '/roadmap' | relative_url }}) — état d'avancement par phase.
- [Architecture]({{ '/architecture' | relative_url }}) — principes, flux, modèle de domaine.
- [Installation & build]({{ '/install' | relative_url }}) — prérequis et procédure.
- [Documentation complète (PDF)](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf) — version LaTeX, mise à jour à chaque push sur `main`.

## Licence

GPL v3 ou ultérieure. Le code et la documentation sont libres.
