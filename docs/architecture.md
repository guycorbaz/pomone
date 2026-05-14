---
title: Architecture
nav_order: 3
description: "Principes, flux applicatif et modèle de domaine de Pomone."
---

# Architecture
{: .no_toc }

<details open markdown="block">
  <summary>Sommaire</summary>
  {: .text-delta }
- TOC
{:toc}
</details>

## Principes

1. **Logique métier pure dans `pomone-domain`** : zéro I/O, 100 % testable.
2. **Abstraction du SGBD via un trait `Repository`** dans `pomone-db` : SQLite et MariaDB partagent la même interface. Les particularités SQL vivent dans les migrations et les implémentations concrètes, jamais dans la logique métier.
3. **Pas de logique métier dans les triggers SQL** : tout calcul de dates, de rendements, de cycles est fait en Rust et testé. Les triggers Qrop ont été abandonnés volontairement pour éviter la duplication SQLite/MariaDB.
4. **UI `pomone-ui` (Slint) ne contient pas de règles métier** : elle appelle uniquement des services exposés par `pomone-app`.

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

## Domaine — vue d'ensemble

Concepts clés :

- **Crop** : type de culture (Tomate, Pommier, Framboisier…) avec `Lifespan` (`Annual` ou `Pluriannual { ProductivePattern, lifespan_years }`).
- **Variety** : variété d'un Crop (Marmande, Reine des Reinettes…).
- **Planting** : plantation concrète, avec `PlantingSchedule` (`Cycle` pour annuelles et bisannuelles ; `Perennial` pour vraies pérennes productives chaque année).
- **YearlyHarvest** : récolte annuelle d'une plantation pérenne, pour suivre l'évolution du rendement (montée en charge, plateau, déclin).
- **Location** : lieu hiérarchique (ferme → parcelle → planche / verger / haie…).
- **Strata** : strate agroforestière (table user-managed avec seed initial).

## Persistance

`pomone-db` expose un trait `Repository` partagé par deux implémentations :

- **SQLite** — cible mono-utilisateur, fichier local, migrations dans `migrations/sqlite/`.
- **MariaDB** — cible multi-utilisateurs / serveur, migrations dans `migrations/mariadb/`.

Les tests d'intégration sont paramétrés sur les deux backends pour garantir la parité comportementale.

## Documentation complète

La version exhaustive (modèle de domaine détaillé, choix de conception, formats de données) est rédigée en LaTeX et compilée à chaque push sur `main` :

[Télécharger le PDF](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf){: .btn .btn-primary }

Les sources `.tex` vivent dans [`doc-latex/`](https://github.com/guycorbaz/pomone/tree/main/doc-latex) du dépôt.
