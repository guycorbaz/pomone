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
nav-group-planning = Planification
nav-group-catalog = Catalogue
nav-group-system = Système
nav-home = Accueil
nav-plantings = Plantations
nav-cultures = Cultures
nav-locations = Lieux
label-variety = Variété
label-location = Lieu
label-sown-on = Date de semis
label-planting-method = Méthode de mise en culture
label-planting-date = Date de plantation
method-direct-sow = Semis direct sur place
method-raised-transplant = Semis puis repiquage
method-bought-plants = Plants achetés
label-established-on = Date de plantation
label-removal-on = Arrachage prévu (optionnel)
placeholder-removal-date = AAAA-MM-JJ (laisser vide)
label-area = Surface (m²)
label-plants-count = Nombre de plants
placeholder-date = AAAA-MM-JJ
placeholder-area = 20
placeholder-count = 100
plants-suffix = plants
section-new-planting = Nouvelle plantation annuelle
title-plantings = Plantations
empty-plantings = Aucune plantation pour l'instant. Renseignez le formulaire ci-dessous pour en créer une.
status-planting-created = Plantation créée
status-planting-failed = Échec de la création : { $message }

## Cultures + Variétés screen
title-cultures = Cultures et variétés
crops-title = Cultures
empty-crops = Aucune culture pour l'instant.
varieties-title = Variétés
empty-varieties = Aucune variété pour cette culture.
no-crop-selected = Sélectionnez une culture pour voir et créer ses variétés.
new-crop-section = Nouvelle culture annuelle
new-variety-section = Nouvelle variété annuelle
new-variety-section-pluriannual = Nouvelle variété pluriannuelle
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
status-variety-deleted = Variété supprimée
confirm-delete-variety = Supprimer cette variété ? Cette action est irréversible.
error-variety-in-use = Cette variété est utilisée par une plantation ; supprimez d'abord ces plantations.
status-crop-updated = Culture mise à jour
status-crop-deleted = Culture supprimée
button-edit = Modifier
button-save-crop = Enregistrer
button-cancel-crop-edit = Annuler
crop-form-section-edit = Modifier la culture
crop-in-use = Utilisée
confirm-delete-crop = Supprimer cette culture ? Ses variétés seront aussi supprimées. Impossible si une variété est déjà plantée.
error-crop-in-use = Cette culture a une variété déjà plantée ; supprimez d'abord ces plantations.

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
section-planting-tasks = Tâches
empty-planting-tasks = Aucune tâche pour cette plantation.
task-badge-overdue = en retard
task-badge-done = fait

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
calendar-prev = ‹
calendar-next = ›
calendar-today = Aujourd'hui
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
event-transplanting-glyph = R
event-harvest-start-glyph = ▶
event-harvest-end-glyph = ◼
event-establishment-glyph = P
event-removal-glyph = ✕
event-bud-break-glyph = B
event-flowering-glyph = F

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
settings-migrate-target-not-empty = La base de destination contient déjà des données. Pointez la migration vers une base vide et neuve.
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
status-location-deleted = Lieu supprimé
status-location-updated = Lieu mis à jour
loc-form-section-edit = Modifier le lieu
error-location-cycle = Un lieu ne peut pas devenir un sous-lieu de lui-même.
confirm-delete-location = Supprimer ce lieu ? Cette action est irréversible.
error-location-in-use = Ce lieu contient d'autres lieux ou des plantations ; supprimez-les d'abord.

## Common nouns
crop = Culture
family = Famille
location = Lieu
planting = Plantation
strata = Strate
variety = Variété
yearly-harvest = Récolte annuelle

## Planting life-cycle status (issue #63)
planting-status-active = En cours
planting-status-completed = Terminée
planting-status-failed = Échouée
planting-status-abandoned = Abandonnée

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
error-count-positive = Le nombre doit être strictement positif.
error-harvest-window = Le cycle doit couvrir une fenêtre de récolte non vide.
error-date-range = Date hors de la plage autorisée.
error-unexpected = Erreur inattendue : { $message }
status-validation-failed = { $message }

## Help / manual
nav-help = Aide
status-manual-opened = Manuel ouvert dans le visualiseur système.
status-manual-not-found = Manuel introuvable. Réinstallez Pomone ou consultez le PDF en ligne.
status-manual-open-failed = Impossible d'ouvrir le manuel : { $message }

# Prioritized to-do list (overdue / today / upcoming). Internally still the
# "agenda" view, but surfaced to users as « Tâches » — the dated month grid
# below carries the « Agenda » label instead (closer to its calendar nature).
## Agenda (shown as « Tâches »)
nav-agenda = Tâches
title-agenda = Tâches
agenda-empty = Aucune tâche.

## Confirmation dialog
confirm-delete-title = Confirmer la suppression
confirm-ok = Supprimer
confirm-cancel = Annuler
confirm-delete-strata = Supprimer cette strate ? Cette action est irréversible.
confirm-delete-task = Supprimer cette tâche ? Cette action est irréversible.
confirm-delete-task-type = Supprimer ce type de tâche ? Cette action est irréversible.
confirm-delete-planting = Supprimer cette plantation ? Cette action est irréversible. Elle n'est possible que si aucune activité n'a été enregistrée.

