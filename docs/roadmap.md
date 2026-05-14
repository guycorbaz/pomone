---
title: Feuille de route
nav_order: 2
description: "Plan d'avancement de Pomone par phases, du bootstrap au polish."
---

# Feuille de route
{: .no_toc }

État d'avancement réel du projet, mis à jour à chaque livraison.
{: .fs-5 .fw-300 }

## Phases

- [x] **Phase 0** — bootstrap (workspace, CI, tooling)
- [x] **Phase 1** — `pomone-domain` : modèle métier complet, tests proptest
- [x] **Phase 2** — `pomone-db` : trait `Repository`, schéma + migrations SQLite, impl SQLite
- [x] **Phase 3** — backend MariaDB + tests d'intégration paramétrés sur les deux backends
- [x] **Phase 4** — `pomone-app` : services, use cases, gestion d'état
- [x] **Phase 5** — i18n (fr + en) avec Fluent
- [ ] **Phase 6** — `pomone-ui` Slint : écrans principaux
  - [x] sidebar de navigation persistante
  - [x] écran Plantings (liste + formulaire annuel/pluriannuel)
  - [x] écran Cultures + Variétés (master-detail)
  - [x] écran Locations (hiérarchie ferme → parcelle → planche)
  - [x] écran Calendrier
- [ ] **Phase 7** — fonctionnalités pérennes spécifiques (vergers, agroforesterie)
- [ ] **Phase 8** — packaging Linux/macOS/Windows
- [ ] **Phase 9** — polish, perf, fix des bugs hérités de Qrop

## Statut actuel

Phase 6 terminée : l'application est utilisable pour saisir et naviguer dans les plantations, cultures et lieux, et le calendrier mensuel affiche les événements dérivés des plantings. Viennent ensuite les fonctionnalités pérennes (suivi `YearlyHarvest`, gestion des strates agroforestières).
