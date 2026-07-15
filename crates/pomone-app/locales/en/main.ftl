# Pomone — English translations.
#
# Mirror of fr/main.ftl. Keys in alphabetical order within each section.

## Application
welcome = Welcome to Pomone
welcome-user = Welcome, { $name }!
welcome-summary = Pomone manages your annual and pluriannual crops: market gardening, field crops, orcharding and agroforestry.
button-switch-language = Français
button-create-planting = Create planting
nav-group-planning = Planning
nav-group-catalog = Catalog
nav-group-system = System
nav-home = Home
nav-plantings = Plantings
nav-cultures = Crops
nav-locations = Locations
label-variety = Variety
label-location = Location
label-sown-on = Sowing date
label-planting-method = Establishment method
label-planting-date = Planting date
method-direct-sow = Direct sow in place
method-raised-transplant = Sow then transplant
method-bought-plants = Bought plants
label-established-on = Planting date
label-removal-on = Expected removal (optional)
placeholder-removal-date = YYYY-MM-DD (leave empty)
label-area = Area ({ $unit })
label-plants-count = Plant count
placeholder-date = YYYY-MM-DD
placeholder-area = 20
placeholder-count = 100
plants-suffix = plants
section-new-planting = New annual planting
title-plantings = Plantings
empty-plantings = No plantings yet. Fill in the form below to create one.
plantings-col-variety = Variety
plantings-col-location = Location
plantings-col-schedule = Schedule
plantings-col-area = Area
plantings-col-plants = Plants
plantings-col-status = Status
status-planting-created = Planting created
status-planting-failed = Creation failed: { $message }

## Cultures + Varieties screen
title-cultures = Crops and varieties
crops-title = Crops
empty-crops = No crops yet.
varieties-title = Varieties
empty-varieties = No varieties for this crop.
no-crop-selected = Pick a crop to see and create its varieties.
new-crop-section = New annual crop
new-variety-section = New annual variety
new-variety-section-pluriannual = New pluriannual variety
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
status-variety-updated = Variety updated
status-variety-deleted = Variety deleted
variety-form-section-edit = Edit variety
button-save-variety = Save
button-cancel-variety-edit = Cancel
confirm-delete-variety = Delete this variety? This cannot be undone.
error-variety-in-use = This variety is used by a planting; delete those plantings first.
status-crop-updated = Crop updated
status-crop-deleted = Crop deleted
button-edit = Edit
button-save-crop = Save
button-cancel-crop-edit = Cancel
crop-form-section-edit = Edit crop
crop-in-use = In use
confirm-delete-crop = Delete this crop? Its varieties are deleted too. Not possible if a variety is already planted.
error-crop-in-use = This crop has a planted variety; delete those plantings first.

## Planting detail screen
title-planting-detail = Planting detail
button-back = ‹ Back
section-schedule = Schedule
section-summary = Summary
label-planting-name = Name
label-planting-notes = Notes
label-transplanted-on = Transplant date
label-first-harvest = First harvest
label-last-harvest = Last harvest
empty-planting-detail = Planting not found.
section-planting-tasks = Tasks
empty-planting-tasks = No task for this planting yet.
task-badge-overdue = overdue
task-badge-done = done

## Yearly harvest (perennials)
section-yearly-harvest = Yearly harvests
section-record-harvest = Record a harvest
empty-yearly-harvest = No harvest recorded yet.
harvest-header-year = Year
harvest-header-expected = Expected
harvest-header-actual = Actual
harvest-header-variance = Variance
harvest-header-notes = Notes
label-harvest-year = Year
label-harvest-expected = Expected yield ({ $unit })
label-harvest-actual = Actual yield ({ $unit })
label-harvest-notes = Notes (optional)
placeholder-harvest-year = 2030
placeholder-harvest-kg = 50
placeholder-harvest-notes = late frost, hard pruning…
button-record-harvest = Record
status-harvest-recorded = Harvest recorded

## Treatments (phytosanitary traceability, issue #82)
section-treatments = Phytosanitary treatments
section-record-treatment = Record a treatment
empty-treatments = No treatment recorded yet.
treatment-header-date = Date
treatment-header-substance = Active substance
treatment-header-product = Product
treatment-header-dose = Dose
treatment-header-notes = Notes
label-treatment-date = Date (YYYY-MM-DD)
label-treatment-substance = Active substance
label-treatment-product = Product (brand)
label-treatment-dose = Dose
label-treatment-unit = Unit
label-treatment-notes = Notes (optional)
placeholder-treatment-substance = copper, sulphur…
placeholder-treatment-product = Bordeaux mixture
placeholder-treatment-dose = 1.25
placeholder-treatment-unit = kg/ha
placeholder-treatment-notes = weather, operator…
button-record-treatment = Record
button-delete-treatment = Delete
status-treatment-recorded = Treatment recorded
status-treatment-deleted = Treatment deleted

