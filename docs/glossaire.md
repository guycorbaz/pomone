# Glossaire fondateur — Pomone

Ce glossaire fixe le vocabulaire de Pomone : **un terme, une traduction FR, une
traduction EN**, pour que l'interface, la documentation et le code parlent la
même langue par locale. C'est la référence de la convergence QRop (épics E1→E8).

Deux tables :

1. **Termes fondateurs (vérifiés en CI)** — les concepts déjà présents dans
   l'application. Le test `crates/pomone-app/tests/glossary_coherence.rs` vérifie
   **chaque ligne** : le préfixe Fluent résout au moins une clé, et les
   catalogues fr↔en sont à parité sous ce préfixe. Le test tourne **non-scopé**
   (story 0.8) : plus de colonne « portée », toute ligne de cette table est
   couverte.
2. **Vocabulaire documenté et planifié (hors gate)** — concepts déjà nommés mais
   soit sans ancre Fluent stable (`bed`), soit introduits par un épic à venir
   (E1/E2). Cette table **n'est pas** parsée par le test. Chaque épic qui câble
   son concept en promeut le terme dans la table 1 (avec ses clés fr+en).

## Comment lire la table 1

- **term_id** — identifiant stable, `kebab-case`, cité dans le code et les commits.
- **FR / EN** — le mot retenu dans chaque locale ; les deux `.ftl`
  (`crates/pomone-app/locales/{fr,en}/main.ftl`) doivent s'y conformer.
- **Définition** — une phrase, du point de vue du maraîcher.
- **Préfixe Fluent** — le préfixe de clé qui ancre le terme (ex. `category`
  couvre `category-sow`, `category-harvest`…).

## Table 1 — Termes fondateurs (vérifiés en CI)

