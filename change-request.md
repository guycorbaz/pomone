# Change requests

Liste tenue à la main des évolutions demandées sur Pomone, hors feuille
de route principale (`docs/architecture.md`). Chaque entrée précise le
contexte, le périmètre attendu et les critères d'acceptation. Les
demandes peuvent être traitées dans l'ordre que l'on veut ; on coche
celles qui sont closes.

---

## CR-1 — Refonte visuelle de l'interface

**Statut :** ouvert
**Source :** retour utilisateur après le MVP de la Phase 6 step 1.
**Périmètre :** crate `pomone-ui` (fichier `ui/main.slint` + glue Rust si
besoin).

**Contexte.** L'interface actuelle est fonctionnelle mais brute :
typographie par défaut, espacements minimes, aucun style propre, pas
d'iconographie, pas de couleurs cohérentes avec l'identité Pomone
(verger / fruits / jardin).

**Périmètre attendu.**

- Définir une palette et une typographie cohérentes au niveau d'un
  composant `Theme` ou de propriétés globales Slint, réutilisables sur
  tous les écrans qui viendront ensuite.
- Appliquer des espacements généreux et une hiérarchie visuelle claire
  (titres / sous-titres / corps / labels secondaires).
- Soigner les états des boutons (hover, focus, pressed) et des champs.
- Soigner la fenêtre racine : marges, fond, éventuellement une zone
  d'en-tête persistante avec le nom de l'application et la version.
- Vérifier le rendu en haute densité (HiDPI) et avec différentes
  tailles de fenêtre.
- Garder l'intégration i18n actuelle (toutes les chaînes affichées
  doivent rester pilotées par Fluent depuis Rust).

**Critères d'acceptation.**

- Le binaire `pomone` ouvre une fenêtre dont le rendu est cohérent
  avec une application desktop moderne (proche de l'allure que l'on
  attend de Slint en 2026, pas de la fenêtre par défaut).
- Le `Theme` (ou équivalent) est documenté et réutilisable pour les
  écrans à venir (Plantations, Cultures, Parcelles, Calendrier…).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  et `cargo test --workspace` passent.

**Notes d'implémentation.**

- Slint a un système de globals et de composants exportables ; un
  fichier `ui/theme.slint` exporté comme `global Theme` est l'idiome
  habituel.
- Les composants `Button`, `LineEdit`, etc. de `std-widgets.slint`
  héritent du style natif (Fluent, Cosmic, Cupertino, Material selon
  le backend). On peut soit s'appuyer dessus en les composant mieux,
  soit définir des composants Pomone propres pour avoir une identité.
- Penser au mode sombre dès le début (au moins prévoir la
  variabilisation des couleurs).

---

## CR-2 — Choisir la base de données depuis l'interface graphique

**Statut :** ouvert
**Source :** demande utilisateur, Phase 6 step 1.
**Périmètre :** crate `pomone-ui` + `pomone-app` (probablement un nouvel
écran de paramètres et l'extension de la couche config).

**Contexte.** Aujourd'hui la sélection du backend et du chemin de la
base se fait uniquement par éditer `~/.config/pomone/config.toml` à la
main. Pour une vraie ergonomie, l'utilisateur doit pouvoir :

1. Choisir SQLite ou MariaDB depuis l'UI.
2. Pour SQLite, choisir le fichier (création nouvelle ou ouverture
   existante) via une boîte de dialogue.
3. Pour MariaDB, saisir l'URL de connexion (host, port, user, password,
   database) et tester la connexion avant validation.
4. Voir clairement quelle base est actuellement active.
5. Basculer de base sans avoir à éditer un fichier ni redémarrer
   l'application si possible.

**Périmètre attendu.**

- Un écran/dialogue **Paramètres → Base de données** dans l'UI.
- Un sélecteur radio (ou onglets) entre SQLite et MariaDB.
- Pour SQLite : champ de chemin avec un bouton **Parcourir…** qui
  ouvre une `FileDialog` native (à créer ou ouvrir un fichier).
- Pour MariaDB : champs host, port, user, password, database, plus un
  bouton **Tester la connexion** qui appelle `MariaDbRepository::connect`
  et affiche le résultat sans persister.
- Bouton **Enregistrer** qui :
  1. valide les champs,
  2. tente de se connecter au nouveau backend,
  3. si ça réussit, met à jour `AppConfig` et le persiste via
     `AppConfig::save_default()`,
  4. remplace le `Repository` actif dans `App` (et relance les
     migrations + seed).
- Bouton **Annuler** qui ferme la boîte sans rien changer.
- Indicateur visible (dans l'en-tête ou la barre d'état) du backend
  courant : `SQLite (~/.local/share/pomone/pomone.sqlite)` ou
  `MariaDB (mysql://user@host:3306/db)`.

**Critères d'acceptation.**

- À partir d'une installation neuve, on peut depuis l'UI seule créer
  un nouveau fichier SQLite ailleurs que dans le dossier par défaut, ou
  basculer sur une instance MariaDB locale (ou distante), sans jamais
  toucher au fichier TOML à la main.
- L'erreur de connexion (mauvais identifiants MariaDB, chemin SQLite
  invalide…) est affichée dans la boîte avant la validation, pas après.
- Le mot de passe MariaDB n'est pas stocké en clair visible dans l'UI
  après validation (au moins remplacé par `••••••` dans les écrans de
  lecture). Le stockage en clair dans `config.toml` est un compromis
  acceptable pour la v1, mais à documenter.
- Tests unitaires sur la logique de validation des entrées et sur la
  bascule de backend (sur SQLite, simple à tester ; pour MariaDB, ce
  sera un test d'intégration `#[ignore]` comme les autres).

**Notes d'implémentation.**

- L'`App` actuelle contient un `Box<dyn Repository>`. Pour permettre
  un swap à chaud, soit on l'enveloppe dans un `RefCell`/`Mutex`, soit
  on rebuild un `App` complet et on remplace l'instance dans la cellule
  partagée. La seconde approche est plus simple à raisonner (et l'UI
  peut transitoirement afficher un écran "rechargement…").
- Pour le file picker SQLite, `rfd` (Rust File Dialogs) est le crate
  standard ; il marche en natif sur Linux/macOS/Windows et n'a pas de
  conflit avec Slint.
- Pour la prise en compte propre du test de connexion MariaDB,
  ajouter une fonction `MariaDbRepository::probe(url)` qui tente la
  connexion sans démarrer le pool plein.

**Dépendances entre CR.** Mieux vaut traiter CR-1 avant ou en parallèle
de CR-2 — le nouvel écran "Paramètres" héritera naturellement du
`Theme` mis en place.