## Calendar screen
calendar-prev = ‹
calendar-next = ›
calendar-today = Today
weekday-mon-short = Mo
weekday-tue-short = Tu
weekday-wed-short = We
weekday-thu-short = Th
weekday-fri-short = Fr
weekday-sat-short = Sa
weekday-sun-short = Su
month-1 = January
month-2 = February
month-3 = March
month-4 = April
month-5 = May
month-6 = June
month-7 = July
month-8 = August
month-9 = September
month-10 = October
month-11 = November
month-12 = December
event-sowing-glyph = S
event-transplanting-glyph = T
event-harvest-start-glyph = ▶
event-harvest-end-glyph = ◼
event-establishment-glyph = P
event-removal-glyph = ✕
event-bud-break-glyph = B
event-flowering-glyph = F

## Settings screen
nav-settings = Settings
title-settings = Settings
settings-current-section = Active database
settings-current-label = Backend
settings-edit-section = Change database
settings-backend-kind-label = Backend kind
settings-backend-sqlite = SQLite (local file)
settings-backend-mariadb = MariaDB / MySQL (remote)
settings-sqlite-path-label = File path
settings-sqlite-path-placeholder = /path/to/pomone.sqlite
settings-mariadb-host-label = Host
settings-mariadb-host-placeholder = mariadb.example.com
settings-mariadb-port-label = Port
settings-mariadb-port-placeholder = 3306
settings-mariadb-user-label = User
settings-mariadb-user-placeholder = pomone
settings-mariadb-password-label = Password
settings-mariadb-password-placeholder = ••••••••
settings-mariadb-database-label = Database
settings-mariadb-database-placeholder = pomone
settings-button-test = Test connection
settings-button-save = Switch without copying
settings-button-save-migrate = Migrate data
settings-migrate-warning = "Migrate data" copies every record into the new database before switching. "Switch without copying" opens the new database with default seed data only — useful for starting fresh.
settings-test-ok = Connection successful.
settings-save-ok = Switched database to: { $backend }
settings-migrate-ok = Migration done into: { $backend } — { $report }
settings-migrate-target-not-empty = The destination database already contains data. Point the migration at a fresh, empty database.
settings-report = { $families } families · { $strata } strata · { $kinds } location kinds · { $locations } locations · { $crops } crops · { $varieties } varieties · { $plantings } plantings · { $harvests } harvests · { $tasktypes } task types · { $taskmethods } methods · { $taskimplements } implements · { $taskseries } series · { $tasks } tasks · { $treatments } treatments

## Backup (issue #58)
section-backup = Backup
settings-backup-explain = Instant copy of the SQLite file, created next to the database. An automatic backup is also taken before every backend switch. For MariaDB, use the server's tooling (mysqldump).
button-backup-now = Back up now
settings-backup-done = Backup created: { $path }
settings-backup-note = Automatic backup: { $path }
settings-refresh-warning = Warning: some screens could not be reloaded ({ $screens }) — reopen them or restart the application.
error-backup-sqlite-only = Built-in backup only covers the SQLite backend.

## Strata screen
nav-strata = Strata
title-strata = Strata
strata-list-title = Vegetation strata
empty-strata = No strata recorded.
button-delete = Delete
button-create-strata = Create stratum
section-new-strata = New stratum
strata-in-use = Stratum is referenced by a crop — cannot delete.
label-strata-name = Name
placeholder-strata-name = Canopy
label-strata-description = Description (optional)
placeholder-strata-description = mature trees above 6 m
label-strata-height-min = Min height (m)
placeholder-strata-height-min = 6
label-strata-height-max = Max height (m)
placeholder-strata-height-max = 40
label-strata-sort-order = Display order
placeholder-strata-sort-order = 10
status-strata-created = Stratum created
status-strata-deleted = Stratum deleted

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
status-location-deleted = Location deleted
status-location-updated = Location updated
loc-form-section-edit = Edit location
error-location-cycle = A location cannot become a sub-location of itself.
confirm-delete-location = Delete this location? This cannot be undone.
error-location-in-use = This location holds child locations or plantings; delete those first.

