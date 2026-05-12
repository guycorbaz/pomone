# Pomone — French translations.
#
# Keep keys in alphabetical order within each section to make merges easier.
# Variable references use { $name }. Selectors and plural forms (e.g. for
# crop counts) can be added as the UI requires them.

## Application
welcome = Bienvenue dans Pomone
welcome-user = Bienvenue, { $name } !
welcome-summary = Pomone gère vos cultures annuelles et pluriannuelles, du maraîchage à l'agroforesterie.
button-refresh = Actualiser
button-switch-language = English
button-plantings = Plantations →
button-back = ← Retour
button-create-planting = Créer la plantation
label-strata-count = Strates
label-families-count = Familles
label-location-kinds-count = Types de lieux
label-variety = Variété
label-location = Lieu
label-sown-on = Date de semis
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
button-cultures = Cultures →
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

## Common nouns
crop = Culture
family = Famille
location = Lieu
planting = Plantation
strata = Strate
variety = Variété
yearly-harvest = Récolte annuelle

## Lifespan labels
lifespan-annual = Annuelle
lifespan-pluriannual = Pluriannuelle
lifespan-pluriannual-recurring = Pluriannuelle récurrente
lifespan-pluriannual-single-cycle = Bisannuelle ou cycle unique pluriannuel

## Pruning seasons
pruning-both = Hiver et été
pruning-none = Pas de taille
pruning-summer = Été
pruning-winter = Hiver

## Errors
error-config = Erreur de configuration : { $message }
error-database = Erreur de base de données
error-empty-name = Le nom ne peut pas être vide
error-non-positive-area = La surface doit être strictement positive (reçu : { $value })
error-not-found = Élément introuvable : { $kind } { $id }
