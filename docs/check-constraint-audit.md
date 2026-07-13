# Audit des contraintes CHECK — migrations 0001–0006

**Contexte (story 0.6).** SQLite ne sait pas ajouter une valeur autorisée à une
contrainte `CHECK (x IN (…))` sans **reconstruire la table** (opération risquée sur
une base de production). La convergence QRop (E1→E8) va introduire de nouveaux états,
catégories et modes ; cet audit recense **toute** contrainte `CHECK` des migrations
0001–0006, la classe par risque pour la convergence, et nomme sa mitigation.

Portée : `migrations/sqlite/000{1..6}_*.sql`, confrontées à leurs jumelles
`migrations/mariadb/`. Les deux backends doivent rester **comportementalement
identiques** (`cross_backend_tests.rs`).

## Résumé

- **12 contraintes CHECK** au total, toutes dans `0001_initial.sql` (11) et
  `0003_planting_status.sql` (1). Les migrations **0002, 0004, 0005, 0006 n'ajoutent
  aucune contrainte CHECK** (colonnes additives simples).
- **1 seule** contrainte est un piège actif pour la convergence : `task_type.category`.
  **2** sont à surveiller (`planting.status`, `task_series.recurrence_unit`).
  Les **9 autres** sont soit des enums fondamentaux stables, soit des invariants
  structurels de sum-type, soit des bornes de valeur — toutes inoffensives.

## Classification

### A. Enums « ensemble de valeurs » — piège potentiel

Ces contraintes figent une liste de littéraux. En ajouter un exige un rebuild SQLite.

| # | Table.colonne | Valeurs autorisées | Migration | Risque convergence | Mitigation |
|---|---|---|---|---|---|
| 1 | `task_type.category` | sow, transplant, harvest, weeding, irrigation, treatment, tillage, **other** | 0001 | **Élevé** — E1/E2/E3 peuvent vouloir de nouvelles catégories d'opérations. | **Pas de nouveau littéral.** `other` est le fourre-tout ; la granularité fine se fait via `task_type.name` (libre) et le libellé Fluent `category-*`. Mandat de planification (AR2) : étendre par **seed defaults additifs**, jamais par extension du CHECK. Rebuild réservé à un cas de force majeure, tracé par issue. |
| 2 | `planting.status` | active, completed, failed, abandoned | 0003 | **Moyen** — E1 (« états ») pourrait suggérer un nouvel état. | Les nouveaux états de terrain passent par la **table `field_event` (E1, additive, sans CHECK)** et les colonnes de skip, **pas** par extension de cet enum. Le cycle de vie de la plantation reste ces 4 valeurs. |
| 3 | `task_series.recurrence_unit` | days, weeks, months | 0001 | **Faible** — un pas `years` pourrait tenter les séries pérennes (E2). | Si nécessaire, ajout d'un littéral = migration additive de type rebuild guardé, tracée par issue ; à ce stade non requis (les récurrences réelles sont infra-annuelles). |

### B. Enums fondamentaux — stables par conception

Dichotomies/petits ensembles adossés à des sum-types du domaine, non destinés à croître.

| # | Table.colonne | Valeurs | Migration | Note |
|---|---|---|---|---|
| 4 | `crop.pruning_season` | winter, summer, both, none | 0001 | Enum `PruningSeason` (4 saisons de taille). Fermé. |
| 5 | `crop.lifespan_kind` | annual, pluriannual | 0001 | Dichotomie fondatrice `Lifespan`. Fermée. |
| 6 | `crop.productive_pattern` | single_cycle, recurring | 0001 | Sous-type `Pluriannual`. Fermé. (Contrainte incluse dans l'invariant #9.) |
| 7 | `variety.profile_kind` | annual, pluriannual | 0001 | Doit matcher le `lifespan` de la culture (`check_compatible`). Fermé. |
| 8 | `planting.schedule_kind` | cycle, perennial | 0001 | Sum-type `PlantingSchedule`. Fermé. |

Un ajout de variante ici serait un changement de modèle majeur (codec + domaine +
UI), pas une simple extension d'enum : le rebuild ferait alors partie d'un chantier
déjà lourd. Risque convergence : **nul** au sens du piège CHECK.

### C. Invariants structurels de sum-type — inoffensifs

`CHECK` multi-colonnes qui reproduisent en SQL les invariants des constructeurs du
domaine (nullabilité croisée selon le `*_kind`). Ils ne listent aucune valeur
extensible ; ils ne gênent que si l'on **ajoute une colonne** à l'un des blocs — ce
que le domaine encadre déjà.

| # | Table | Rôle | Migration |
|---|---|---|---|
| 9 | `crop` | annuel ⇒ champs pluriannuels NULL, et réciproquement (dont `productive_pattern`/`years_to_first_yield`). | 0001 |
| 10 | `variety` | profil annuel ⇒ champs DOY NULL ; profil pluriannuel ⇒ DTT/DTM/fenêtre NULL. | 0001 |
| 11 | `planting` | cycle ⇒ dates de récolte présentes, dates pérennes NULL ; pérenne ⇒ l'inverse. | 0001 |

Mitigation : toute colonne additive d'un épic (ex. E2/E3) qui touche ces blocs doit
mettre à jour l'invariant **dans la même migration**, sur les deux backends, avec
couverture `cross_backend_tests`. Ce sont des `ALTER`/rebuild volontaires, pas des
pièges d'enum.

### D. Bornes de valeur — inoffensives

| # | Contrainte | Migration | Note |
|---|---|---|---|
| 12 | `task_type.color` : `length(color) IN (4, 7)` | 0001 | Longueur d'un code hex (`#rgb`/`#rrggbb`). Invariant de format, jamais étendu. |
| — | `task_series.recurrence_interval >= 1` | 0001 | Borne numérique. Inoffensive. |

## Parité SQLite ↔ MariaDB

Confirmée pour chaque contrainte : `migrations/mariadb/0001_initial.sql` et
`0003_planting_status.sql` portent les **mêmes ensembles de littéraux** (nommée
`chk_task_type_category` côté MariaDB, `CHAR_LENGTH` au lieu de `length`, sinon
identiques). Toute évolution d'un CHECK devra rester synchrone sur les deux arbres.

## Conclusion pour la convergence

La convergence peut avancer **sans toucher aux contraintes CHECK existantes** :

1. Nouveaux faits/états de terrain (E1) → table `field_event` additive, **sans CHECK**.
2. Nouvelles opérations → `task_type` avec `category = 'other'` + `name`/Fluent libres.
3. `planting.status` reste à 4 valeurs ; les nuances passent par le journal d'événements.

Le seul piège actif, `task_type.category`, est neutralisé par la règle « seed
defaults additifs, jamais d'extension du CHECK » déjà inscrite dans `CLAUDE.md` et
dans le mandat AR2 des épics.