## Common nouns
crop = Crop
family = Family
location = Location
planting = Planting
strata = Stratum
variety = Variety
yearly-harvest = Yearly harvest

## Planting life-cycle status (issue #63)
planting-status-active = Active
planting-status-completed = Completed
planting-status-failed = Failed
planting-status-abandoned = Abandoned

## Errors
error-config = Configuration error: { $message }
error-database = Database error
error-empty-name = Name cannot be empty
error-non-positive-area = Area must be strictly positive (got: { $value })
error-not-found = Not found: { $kind } { $id }
error-name-required = Name is required.
error-date-invalid = Invalid date (expected format: YYYY-MM-DD).
error-number-invalid = Invalid number.
error-positive-required = Value must be strictly positive.
error-year-required = Year is required.
error-height-range = Minimum height must be less than or equal to maximum.
error-count-positive = Count must be strictly positive.
error-harvest-window = The cycle must span a non-empty harvest window.
error-date-range = Date is outside the allowed range.
error-unexpected = Unexpected error: { $message }
status-validation-failed = { $message }

## Help / manual
nav-help = Help
status-manual-opened = Manual opened in your system viewer.
status-manual-not-found = Manual not found. Reinstall Pomone or read the PDF online.
status-manual-open-failed = Couldn't open the manual: { $message }

# Prioritized to-do list (overdue / today / upcoming). Internally still the
# "agenda" view, but surfaced to users as "Tasks" — the dated month grid
# below carries the "Agenda" label instead (closer to its calendar nature).
## Agenda (shown as "Tasks")
nav-agenda = Tasks
title-agenda = Tasks
agenda-empty = No tasks.

## Confirmation dialog
confirm-delete-title = Confirm deletion
confirm-ok = Delete
confirm-cancel = Cancel
confirm-delete-strata = Delete this stratum? This cannot be undone.
confirm-delete-treatment = Delete this treatment? This cannot be undone.
confirm-delete-task = Delete this task? This cannot be undone.
confirm-delete-task-type = Delete this task type? This cannot be undone.
confirm-delete-planting = Delete this planting? This cannot be undone, and is only allowed when no activity has been recorded.

## Planting life cycle (issue #63)
section-planting-lifecycle = Life cycle
label-status = Status
button-change-status = Change status
button-delete-planting = Delete planting
status-planting-status-updated = Status updated
status-planting-deleted = Planting deleted
agenda-overdue-title = Overdue
agenda-today-title = Today
agenda-skipped-title = Skipped

## Plan grid (Epic 2, story 2.3)
nav-plan = Plan
title-plan = Crop plan
plan-hint = Spreadsheet-style entry: type in a cell, Enter validates the row (inline refusal if invalid). Grey columns (dates) are computed, not editable.
plan-empty = No plan lines yet. Add one to get started.
plan-add-line = + Add a line
plan-derived-placeholder = —
plan-col-variety = Variety
plan-col-series = Series
plan-col-bed-meters = m/series
plan-col-stagger = Stagger (d)
plan-col-first-on = First date
plan-col-derived = Dates (computed)
plan-col-needs = Need (m)
plan-col-draft = Draft
plan-col-notes = Notes
plan-draft-badge = Draft
plan-no-variety = Add a variety (Crops) before creating a plan line.
plan-generate-label = Generate
plan-generated = { $count } planned planting(s) generated.
plan-generate-draft = Can't generate a draft line — promote it first.
plan-generate-no-date = Set a first date before generating.
# Needs list «Besoins» (story 2.7)
nav-needs = Needs
title-needs = Needs list
needs-hint = Per-variety totals aggregated from every non-draft plan line, placed or not. «Buy by» is the earliest sowing date — order the seed or plants before then.
needs-empty = Nothing to order yet. Add non-draft plan lines with dates.
needs-col-variety = Variety
needs-col-quantity = Quantity (m)
needs-col-buy-by = Buy by
needs-col-lines = Lines
needs-buy-by-none = no date
needs-print = Print
needs-print-disabled = Printing arrives in a later version (documents).
# Row ⋯ settle menu (story 1.5)
agenda-menu-done = Mark done
agenda-menu-skip = Skip…
agenda-menu-correct = Correct
# Skip-reason dialog (story 1.5)
skip-dialog-title = Skip the task
skip-dialog-note-placeholder = Note (optional)
skip-dialog-ok = Skip
skip-dialog-cancel = Cancel
status-task-done = Task marked done
status-task-skipped = Task skipped
status-task-reopened = Task reopened

