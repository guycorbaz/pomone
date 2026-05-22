---
title: Feuille de route
nav_order: 2
description: "Plan d'avancement de Pomone par phases, du bootstrap à la parité Qrop v1.0.0."
---

# Feuille de route
{: .no_toc }

État d'avancement réel du projet, mis à jour à chaque livraison.
{: .fs-5 .fw-300 }

## Phases socle (terminées)

- [x] **Phase 0** — bootstrap (workspace, CI 3 OS, gate de couverture ≥ 80 %)
- [x] **Phase 1** — `pomone-domain` : modèle métier complet, tests proptest
- [x] **Phase 2** — `pomone-db` SQLite : trait `Repository`, schéma + migrations, codec Decimal-as-TEXT
- [x] **Phase 3** — backend MariaDB + tests d'intégration paramétrés sur les deux backends
- [x] **Phase 4** — `pomone-app` : services, use cases, `AppError` unifié
- [x] **Phase 5** — i18n (fr + en) avec [Fluent](https://projectfluent.org/)
- [x] **Phase 6** — `pomone-ui` Slint : tous les écrans principaux
  - [x] sidebar de navigation persistante (raccourcis `Ctrl+1..7`, `F1` pour le manuel)
  - [x] écran Plantations (liste + Gantt + formulaire annuel/pluriannuel)
  - [x] écran Cultures + Variétés (master-detail)
  - [x] écran Lieux (hiérarchie ferme → parcelle → planche)
  - [x] écran Calendrier mensuel
  - [x] écran Strates (CRUD)
  - [x] écran détail de plantation
  - [x] écran Paramètres avec bascule SQLite ↔ MariaDB et migration des données
- [x] **Phase 7** — fonctionnalités pérennes
  - [x] suivi `YearlyHarvest` (saisie année par année, variance attendu/réel)
  - [x] formulaire pluriannuel (date d'établissement, arrachage prévu)
- [x] **Phase 8 Linux** — packaging `.deb` + AppImage via [cargo-packager](https://github.com/crabnebula-dev/cargo-packager), avec icône, dépendances système déclarées et manuel utilisateur PDF embarqué
- [x] **Phase 9** — polish (validation des formulaires, navigation clavier, transitions, fallback i18n)

## Phase 10 — Parité Qrop pour v1.0.0 (en cours)

Objectif explicite : v1.0.0 au moins équivalente à Qrop dans sa version actuelle, avec un meilleur contrôle qualité (CI 3 OS + ≥ 80 % couverture, type-safety Rust, tests systématiques). Découpé en sous-livraisons :

### Manuel + structure (livré)
- [x] Manuel utilisateur LaTeX (`docs/manual/`) compilé en PDF par CI
- [x] PDF embarqué dans le `.deb` et l'AppImage à `/usr/share/doc/pomone/manuel.pdf`
- [x] Bouton **Aide** dans la sidebar + raccourcis `F1` / `Ctrl+8`
- [x] Règle "un fichier par fonctionnalité" appliquée (refactor HomePage extrait dans `home.slint`)

### Vue Gantt (livré)
- [x] Composants Slint `Timeline` + `Timegraph` réutilisables (12 mois, barres serre/champ/récolte, ligne du jour)
- [x] Gantt branché sur l'écran Plantations (au-dessus de la liste) et sur l'Accueil (vue compacte)

### Tâches et opérations (en cours)
- [x] Backend Tasks : `Task`, `TaskType`, `TaskMethod`, `TaskImplement`, repository pour les deux SGBD
- [ ] Auto-génération des tâches depuis les plantations (semis, repiquage, récolte) — **PR E**
- [ ] Écran TaskCalendar mensuel + complétion d'une tâche en un clic — **PR E**
- [ ] Superposition des marqueurs de tâches sur la vue Gantt — **PR F**

### À venir (priorité produit)
- [ ] **Crop Map** : occupation des planches sur axe temporel, drag-and-drop
- [ ] **Exports** PDF + CSV (plans de culture, listes de semences, calendrier)
- [ ] **Notes** texte et photos liées aux plantations / tâches
- [ ] **Rotations** : historique par planche + alertes conflits par famille
- [ ] **Charts** : distribution mètres-de-planche par culture, revenus
- [ ] **Templates** de tâches réutilisables
- [ ] **Dépenses / coûts** : taux horaire × temps de travail
- [ ] **Mots-clés colorés** sur plantations + succession numbering
- [ ] **Multi-base simultanée** (comparer deux plans)
- [ ] **Issue #29** — extensions spécifiques aux grandes cultures (échelle ha, calendriers céréales, tâches mécanisées)

### Post-v1.0.0
- [ ] Packaging Windows (`.msi`) et macOS (`.dmg`)
- [ ] Toggle dark mode exposé dans l'UI
- [ ] Notifications/rappels pour tâches
- [ ] Multi-utilisateurs et rôles
