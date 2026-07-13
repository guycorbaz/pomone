# Glossaire fondateur — Pomone

Ce glossaire fixe le vocabulaire fondateur de Pomone : **un terme, une traduction
FR, une traduction EN**, pour que l'interface, la documentation et le code parlent
la même langue par locale. C'est la référence de la convergence QRop (épics E1→E8) :
chaque nouveau concept y entre avant d'apparaître à l'écran.

## Comment lire ce tableau

- **term_id** — identifiant stable, `kebab-case`, cité dans le code et les commits.
- **FR / EN** — le mot retenu dans chaque locale. Les deux catalogues Fluent
  (`crates/pomone-app/locales/{fr,en}/main.ftl`) doivent s'y conformer.
- **Définition** — une phrase, du point de vue du maraîcher.
- **Préfixe Fluent** — le préfixe de clé qui ancre le terme dans les `.ftl`
  (ex. `category` couvre `category-sow`, `category-harvest`…). `—` si le terme
  n'est pas encore câblé.
- **Portée CI** — `checked` : le terme est vérifié **dès maintenant** par le test
  de cohérence `glossary_coherence` (clé présente + parité fr↔en).
  `deferred` : terme fondateur déjà nommé mais dont l'alignement Fluent est
  planifié (renommage en story 0.8, ou introduction par un épic ultérieur) —
  hors périmètre du test tant qu'il reste `deferred`, ce qui garde le test **vert
  dès sa naissance**.

## Termes fondateurs

| term_id | FR | EN | Définition | Préfixe Fluent | Portée CI |
|---|---|---|---|---|---|
| crop | Culture | Crop | Type de plante cultivée (au niveau de l'espèce), parent de ses variétés. | crop | checked |
| variety | Variété | Variety | Cultivar nommé d'une culture, porteur du profil de croissance (annuel ou pluriannuel). | variety | checked |
| family | Famille | Family | Famille botanique regroupant des cultures (Solanacées, Poacées…), support de la rotation. | family | checked |
| location | Lieu | Location | Endroit physique qui accueille des plantations : jardin, planche, serre, pièce. | location | checked |
| strata | Strate | Stratum | Couche verticale occupée par une culture (herbacée, arbustive, canopée…) pour l'empilement agroforestier. | strata | checked |
| planting | Plantation | Planting | Une mise en place d'une variété, à un lieu, sur un calendrier (cycle annuel ou pérenne). | planting | checked |
| yearly-harvest | Récolte annuelle | Yearly harvest | Relevé de rendement, année par année, d'une plantation pérenne. | yearly-harvest | checked |
| task | Tâche | Task | Opération datée (semis, repiquage, désherbage…) visant une plantation et/ou un lieu. | task | checked |
| treatment | Traitement | Treatment | Application phytosanitaire consignée sur une plantation à des fins de traçabilité. | treatment | checked |
| planting-status | Statut de plantation | Planting status | État du cycle de vie d'une plantation : en cours, terminée, échouée, abandonnée. | planting-status | checked |
| task-category | Catégorie de tâche | Task category | Enum stable qui classe un type de tâche (semis, repiquage, récolte, désherbage…). | category | checked |
| succession | Série | Succession | Une culture replantée en lots échelonnés dans la saison (la «succession» de QRop). Aujourd'hui rendue EN «series» ; alignée en story 0.8. | — | deferred |
| bed | Planche | Bed | Bande de culture élémentaire à l'intérieur d'un jardin ou d'une pièce. Aujourd'hui diffuse dans les libellés de lieux ; alignée en story 0.8. | — | deferred |
| growing-schedule | Itinéraire technique (ITK) | Growing schedule | Gabarit ordonné des opérations d'une culture (semis→…→récolte). Introduit à l'épic 2. | — | deferred |
| skipped | Ignorée | Skipped | Tâche délibérément non faite, consignée avec un motif — jamais perdue en silence. Introduit à l'épic 1. | — | deferred |
| field-event | Fait de terrain | Field event | Enregistrement en ajout-seul de ce qui s'est passé au champ (fait, ignoré, corrigé). Introduit à l'épic 1. | — | deferred |
| correction | Correction | Correction | Amendement explicite d'un fait antérieur, l'historique étant préservé. Introduit à l'épic 1. | — | deferred |

## Règle de tenue

- **Ajouter un terme** = une ligne ici + (si `checked`) des clés Fluent fr **et** en
  sous le préfixe déclaré. Le test `glossary_coherence` échoue sinon.
- **Faire passer un terme de `deferred` à `checked`** = son travail d'alignement
  Fluent (story 0.8 pour les renommages, ou l'épic qui l'introduit) est fait,
  fr↔en sont à parité sous le préfixe : on bascule alors la portée CI.
- Les deux `.ftl` restent des miroirs (même jeu de clés, ordre alphabétique par
  section). Voir `crates/pomone-app/tests/glossary_coherence.rs`.