## Unified calendar (tasks + crop-cycle milestones)
nav-tasks = Calendar
title-task-calendar = Calendar
task-calendar-empty = No tasks planned this month.
task-calendar-hint = Click a task to edit it (date, type, state, deletion), or drag it onto another day to reschedule it.
task-calendar-new-task = + New task
task-calendar-filter-hint = Filter by type:
task-calendar-filter-all = Show all
task-calendar-filter-milestones = Crop milestones
task-calendar-legend-task = Task
task-calendar-legend-milestone = Milestone
task-calendar-summary = { $tasks } tasks · { $milestones } milestones

## Task form (create / edit)
task-form-title-new = New task
task-form-title-edit = Edit task
task-form-planting-none = — None —
label-task-type = Type
label-task-planting = Planting (optional)
label-task-planned-on = Planned date
label-task-notes = Notes
placeholder-task-notes = Details, conditions, observations…
label-task-completed = Task done
btn-task-save = Save
btn-task-cancel = Cancel
btn-task-delete = Delete

## Task form errors / status
error-date-required = Date is required.
error-task-no-types = No task types available. Reset the default data.
error-task-type-required = Task type is required.
error-task-edit-id-missing = Missing task ID for editing.
error-recurrence-unit-required = Recurrence unit is required.
status-task-failed = Task operation failed: { $message }

## Task form — recurrence sub-section
label-task-recurring = Recurring task
label-task-recurrence-interval = Interval
placeholder-task-recurrence-interval = 7
label-task-recurrence-unit = Unit
label-task-recurrence-end-on = End (inclusive)
placeholder-task-recurrence-end-on = YYYY-MM-DD
hint-task-recurrence-end-on = Leave empty for an open-ended series: the calendar will extend automatically up to one year ahead.
recurrence-unit-days = days
recurrence-unit-weeks = weeks
recurrence-unit-months = months
task-form-series-badge = This task is part of a recurring series. To modify the series, delete the occurrences and recreate it.

## Task Types catalog
title-task-types = Task types
task-types-button = Manage types
task-types-list-title = Catalog
task-types-empty = No types defined yet. Use the form on the right to add one.
task-types-form-section-create = New type
task-types-form-section-edit = Edit type
label-task-type-name = Name
placeholder-task-type-name = Manual weeding, Mowing, Scything…
label-task-type-category = Category
label-task-type-color = Color
placeholder-task-type-color = #3C6E47
hint-task-type-color = Hexadecimal format: "#RGB" or "#RRGGBB".
btn-task-type-save = Save
btn-task-type-cancel = Cancel edit
btn-task-type-back = ← Back to calendar
btn-task-type-edit = Edit
btn-task-type-delete = Delete
task-type-in-use = In use
error-task-type-color-required = Color is required.
error-task-type-edit-id-missing = Missing type ID for editing.
error-task-type-category-required = Category is required.
error-task-type-in-use = This type is used by existing tasks. Delete or reassign those tasks before deleting the type.
status-task-type-failed = Type operation failed: { $message }

## Families catalog
nav-families = Families
title-families = Families
confirm-delete-family = Delete this family? This cannot be undone.
families-list-title = Catalog
families-empty = No families defined yet. Use the form on the right to add one.
families-form-section-create = New family
families-form-section-edit = Edit family
label-family-name = Name
placeholder-family-name = Solanaceae, Rosaceae…
label-family-latin = Latin name
placeholder-family-latin = Solanaceae
label-family-description = Description
placeholder-family-description = tomato, potato, pepper…
label-family-color = Color
placeholder-family-color = #B85C38
hint-family-color = Hexadecimal format: "#RGB" or "#RRGGBB". Tints plantings and the crop map by family.
btn-family-save = Save
btn-family-cancel = Cancel edit
btn-family-edit = Edit
btn-family-delete = Delete
family-in-use = In use
error-family-color-required = Color is required.
error-family-edit-id-missing = Missing family ID for editing.
error-family-in-use = This family is used by existing crops. Reassign or delete those crops before deleting the family.
status-family-failed = Family operation failed: { $message }

## Task categories (stable enum labels)
category-sow = Sowing
category-transplant = Transplanting
category-harvest = Harvest
category-weeding = Weeding
category-irrigation = Irrigation
category-treatment = Treatment
category-tillage = Tillage
category-other = Other

