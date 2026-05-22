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
- **Variety** : variété d'un Crop (Marmande, Reine des Reinettes…) avec son profil agronomique (DTT, DTM, fenêtre de récolte pour les annuelles ; DOY de débourrement, floraison et récolte pour les pluriannuelles).
- **Planting** : plantation concrète, avec `PlantingSchedule` (`Cycle` pour annuelles et bisannuelles ; `Perennial` pour vraies pérennes productives chaque année). Trois variantes de cycle annuel sont supportées : semis direct, semis serre puis repiquage, et plantation directe (plants achetés).
- **YearlyHarvest** : récolte annuelle d'une plantation pérenne, pour suivre l'évolution du rendement (montée en charge, plateau, déclin).
- **Location** : lieu hiérarchique (ferme → parcelle → planche / verger / haie…) avec dimensions `length_m × width_m`.
- **Strata** : strate agroforestière (table user-managed avec seed initial).
- **Task / TaskType / TaskMethod / TaskImplement** : opérations sur les plantations ou les lieux (semis, repiquage, désherbage, récolte, traitement, travail du sol…). Chaque `TaskType` porte un `TaskCategory` enum stable pour que l'auto-générateur reconnaisse les types indépendamment du nom choisi par l'utilisateur.

## UI — composants notables

- **`Timeline` + `Timegraph`** (`ui/components/timeline.slint`) : composants réutilisables qui rendent la frise Gantt sur 12 mois. Chaque planting devient une barre multi-segments serre/champ/récolte avec ligne du jour. Utilisée sur l'Accueil (vue compacte) et l'écran Plantations (vue principale).
- **Per-file rule** : un écran Slint = un fichier sous `ui/`, un composant réutilisable = un fichier sous `ui/components/`. Évite les fichiers-monolithes hérités de l'organisation Qrop QML.

## Persistance

`pomone-db` expose un trait `Repository` partagé par deux implémentations :

- **SQLite** — cible mono-utilisateur, fichier local, migrations dans `migrations/sqlite/`.
- **MariaDB** — cible multi-utilisateurs / serveur, migrations dans `migrations/mariadb/`.

Les tests d'intégration sont paramétrés sur les deux backends pour garantir la parité comportementale.

Tant que Pomone n'a pas atteint v1.0.0, les changements de schéma vivent **dans un seul fichier de migration** par backend (`0001_initial.sql`) — pas d'empilement `0002_*.sql`, `0003_*.sql`. Pomone n'a pas d'utilisateurs en production à ce stade, donc pas de bases existantes à migrer incrémentalement. À partir de v1.0.0, le mode incrémental classique reprendra : `0001_initial.sql` figé comme schéma de départ, nouvelles modifications dans des fichiers numérotés successifs.

Une bascule SQLite ↔ MariaDB avec copie des données est disponible depuis l'écran Paramètres (`pomone-app::migration`).

## Documentation complète

La version exhaustive (modèle de domaine détaillé, choix de conception, formats de données) est rédigée en LaTeX et compilée à chaque push sur `main` :

[Télécharger le PDF](https://github.com/guycorbaz/pomone/releases/download/docs-latest/pomone.pdf){: .btn .btn-primary }

Les sources `.tex` vivent dans [`doc-latex/`](https://github.com/guycorbaz/pomone/tree/main/doc-latex) du dépôt.