| term_id | FR | EN | Définition | Préfixe Fluent |
|---|---|---|---|---|
| crop | Culture | Crop | Type de plante cultivée (au niveau de l'espèce), parent de ses variétés. | crop |
| variety | Variété | Variety | Cultivar nommé d'une culture, porteur du profil de croissance (annuel ou pluriannuel). | variety |
| family | Famille | Family | Famille botanique regroupant des cultures (Solanacées, Poacées…), support de la rotation. | family |
| location | Lieu | Location | Endroit physique qui accueille des plantations : jardin, planche, serre, pièce. | location |
| strata | Strate | Stratum | Couche verticale occupée par une culture (herbacée, arbustive, canopée…) pour l'empilement agroforestier. | strata |
| planting | Plantation | Planting | Une mise en place d'une variété, à un lieu, sur un calendrier (cycle annuel ou pérenne). | planting |
| yearly-harvest | Récolte annuelle | Yearly harvest | Relevé de rendement, année par année, d'une plantation pérenne. | yearly-harvest |
| task | Tâche | Task | Opération datée (semis, repiquage, désherbage…) visant une plantation et/ou un lieu. | task |
| treatment | Traitement | Treatment | Application phytosanitaire consignée sur une plantation à des fins de traçabilité. | treatment |
| planting-status | Statut de plantation | Planting status | État du cycle de vie d'une plantation : en cours, terminée, échouée, abandonnée. | planting-status |
| task-category | Catégorie de tâche | Task category | Enum stable qui classe un type de tâche (semis, repiquage, récolte, désherbage…). | category |
| task-series | Série (récurrente) | Series (recurring) | Un modèle de tâche qui se répète à intervalle régulier (arrosage, tonte) ; chaque occurrence est une tâche datée. | task-form-series |
| printdoc | Feuille de semaine (PrintDoc) | Week sheet (PrintDoc) | Contrat de données figé et versionné (v1) projetant les tâches d'une semaine (par jour puis planche, tour-de-plaine, états ☐/☒/⊘) pour l'impression — rendu texte en story 1.4, PDF en épic 4. | print |

## Le contrat PrintDoc (`WeekSheet`, v1)

Le **PrintDoc** est un contrat de données **figé et versionné** (`PRINTDOC_VERSION`,
actuellement **1**) — voir `crates/pomone-app/src/printdoc.rs`. Il est
**neutre en langue** : il porte des enums (`EntryState` = Pending/Done/Skipped,
`SkipReason`) et des dates (`NaiveDate`), jamais de chaînes localisées ; chaque
moteur de rendu localise le chrome (jours, mot « ignorée », motifs) via Fluent
(préfixe `print`). Forme v1 :

- `WeekSheet { version, week_start (lundi), week_end (dimanche), days: [DaySheet] }`
- `DaySheet { date, entries: [Entry] }` — entrées triées planche puis opération.
- `Entry { task_id, state, bed?, crop?, task, skip_reason? }`.

Story 1.4 le rend en **texte simple** (`render_text`) ; l'épic 4 rendra le même
contrat en **PDF**. Le harness paper-loop l'utilise comme oracle `faits → PrintDoc`
et en vérifie la forme (`version`). **Toute évolution cassante = bump de
`PRINTDOC_VERSION`.**

## Table 2 — Vocabulaire documenté et planifié (hors gate)

| term_id | FR | EN | Définition | Statut |
|---|---|---|---|---|
| bed | Planche | Bed | Bande de culture élémentaire à l'intérieur d'un jardin ou d'une pièce. | Documenté — FR↔EN déjà alignés dans les chaînes ; pas de clé-libellé dédiée à ancrer (les types de lieux sont des données utilisateur), donc hors gate. |
| succession | Succession | Succession | Une culture replantée en lots échelonnés dans la saison (la «succession» de QRop). | Planifié E2 (plantations échelonnées). **Distinct de `task-series`** : une succession concerne des *plantations*, une série des *tâches*. |
| growing-schedule | Itinéraire technique (ITK) | Growing schedule | Gabarit ordonné des opérations d'une culture (semis→…→récolte). | Planifié E2. |
| skipped | Ignorée | Skipped | Tâche délibérément non faite, consignée avec un motif — jamais perdue en silence. | Planifié E1. |
| field-event | Fait de terrain | Field event | Enregistrement en ajout-seul de ce qui s'est passé au champ (fait, ignoré, corrigé). | Planifié E1. |
| correction | Correction | Correction | Amendement explicite d'un fait antérieur, l'historique étant préservé. | Planifié E1. |

## Décisions de terminologie (story 0.8)

L'audit des chaînes existantes a montré que **rien de visible n'était à renommer** :

- **`série`/`series` restent tels quels.** Toutes leurs occurrences (fr et en)
  désignent les **séries de tâches récurrentes** (arrosage, tonte) — un concept
  correct et **distinct** de la « succession » maraîchère. L'intitulé initial de
  l'épic (« EN succession pour série ») confondait les deux ; `succession` est
  réservé aux *plantations* échelonnées et entre avec E2.
- **`planche`↔`bed` sont déjà alignés** dans les chaînes (EN « bed » partout, FR
  « planche »). Aucune clé-libellé dédiée n'existe (les lieux sont des données
  utilisateur), donc `bed` reste documenté mais hors gate.
- Les autres termes planifiés (`skipped`, `field-event`, `correction`,
  `growing-schedule`) n'ont **aucune chaîne aujourd'hui** : ils sont câblés (et
  promus en table 1) par l'épic qui les introduit.

## Règle de tenue

- **Ajouter un terme fondateur** (table 1) = une ligne + des clés Fluent fr **et**
  en sous le préfixe déclaré. Le test `glossary_coherence` échoue sinon.
- **Promouvoir un terme planifié** (table 2 → table 1) = l'épic qui le câble crée
  ses clés fr+en, ajoute la ligne en table 1 avec son préfixe, et retire l'entrée
  de la table 2.
- Les deux `.ftl` restent des miroirs (même jeu de clés, ordre alphabétique par
  section). Voir `crates/pomone-app/tests/glossary_coherence.rs`.
