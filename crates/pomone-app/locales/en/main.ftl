# Pomone — English translations.
#
# Mirror of fr/main.ftl. Keys in alphabetical order within each section.

## Application
welcome = Welcome to Pomone
welcome-user = Welcome, { $name }!
welcome-summary = Pomone manages your annual and pluriannual crops, from market gardening to agroforestry.
button-switch-language = Français
button-create-planting = Create planting
nav-home = Home
nav-plantings = Plantings
nav-cultures = Crops
nav-locations = Locations
label-strata-count = Strata
label-families-count = Families
label-location-kinds-count = Location kinds
label-variety = Variety
label-location = Location
label-sown-on = Sowing date
label-established-on = Planting date
label-removal-on = Expected removal (optional)
placeholder-removal-date = YYYY-MM-DD (leave empty)
label-area = Area (m²)
label-plants-count = Plant count
placeholder-date = YYYY-MM-DD
placeholder-area = 20
placeholder-count = 100
section-overview = Overview
section-new-planting = New annual planting
title-plantings = Plantings
empty-plantings = No plantings yet. Fill in the form below to create one.
status-planting-created = Planting created
status-planting-failed = Creation failed: { $message }
status-pick-variety = Pick a variety and a location before creating.

## Cultures + Varieties screen
title-cultures = Crops and varieties
crops-title = Crops
empty-crops = No crops yet.
varieties-title = Varieties
empty-varieties = No varieties for this crop.
no-crop-selected = Pick a crop to see and create its varieties.
new-crop-section = New annual crop
new-variety-section = New annual variety
label-crop-name = Name
placeholder-crop-name = Tomato
label-crop-latin = Latin name (optional)
placeholder-crop-latin = Solanum lycopersicum
label-crop-family = Family
label-crop-strata = Stratum
label-lifespan = Lifespan
label-lifespan-years = Lifespan (years)
placeholder-lifespan-years = 30
label-years-to-first-yield = Years to first yield
placeholder-years-to-first-yield = 3
label-pruning = Pruning
lifespan-annual = Annual
lifespan-pluriannual-single = Pluriannual single cycle
lifespan-pluriannual-recurring = Pluriannual recurring
pruning-none-label = No pruning
pruning-winter-label = Winter pruning
pruning-summer-label = Summer pruning
pruning-both-label = Winter + summer pruning
label-bud-break-doy = Bud break (day of year)
placeholder-bud-break-doy = 80
label-flowering-doy = Flowering (day of year)
placeholder-flowering-doy = 120
label-harvest-start-doy = Harvest start (day of year)
placeholder-harvest-start-doy = 220
label-harvest-end-doy = Harvest end (day of year)
placeholder-harvest-end-doy = 280
label-yield-kg = Expected yield (kg per plant, optional)
placeholder-yield-kg = 15.5
label-variety-name = Variety name
placeholder-variety-name = Marmande
label-variety-description = Description (optional)
placeholder-variety-description = heritage variety, ribbed fruit
label-dtt = Sow→transplant days (DTT)
label-dtm = Sow→harvest days (DTM)
label-window = Harvest window (days)
placeholder-dtt = 35
placeholder-dtm = 70
placeholder-window = 60
button-create-crop = Create crop
button-create-variety = Create variety
status-crop-created = Crop created
status-variety-created = Variety created
status-pick-crop-first = Pick a crop before creating a variety.

## Locations screen
title-locations = Locations
locations-list-title = Location hierarchy
empty-locations = No locations yet.
new-location-section = New location
label-loc-name = Name
placeholder-loc-name = Bed B
label-loc-kind = Kind
label-loc-length = Length (m)
placeholder-loc-length = 25
label-loc-width = Width (m)
placeholder-loc-width = 0.8
label-loc-parent = Parent location
label-loc-notes = Notes (optional)
placeholder-loc-notes = east-west orientation, south-facing…
button-create-location = Create location
parent-none = (root — no parent)
status-location-created = Location created

## Common nouns
crop = Crop
family = Family
location = Location
planting = Planting
strata = Stratum
variety = Variety
yearly-harvest = Yearly harvest

## Errors
error-config = Configuration error: { $message }
error-database = Database error
error-empty-name = Name cannot be empty
error-non-positive-area = Area must be strictly positive (got: { $value })
error-not-found = Not found: { $kind } { $id }
