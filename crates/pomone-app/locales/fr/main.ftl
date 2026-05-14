# Pomone — French translations.
#
# Keep keys in alphabetical order within each section to make merges easier.
# Variable references use { $name }. Selectors and plural forms (e.g. for
# crop counts) can be added as the UI requires them.

## Application
welcome = Bienvenue dans Pomone
welcome-user = Bienvenue, { $name } !
welcome-summary = Pomone gère vos cultures annuelles et pluriannuelles, du maraîchage à l'agroforesterie.
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
