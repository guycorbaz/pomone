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
- **Accueil** : courbe d'occupation des planches (plein champ vs sous abri) sur la saison, au-dessus du Gantt.
- **Vue Gantt** de la saison sur l'écran Plantations : barres multi-segments serre → champ → récolte, ligne du jour, clic pour ouvrir le détail.
- **Calendrier unifié** : grille mensuelle mêlant tâches (pleines) et jalons de culture (en contour), glisser-déposer pour replanifier, filtres par catégorie.
- **Tâches & opérations** : auto-génération depuis les plantations (semis, repiquage, récolte), séries récurrentes, et liste à plat « Tâches » avec badges En retard / Aujourd'hui.
- **Persistance au choix** : SQLite (local mono-poste, recommandé) ou MariaDB (serveur partagé) derrière le même trait `Repository`. Bascule à chaud avec migration des données depuis l'écran Paramètres.
- **UI native desktop** avec [Slint](https://slint.dev/) — pas de WebView.
- **Multilingue** (français, anglais) via [Project Fluent](https://projectfluent.org/).
- **Manuel utilisateur** PDF embarqué dans l'application (`F1` pour l'ouvrir).
- **Plateformes** : Linux (`.deb` + AppImage livrés), Windows et macOS prévus post-v1.0.0.

## Aller plus loin

- [Feuille de route]({{ '/roadmap' | relative_url }}) — état d'avancement par phase, sous-livraisons de la parité Qrop.
- [Architecture]({{ '/architecture' | relative_url }}) — principes, flux, modèle de domaine.
- [Installation & build]({{ '/install' | relative_url }}) — prérequis, procédure, chemin par défaut de la base SQLite.
- [Manuel utilisateur (PDF)](https://github.com/guycorbaz/pomone/releases/download/docs-latest/manuel.pdf) — destiné aux utilisateurs finaux ; aussi accessible via `F1` dans l'application.
- [Documentation technique (PDF)](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf) — pour les développeurs (architecture, modèle de domaine, persistance).
- Les deux PDF sont recompilés à chaque push sur `main`.

## Licence

GPL v3 ou ultérieure. Le code et la documentation sont libres.
