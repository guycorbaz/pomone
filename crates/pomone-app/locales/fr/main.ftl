# Pomone — French translations.
#
# Keep keys in alphabetical order within each section to make merges easier.
# Variable references use { $name }. Selectors and plural forms (e.g. for
# crop counts) can be added as the UI requires them.

## Application
welcome = Bienvenue dans Pomone
welcome-user = Bienvenue, { $name } !
welcome-summary = Pomone gère vos cultures annuelles et pluriannuelles : maraîchage, grandes cultures, arboriculture et agroforesterie.
button-switch-language = English
button-create-planting = Créer la plantation
nav-home = Accueil
nav-plantings = Plantations
nav-cultures = Cultures
nav-locations = Lieux
label-strata-count = Strates
label-families-count = Familles
label-location-kinds-count = Types de lieux
label-variety = Variété
label-location = Lieu
label-sown-on = Date de semis
label-established-on = Date de plantation
label-removal-on = Arrachage prévu (optionnel)
placeholder-removal-date = AAAA-MM-JJ (laisser vide)
label-area = Surface (m²)
label-plants-count = Nombre de plants
placeholder-date = AAAA-MM-JJ
placeholder-area = 20
placeholder-count = 100
plants-suffix = plants
section-overview = Aperçu
section-new-planting = Nouvelle plantation annuelle
title-plantings = Plantations
empty-plantings = Aucune plantation pour l'instant. Renseignez le formulaire ci-dessous pour en créer une.
status-planting-created = Plantation créée
status-planting-failed = Échec de la création : { $message }
status-pick-variety = Sélectionnez une variété et un lieu avant de créer.