## Crop Map
nav-crop-map = Map
title-crop-map = Crop Map
crop-map-hint = Click a bar to select it. You can then move it to another location or split it between several locations.
crop-map-empty = No location defined yet. Add some from the Locations screen.
btn-crop-map-move = Move to…
btn-crop-map-split = Split
btn-crop-map-deselect = Deselect
crop-map-picker-title = Pick a destination location
crop-map-picker-cancel = Cancel
crop-map-split-title = Split the planting
crop-map-split-hint = Split the planting between two locations. Part A keeps the history (tasks, harvests); part B is created fresh. Totals are not checked — your call.
crop-map-split-part-a = Part A (keeps the history)
crop-map-split-part-b = Part B (new planting)
crop-map-split-location = Location
crop-map-split-area = Area ({ $unit })
crop-map-split-count = Plants count
crop-map-split-placeholder-area = 10
crop-map-split-placeholder-count = 50
crop-map-split-confirm = Split
crop-map-split-cancel = Cancel
error-no-planting-selected = No planting selected.
error-location-required = Destination location is required.
error-planting-has-activity = This planting has recorded activity (completed tasks or logged hours) and cannot be deleted. Mark it completed, failed, or abandoned instead.

## Bed-usage curve (home)
bed-usage-legend-open = Open field
bed-usage-legend-sheltered = Under cover

## Gantt timeline
section-season = Season
empty-season = No annual plantings yet. Add some on the Plantings screen.
section-gantt = Season Gantt view
gantt-month-1 = Jan
gantt-month-2 = Feb
gantt-month-3 = Mar
gantt-month-4 = Apr
gantt-month-5 = May
gantt-month-6 = Jun
gantt-month-7 = Jul
gantt-month-8 = Aug
gantt-month-9 = Sep
gantt-month-10 = Oct
gantt-month-11 = Nov
gantt-month-12 = Dec

## Contextual tooltips (#39) — hover help.
## Sidebar: recalls what the screen does + its keyboard shortcut.
tooltip-nav-home = Overview: bed occupation and season Gantt. (Ctrl+1)
tooltip-nav-plantings = Planting list, creation form and Gantt view. (Ctrl+2)
tooltip-nav-tasks = Monthly calendar of tasks and crop milestones. (Ctrl+3)
tooltip-nav-agenda = Every task as a list: overdue, today, upcoming. (Ctrl+4)
tooltip-nav-crop-map = Location occupation over time; move or split a planting. (Ctrl+9)
tooltip-nav-cultures = Catalog of crops and their varieties. (Ctrl+5)
tooltip-nav-locations = Catalog of locations: gardens, beds, greenhouses… (Ctrl+6)
tooltip-nav-strata = Vegetation strata (canopy, shrub, ground cover…). (Ctrl+7)
tooltip-nav-families = Botanical families and their display colour.
tooltip-nav-settings = Database (SQLite/MariaDB), backups. (Ctrl+8)
tooltip-nav-help = Opens the user manual (PDF). (F1)
tooltip-nav-language = Switches the interface language between French and English.

## New-planting form
tooltip-planting-variety = Variety to plant. Managed under Crops → varieties.
tooltip-planting-location = Location hosting the planting (garden, bed, greenhouse…).
tooltip-planting-strata = Vegetation layer occupied — useful in agroforestry; "Ground cover" fits market gardening.
tooltip-planting-method = Establishment method: direct sowing, raised then transplanted, or bought plants.
tooltip-planting-sown-on = Sowing date (or planting date for bought plants), as YYYY-MM-DD.
tooltip-planting-established-on = Establishment date of the perennial crop, as YYYY-MM-DD.
tooltip-planting-removal-on = Expected removal date (optional), as YYYY-MM-DD.
tooltip-planting-area = Area used in { $unit } (decimals accepted, e.g. 12.5).
tooltip-planting-count = Number of plants.
tooltip-planting-create = Creates the planting and auto-generates its tasks (sowing, transplanting, harvest…).

## Task form
tooltip-task-type = Kind of operation (sowing, weeding, harvest…). Manage the list via "Manage types".
tooltip-task-planting = Planting this task belongs to — "None" for a general task.
tooltip-task-planned-on = Planned date of the task, as YYYY-MM-DD.
tooltip-task-completed = Tick when the task is done; it shows greyed out in lists.
tooltip-task-notes = Free notes: product used, weather, remarks…
tooltip-task-recurring = Repeats the task at a regular interval (watering, mowing…). The series is created on save.
tooltip-task-recurrence-interval = Number of units between two occurrences (e.g. 7 with "days" = weekly).
tooltip-task-recurrence-unit = Unit of the interval: days, weeks or months.
tooltip-task-recurrence-end-on = Last date of the series, as YYYY-MM-DD.