## Planting life cycle (issue #63)
section-planting-lifecycle = Cycle de vie
label-status = Statut
button-change-status = Changer le statut
button-delete-planting = Supprimer la plantation
status-planting-status-updated = Statut mis à jour
status-planting-deleted = Plantation supprimée
agenda-overdue-title = En retard
agenda-today-title = Aujourd'hui

## Unified calendar (tasks + crop-cycle milestones)
nav-tasks = Calendrier
title-task-calendar = Calendrier
task-calendar-empty = Aucune tâche planifiée ce mois-ci.
task-calendar-hint = Cliquez sur une tâche pour la modifier (date, type, état, suppression), ou glissez-la sur un autre jour pour la reprogrammer.
task-calendar-new-task = + Nouvelle tâche
task-calendar-filter-hint = Filtrer par type :
task-calendar-filter-all = Tout afficher
task-calendar-filter-milestones = Jalons de culture
task-calendar-legend-task = Tâche
task-calendar-legend-milestone = Jalon
task-calendar-summary = { $tasks } tâches · { $milestones } jalons

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
error-recurrence-unit-required = L'unité de récurrence est requise.
status-task-failed = Échec de l'opération sur la tâche : { $message }

## Task form — recurrence sub-section
label-task-recurring = Tâche récurrente
label-task-recurrence-interval = Intervalle
placeholder-task-recurrence-interval = 7
label-task-recurrence-unit = Unité
label-task-recurrence-end-on = Fin (incluse)
placeholder-task-recurrence-end-on = AAAA-MM-JJ
hint-task-recurrence-end-on = Laissez vide pour une série sans fin : le calendrier sera étendu automatiquement jusqu'à un an à l'avance.
recurrence-unit-days = jours
recurrence-unit-weeks = semaines
recurrence-unit-months = mois
task-form-series-badge = Cette tâche fait partie d'une série récurrente. Pour modifier la série, supprimez les occurrences et recréez-la.

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

## Familles (catalogue)
nav-families = Familles
title-families = Familles
confirm-delete-family = Supprimer cette famille ? Cette action est irréversible.
families-list-title = Catalogue
families-empty = Aucune famille définie. Utilisez le formulaire à droite pour en ajouter une.
families-form-section-create = Nouvelle famille
families-form-section-edit = Modifier la famille
label-family-name = Nom
placeholder-family-name = Solanacées, Rosacées…
label-family-latin = Nom latin
placeholder-family-latin = Solanaceae
label-family-description = Description
placeholder-family-description = tomate, pomme de terre, poivron…
label-family-color = Couleur
placeholder-family-color = #B85C38
hint-family-color = Format hexadécimal : « #RVB » ou « #RRVVBB ». Teinte les plantations et la carte des cultures par famille.
btn-family-save = Enregistrer
btn-family-cancel = Annuler la modification
btn-family-edit = Modifier
btn-family-delete = Supprimer
family-in-use = Utilisée
error-family-color-required = La couleur est requise.
error-family-edit-id-missing = Identifiant de famille manquant pour la modification.
error-family-in-use = Cette famille est utilisée par des cultures existantes. Réaffectez ou supprimez ces cultures avant de supprimer la famille.
status-family-failed = Échec de l'opération sur la famille : { $message }

## Task categories (stable enum labels)
category-sow = Semis
category-transplant = Repiquage
category-harvest = Récolte
category-weeding = Désherbage
category-irrigation = Irrigation
category-treatment = Traitement
category-tillage = Travail du sol
category-other = Autre

## Crop Map
nav-crop-map = Carte
title-crop-map = Carte des cultures
crop-map-hint = Cliquez sur une barre pour la sélectionner. Vous pourrez ensuite la déplacer vers un autre lieu ou la diviser entre plusieurs lieux.
crop-map-empty = Aucun lieu défini pour le moment. Créez-en depuis l'écran Lieux.
btn-crop-map-move = Déplacer vers…
btn-crop-map-split = Diviser
btn-crop-map-deselect = Désélectionner
crop-map-picker-title = Choisir un lieu de destination
crop-map-picker-cancel = Annuler
crop-map-split-title = Diviser la plantation
crop-map-split-hint = Répartissez la plantation entre deux lieux. La part A conserve l'historique (tâches, récoltes) ; la part B est créée fraîche. Les totaux ne sont pas vérifiés — à vous de choisir.
crop-map-split-part-a = Part A (conserve l'historique)
crop-map-split-part-b = Part B (nouvelle plantation)
crop-map-split-location = Lieu
crop-map-split-area = Surface (m²)
crop-map-split-count = Nombre de plants
crop-map-split-placeholder-area = 10
crop-map-split-placeholder-count = 50
crop-map-split-confirm = Diviser
crop-map-split-cancel = Annuler
error-no-planting-selected = Aucune plantation sélectionnée.
error-location-required = Le lieu de destination est requis.
error-planting-has-activity = Cette plantation a une activité enregistrée (tâches faites ou heures saisies) ; elle ne peut pas être supprimée. Marquez-la plutôt comme terminée, échouée ou abandonnée.

## Bed-usage curve (home)
bed-usage-legend-open = Plein champ
bed-usage-legend-sheltered = Sous abri

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
