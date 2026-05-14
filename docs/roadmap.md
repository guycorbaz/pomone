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
- [x] **Phase 7** — fonctionnalités pérennes spécifiques (vergers, agroforesterie)
  - [x] suivi `YearlyHarvest` (saisie année par année, variance attendu/réel)
  - [x] gestion CRUD des strates agroforestières
- [ ] **Phase 8** — packaging Linux/macOS/Windows
- [ ] **Phase 9** — polish, perf, fix des bugs hérités de Qrop

## Statut actuel

Phases 6 et 7 terminées : l'application gère plantations, cultures, lieux, calendrier, détail de plantation, récoltes annuelles et strates agroforestières. La prochaine étape est le packaging desktop (Phase 8) ou le polish général (Phase 9).