## Unified calendar
tooltip-calendar-prev = Previous month. (←)
tooltip-calendar-next = Next month. (→)
tooltip-calendar-today = Back to the current month.
tooltip-calendar-new-task = Creates a task: type, planting, date, recurrence.
tooltip-calendar-manage-types = Manage task types: names, categories, colours.
tooltip-calendar-filter-chip = Shows or hides this task category on the calendar.
tooltip-calendar-filter-all = Shows every category again.
tooltip-calendar-milestones = Shows or hides crop milestones (sowing, harvest…) derived from plantings.

## Public holidays (#35) — names shown on the calendar.
holiday-new-year = New Year's Day
holiday-berchtold = Berchtold's Day (2 January)
holiday-neuchatel-republic = Republic Day (Neuchâtel)
holiday-st-joseph = Saint Joseph's Day
holiday-good-friday = Good Friday
holiday-easter-monday = Easter Monday
holiday-labour-day = Labour Day
holiday-victory-day = Victory Day 1945
holiday-ascension = Ascension Day
holiday-whit-monday = Whit Monday
holiday-corpus-christi = Corpus Christi
holiday-jura-independence = Jura Independence Day
holiday-bastille-day = Bastille Day
holiday-swiss-national-day = Swiss National Day
holiday-assumption = Assumption Day
holiday-geneva-fast = Geneva Fast
holiday-federal-fast-monday = Federal Fast Monday
holiday-all-saints = All Saints' Day
holiday-armistice-day = Armistice Day 1918
holiday-immaculate-conception = Immaculate Conception
holiday-christmas = Christmas Day
holiday-st-stephens = Saint Stephen's Day
holiday-geneva-restoration = Restoration of the Republic

## Holiday region (settings picker)
settings-holiday-section = Public holidays
settings-holiday-region-label = Region
settings-holiday-explain = Public holidays of the chosen region are greyed out on the calendar.
holiday-region-none = None (hide public holidays)
holiday-region-ch-vd = Switzerland — Vaud
holiday-region-ch-ge = Switzerland — Geneva
holiday-region-ch-ne = Switzerland — Neuchâtel
holiday-region-ch-fr = Switzerland — Fribourg
holiday-region-ch-vs = Switzerland — Valais
holiday-region-ch-ju = Switzerland — Jura
holiday-region-fr = France
status-holiday-region-saved = Region saved

## Display units (settings picker, issue #29)
settings-units-section = Display units
settings-units-explain = Units used to display and enter areas and yields. Data stays stored in m² and kg.
settings-area-unit-label = Areas
settings-mass-unit-label = Yields
status-units-saved = Units saved

## Weekly print — rough plain-text sheet (story 1.4)
print-empty-week = No tasks this week.
print-no-bed = (no bed)
print-skipped = skipped
print-week-title = Week of { $start } to { $end }
skip-reason-crop-failure = crop failure
skip-reason-no-time = no time
skip-reason-not-needed = not needed
skip-reason-other = other
skip-reason-pest-disease = pest / disease
skip-reason-replaced = replaced
skip-reason-weather = weather
weekday-friday = Friday
weekday-monday = Monday
weekday-saturday = Saturday
weekday-sunday = Sunday
weekday-thursday = Thursday
weekday-tuesday = Tuesday
weekday-wednesday = Wednesday
home-print-week = 🖨 Print my week (rough)
status-week-print-written = Weekly sheet saved and opened.
status-week-print-failed = Could not generate the weekly sheet.
status-week-print-saved-not-opened = Sheet saved (could not open it automatically).

## ITK editor (Epic 2, story 2.5)
itk-title = Growing schedule (ITK)
itk-empty = No activities yet. An ITK-less crop keeps variety-profile generation.
itk-none = — none —
itk-offset-placeholder = Offset (e.g. -10, 20)
itk-label-placeholder = Label (optional)
itk-add = Add activity
itk-save = Save
itk-cancel = Cancel
itk-method = Method
itk-implement = Implement
itk-edit-tip = Edit
itk-delete-tip = Delete
itk-delete-confirm = Delete this activity from the growing schedule?
itk-delete-confirm-referenced = Delete this activity? { $count } generated task(s) reference it.
