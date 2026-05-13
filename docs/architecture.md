# Architecture Pomone

## Principes

1. **Logique métier pure dans `pomone-domain`** : zéro I/O, 100 % testable.
2. **Abstraction du SGBD via un trait `Repository`** dans `pomone-db` : SQLite et
   MariaDB partagent la même interface. Les particularités SQL vivent dans les
   migrations et les implémentations concrètes, jamais dans la logique métier.
3. **Pas de logique métier dans les triggers SQL** : tout calcul de dates, de
   rendements, de cycles est fait en Rust et testé. Les triggers Qrop ont été
   abandonnés volontairement pour éviter la duplication SQLite/MariaDB.
4. **UI `pomone-ui` (Slint) ne contient pas de règles métier** : elle appelle
   uniquement des services exposés par `pomone-app`.

## Flux

```
   ┌──────────────┐
   │  pomone-ui   │  (Slint)
   └──────┬───────┘
          │ appelle des services (use cases)
          ▼
   ┌──────────────┐
   │  pomone-app  │  (services, état applicatif, i18n)
   └──────┬───────┘
          │ Repository trait
          ▼
   ┌──────────────┐
   │  pomone-db   │  (sqlx — SQLite / MariaDB)
   └──────┬───────┘
          │ types
          ▼
   ┌──────────────┐
   │ pomone-domain│  (types purs, règles métier)
   └──────────────┘
```

## Feuille de route

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
  - [ ] écran Calendrier
- [ ] **Phase 7** — fonctionnalités pérennes spécifiques (vergers, agroforesterie)
- [ ] **Phase 8** — packaging Linux/macOS/Windows
- [ ] **Phase 9** — polish, perf, fix des bugs hérités de Qrop

## Domaine — vue d'ensemble

Voir [domain-model.md](domain-model.md) pour les détails (à écrire en Phase 1).

Concepts clés :

- **Crop** : type de culture (Tomate, Pommier, Framboisier…) avec `Lifespan` (`Annual`
  ou `Pluriannual { ProductivePattern, lifespan_years }`).
- **Variety** : variété d'un Crop (Marmande, Reine des Reinettes…).
- **Planting** : plantation concrète, avec `PlantingSchedule` (`Cycle` pour annuelles
  et bisannuelles ; `Perennial` pour vraies pérennes productives chaque année).
- **YearlyHarvest** : récolte annuelle d'une plantation pérenne, pour suivre
  l'évolution du rendement (montée en charge, plateau, déclin).
- **Location** : lieu hiérarchique (ferme → parcelle → planche / verger / haie…).
- **Strata** : strate agroforestière (table user-managed avec seed initial).