## Cultures + Variétés screen
title-cultures = Cultures et variétés
crops-title = Cultures
empty-crops = Aucune culture pour l'instant.
varieties-title = Variétés
empty-varieties = Aucune variété pour cette culture.
no-crop-selected = Sélectionnez une culture pour voir et créer ses variétés.
new-crop-section = Nouvelle culture annuelle
new-variety-section = Nouvelle variété annuelle
label-crop-name = Nom
placeholder-crop-name = Tomate
label-crop-latin = Nom latin (optionnel)
placeholder-crop-latin = Solanum lycopersicum
label-crop-family = Famille
label-crop-strata = Strate
label-lifespan = Cycle de vie
label-lifespan-years = Années de vie
placeholder-lifespan-years = 30
label-years-to-first-yield = Années avant première récolte
placeholder-years-to-first-yield = 3
label-pruning = Taille
lifespan-annual = Annuelle
lifespan-pluriannual-single = Pluriannuelle cycle unique
lifespan-pluriannual-recurring = Pluriannuelle récurrente
pruning-none-label = Sans taille
pruning-winter-label = Taille d'hiver
pruning-summer-label = Taille d'été
pruning-both-label = Taille hiver + été
label-bud-break-doy = Débourrement (jour de l'année)
placeholder-bud-break-doy = 80
label-flowering-doy = Floraison (jour de l'année)
placeholder-flowering-doy = 120
label-harvest-start-doy = Début récolte (jour de l'année)
placeholder-harvest-start-doy = 220
label-harvest-end-doy = Fin récolte (jour de l'année)
placeholder-harvest-end-doy = 280
label-yield-kg = Rendement attendu (kg/plant, optionnel)
placeholder-yield-kg = 15.5
label-variety-name = Nom de la variété
placeholder-variety-name = Marmande
label-variety-description = Description (optionnel)
placeholder-variety-description = ancienne variété, fruits côtelés
label-dtt = Jours sem.→repiq. (DTT)
label-dtm = Jours sem.→récolte (DTM)
label-window = Fenêtre de récolte (jours)
placeholder-dtt = 35
placeholder-dtm = 70
placeholder-window = 60
button-create-crop = Créer la culture
button-create-variety = Créer la variété
status-crop-created = Culture créée
status-variety-created = Variété créée
status-pick-crop-first = Sélectionnez une culture avant de créer une variété.

## Planting detail screen
title-planting-detail = Détail de la plantation
button-back = ‹ Retour
section-schedule = Calendrier de culture
section-summary = Résumé
label-planting-name = Nom
label-planting-notes = Notes
label-transplanted-on = Date de repiquage
label-first-harvest = Première récolte
label-last-harvest = Dernière récolte
empty-planting-detail = Plantation introuvable.

## Yearly harvest (perennials)
section-yearly-harvest = Récoltes annuelles
section-record-harvest = Enregistrer une récolte
empty-yearly-harvest = Aucune récolte enregistrée pour le moment.
harvest-header-year = Année
harvest-header-expected = Attendu
harvest-header-actual = Réel
harvest-header-variance = Écart
harvest-header-notes = Notes
label-harvest-year = Année
label-harvest-expected = Rendement attendu (kg)
label-harvest-actual = Rendement réel (kg)
label-harvest-notes = Notes (optionnel)
placeholder-harvest-year = 2030
placeholder-harvest-kg = 50
placeholder-harvest-notes = gelée tardive, taille sévère…
button-record-harvest = Enregistrer
status-harvest-recorded = Récolte enregistrée

## Calendar screen
nav-calendar = Calendrier
title-calendar = Calendrier
calendar-prev = ‹
calendar-next = ›
calendar-today = Aujourd'hui
calendar-empty = Aucun événement ce mois-ci.
weekday-mon-short = Lu
weekday-tue-short = Ma
weekday-wed-short = Me
weekday-thu-short = Je
weekday-fri-short = Ve
weekday-sat-short = Sa
weekday-sun-short = Di
month-1 = Janvier
month-2 = Février
month-3 = Mars
month-4 = Avril
month-5 = Mai
month-6 = Juin
month-7 = Juillet
month-8 = Août
month-9 = Septembre
month-10 = Octobre
month-11 = Novembre
month-12 = Décembre
event-sowing-glyph = S
event-sowing-label = Semis
event-transplanting-glyph = R
event-transplanting-label = Repiquage
event-harvest-start-glyph = ▶
event-harvest-start-label = Début récolte
event-harvest-end-glyph = ◼
event-harvest-end-label = Fin récolte
event-establishment-glyph = P
event-establishment-label = Plantation
event-removal-glyph = ✕
event-removal-label = Arrachage
event-bud-break-glyph = B
event-bud-break-label = Débourrement
event-flowering-glyph = F
event-flowering-label = Floraison

## Settings screen
nav-settings = Paramètres
title-settings = Paramètres
settings-current-section = Base de données active
settings-current-label = Backend
settings-edit-section = Modifier la base de données
settings-backend-kind-label = Type de base
settings-backend-sqlite = SQLite (fichier local)
settings-backend-mariadb = MariaDB / MySQL (distant)
settings-sqlite-path-label = Chemin du fichier
settings-sqlite-path-placeholder = /chemin/vers/pomone.sqlite
settings-mariadb-host-label = Hôte
settings-mariadb-host-placeholder = mariadb.example.com
settings-mariadb-port-label = Port
settings-mariadb-port-placeholder = 3306
settings-mariadb-user-label = Utilisateur
settings-mariadb-user-placeholder = pomone
settings-mariadb-password-label = Mot de passe
settings-mariadb-password-placeholder = ••••••••
settings-mariadb-database-label = Base
settings-mariadb-database-placeholder = pomone
settings-button-test = Tester la connexion
settings-button-save = Basculer sans copier
settings-button-save-migrate = Migrer les données
settings-migrate-warning = "Migrer les données" copie toutes les données existantes vers la nouvelle base avant de basculer. "Basculer sans copier" ouvre la nouvelle base avec uniquement les valeurs par défaut — utile pour repartir de zéro.
settings-test-ok = Connexion réussie.
settings-save-ok = Base de données basculée vers : { $backend }
settings-migrate-ok = Migration terminée vers : { $backend } — { $report }
settings-report = { $families } familles · { $strata } strates · { $kinds } types de lieux · { $locations } lieux · { $crops } cultures · { $varieties } variétés · { $plantings } plantations · { $harvests } récoltes

## Strata screen
nav-strata = Strates
title-strata = Strates
strata-list-title = Strates de végétation
empty-strata = Aucune strate enregistrée.
button-delete = Supprimer
button-create-strata = Créer la strate
section-new-strata = Nouvelle strate
strata-in-use = Strate utilisée par une culture — suppression impossible.
label-strata-name = Nom
placeholder-strata-name = Canopée
label-strata-description = Description (optionnel)
placeholder-strata-description = arbres adultes au-dessus de 6 m
label-strata-height-min = Hauteur min (m)
placeholder-strata-height-min = 6
label-strata-height-max = Hauteur max (m)
placeholder-strata-height-max = 40
label-strata-sort-order = Ordre d'affichage
placeholder-strata-sort-order = 10
status-strata-created = Strate créée
status-strata-deleted = Strate supprimée

## Locations screen
title-locations = Lieux
locations-list-title = Hiérarchie des lieux
empty-locations = Aucun lieu pour l'instant.
new-location-section = Nouveau lieu
label-loc-name = Nom
placeholder-loc-name = Planche B
label-loc-kind = Type
label-loc-length = Longueur (m)
placeholder-loc-length = 25
label-loc-width = Largeur (m)
placeholder-loc-width = 0.8
label-loc-parent = Lieu parent
label-loc-notes = Notes (optionnel)
placeholder-loc-notes = orientation est-ouest, exposition sud…
button-create-location = Créer le lieu
parent-none = (racine — aucun parent)
status-location-created = Lieu créé

## Common nouns
crop = Culture
family = Famille
location = Lieu
planting = Plantation
strata = Strate
variety = Variété
yearly-harvest = Récolte annuelle

## Errors
error-config = Erreur de configuration : { $message }
error-database = Erreur de base de données
error-empty-name = Le nom ne peut pas être vide
error-non-positive-area = La surface doit être strictement positive (reçu : { $value })
error-not-found = Élément introuvable : { $kind } { $id }
error-name-required = Le nom est requis.
error-date-invalid = Date invalide (format attendu : AAAA-MM-JJ).
error-number-invalid = Nombre invalide.
error-positive-required = La valeur doit être strictement positive.
error-year-required = L'année est requise.
error-height-range = La hauteur minimale doit être inférieure ou égale à la maximale.
status-validation-failed = { $message }

## Help / manual
nav-help = Aide
status-manual-opened = Manuel ouvert dans le visualiseur système.
status-manual-not-found = Manuel introuvable. Réinstallez Pomone ou consultez le PDF en ligne.
status-manual-open-failed = Impossible d'ouvrir le manuel : { $message }

## Task Calendar
nav-tasks = Tâches
title-task-calendar = Calendrier des tâches
task-calendar-empty = Aucune tâche planifiée ce mois-ci.
task-calendar-hint = Cliquez sur une tâche pour la modifier (date, type, état, suppression).
task-calendar-new-task = + Nouvelle tâche
task-calendar-filter-hint = Filtrer par type :
task-calendar-filter-all = Tout afficher

## Task form (create / edit)
task-form-title-new = Nouvelle tâche
task-form-title-edit = Modifier la tâche
task-form-planting-none = — Aucune —
label-task-type = Type
label-task-planting = Plantation (optionnel)
label-task-planned-on = Date prévue
label-task-notes = Notes
placeholder-task-notes = Précisions, conditions, observations…
label-task-completed = Tâche terminée
btn-task-save = Enregistrer
btn-task-cancel = Annuler
btn-task-delete = Supprimer

## Task form errors / status
error-date-required = La date est requise.
error-task-no-types = Aucun type de tâche disponible. Réinitialisez les données par défaut.
error-task-type-required = Le type de tâche est requis.
error-task-edit-id-missing = Identifiant de tâche manquant pour l'édition.
status-task-failed = Échec de l'opération sur la tâche : { $message }

## Task Types catalog
title-task-types = Types de tâches
task-types-button = Gérer les types
task-types-list-title = Catalogue
task-types-empty = Aucun type défini. Utilisez le formulaire à droite pour en créer.
task-types-form-section-create = Nouveau type
task-types-form-section-edit = Modifier le type
label-task-type-name = Nom
placeholder-task-type-name = Désherbage manuel, Tonte, Fauchage…
label-task-type-category = Catégorie
label-task-type-color = Couleur
placeholder-task-type-color = #3C6E47
hint-task-type-color = Format hexadécimal : « #RGB » ou « #RRGGBB ».
btn-task-type-save = Enregistrer
btn-task-type-cancel = Annuler la modification
btn-task-type-back = ← Retour au calendrier
btn-task-type-edit = Modifier
btn-task-type-delete = Supprimer
task-type-in-use = Utilisé
error-task-type-color-required = La couleur est requise.
error-task-type-edit-id-missing = Identifiant du type manquant pour l'édition.
error-task-type-category-required = La catégorie est requise.
error-task-type-in-use = Ce type est utilisé par des tâches existantes. Supprimez ou réaffectez ces tâches avant de pouvoir supprimer le type.
status-task-type-failed = Échec de l'opération sur le type : { $message }

## Task categories (stable enum labels)
category-sow = Semis
category-transplant = Repiquage
category-harvest = Récolte
category-weeding = Désherbage
category-irrigation = Irrigation
category-treatment = Traitement
category-tillage = Travail du sol
category-other = Autre

## Gantt timeline
section-season = Saison
empty-season = Aucune plantation annuelle pour cette saison. Créez-en depuis l'écran Plantations.
section-gantt = Vue Gantt de la saison
gantt-month-1 = Janv.
gantt-month-2 = Févr.
gantt-month-3 = Mars
gantt-month-4 = Avril
gantt-month-5 = Mai
gantt-month-6 = Juin
gantt-month-7 = Juil.
gantt-month-8 = Août
gantt-month-9 = Sept.
gantt-month-10 = Oct.
gantt-month-11 = Nov.
gantt-month-12 = Déc.
