# QRop vs Pomone — analyse comparative des écarts

> Comparaison des fonctionnalités, du graphisme et de l'organisation des écrans
> entre **QRop** (`../qrop-main`, C++/Qt Quick) et **Pomone** (Rust/Slint), à la
> date du 2026-07-08 (Pomone `main` = `0bf4c69`).
>
> **Périmètre exclu de la comparaison** (par nature en faveur de Pomone) : les
> cultures pérennes/pluriannuelles et l'agroforesterie, que QRop ne modélise pas.
> Tout le reste est comparé « à parité de domaine annuel ».

---

## 1. Verdict d'ensemble

Pomone couvre aujourd'hui **le socle CRUD** de QRop (plantations, tâches,
lieux, cultures/variétés, familles) avec une identité graphique propre et deux
apports structurels que QRop n'a pas (**pérennes/agroforesterie** et **backend
SQLite _ou_ MariaDB avec migration à chaud**). En revanche, Pomone est
**nettement un sous-ensemble** de QRop sur trois axes majeurs :

1. **L'économie et la planification agronomique** — QRop chiffre (rendement,
   prix, revenu), planifie les **successions**, calcule densité/semences, et
   détecte les **conflits de rotation**. Pomone n'a rien de tout cela.
2. **Les écrans « métier » de sortie** — QRop a des pages dédiées **Récoltes**,
   **Liste de semis/plants**, **Graphiques/stats**, des **modèles de tâches**,
   des **notes + photos**, et **5 exports PDF + CSV**. Pomone n'a aucun de ces
   écrans ni aucun export.
3. **La densité d'interaction** — QRop est **centré tableur** : recherche sur
   chaque page, multi-sélection, barres d'outils riches, glisser-déposer,
   side-sheets contextuelles. Pomone est **centré cartes + un tableau**, sans
   recherche nulle part, avec des actions plus limitées.

En clair : ce qui existe dans Pomone est plus moderne et plus propre visuellement,
mais **la surface fonctionnelle représente grosso modo la moitié** de QRop côté
annuel.

---

## 2. Organisation des écrans (navigation)

| | **QRop** | **Pomone** |
|---|---|---|
| Structure | Drawer latéral Material (rail persistant > 1200px) | Rail latéral fixe 240px, **groupé** en sections |
| Groupes | Liste plate de 7 entrées + 2 boutons base de données en bas | **Planification / Catalogue / Système** + Accueil isolé |
| Entrées | Plantings · Tasks · Crop Map · Harvests · Seed list · Charts · Settings | Accueil · Plantations · Tâches · **Agenda** · Carte · Cultures · Lieux · **Strates** · Familles · Paramètres |
| Bases de données | **Deux bases SQLite simultanées** (Main/Secondary, New/Open/Export/Close par slot) | **Une** base, choix backend SQLite **ou** MariaDB + migration live |
| Raccourcis | Ctrl+1..6/0, F11 plein écran, Ctrl+Q | Ctrl+1..9, F1 manuel |

**Différences d'organisation notables :**

- Pomone **regroupe** la navigation (mieux rangé) et ajoute **Agenda** (liste
  plate des tâches) et **Strates** (strates de végétation, concept nouveau).
- QRop expose **Récoltes**, **Liste de semis** et **Graphiques** comme pages de
  premier niveau ; Pomone **n'a aucune de ces trois pages**.
- Le **catalogue** de QRop est concentré dans _Settings > Lists_ (familles/
  cultures/variétés, mots-clés, semenciers, types de tâches, unités) ; Pomone
  l'**éclate** en pages de premier niveau (Cultures, Lieux, Strates, Familles)
  mais **avec beaucoup moins d'entités** (voir §5).
- QRop gère **deux bases ouvertes en parallèle** (usage : comparer deux plans,
  archiver une saison) — absent de Pomone, qui a fait un autre choix (multi-SGBD).

---

## 3. Comparaison fonctionnelle page par page (domaine annuel)

### 3.1 Plantations — l'écart le plus important

| Capacité | QRop | Pomone |
|---|---|---|
| Table triable, colonnes masquables | ✅ 17 colonnes, clic-tri, popup visibilité persistée | ⚠️ ~7 colonnes, tri sur 5, **pas de masquage** |
| Colonnes économiques (Rendement moy., Prix moy., **Revenu**) | ✅ | ❌ **aucune** |
| Colonnes agronomiques (DTT, DTM, Fenêtre récolte, Rangs, Espacement, Longueur) | ✅ | ❌ (seulement Surface, Plants) |
| Gantt de saison | ✅ (barres **glissables** pour re-planifier) | ✅ (lecture seule, clic → détail) |
| Recherche + filtre [Tous/Serre/Champ] | ✅ | ❌ **pas de recherche** |
| Multi-sélection (shift-clic, tout sélectionner) | ✅ | ❌ |
| **Successions** (N plantations + semaines entre) | ✅ | ❌ |
| **Densité / semences** (graines/trou, extra %, par gramme, besoin calculé) | ✅ | ❌ |
| Méthode d'établissement (semis direct / repiqué / acheté) | ✅ | ✅ (issue #94) |
| Durées (jours à maturité, durée serre, fenêtre récolte) éditables | ✅ (mode calcul commutable) | ⚠️ portées par la **variété**, pas éditables par plantation |
| Mots-clés/tags sur plantation | ✅ (`ChoiceChip`) | ❌ |
| Actions : Dupliquer, **Dupliquer vers l'année suivante**, Terminer (avec motif) | ✅ | ❌ (seulement changer statut / supprimer) |
| Saisie rapide (« Ne pas fermer ») | ✅ | ❌ |
| Import/Export CSV du plan | ✅ | ❌ |
| Annulation via snackbar (rollback) | ✅ | ❌ |
| Panneau graphique occupation serre/champ | ✅ (relatif/absolu, filtrable) | ⚠️ **oui mais sur l'Accueil**, pas de filtre |

**Le formulaire de plantation de QRop** collecte ~25 champs (variété, serre,
beds/longueur, espacement, rangs, densité, successions, radios de type, durées,
dates serre/champ/récolte, choix de lieux inline, pertes serre, semences,
**unité + rendement/bed + prix**, mots-clés). Celui de Pomone en collecte ~6
(variété, lieu, strate, méthode + date(s), surface, nombre). **C'est le cœur de
l'écart** : Pomone plante, QRop planifie économiquement.

### 3.2 Tâches

| Capacité | QRop | Pomone |
|---|---|---|
| Vue | Liste hebdo groupée **par type** | **Calendrier mensuel** (grille) |
| Glisser-déposer re-planification | ⚠️ non (délai ±1 semaine par boutons) | ✅ (glisser une pastille sur un jour) |
| Filtres | Fait / Dû / En retard | Puce par catégorie (8) + jalons de culture |
| Recherche | ✅ « Search Tasks » | ❌ |
| Jalons de cycle de culture affichés | ❌ | ✅ (semis/repiquage/récolte/… en pastilles) |
| **Modèles de tâches** (templates, application en masse, cascade) | ✅ (`TemplatePane`) | ❌ |
| Complétion par appui long + date | ✅ | ⚠️ case « terminé » dans le formulaire |
| Type → **Méthode → Outil** sur la tâche | ✅ (combos, ajout inline) | ❌ (champs modélisés en base, **absents de l'UI**) |
| Récurrence | ✅ | ✅ (intervalle/unité/fin) |
| Impression du calendrier | ✅ PDF | ❌ |

### 3.3 Récoltes — **page absente de Pomone**

QRop : page dédiée (colonnes Plantation, Lieux, **Quantité + unité**, Date,
Heure de main-d'œuvre), dialogue d'ajout avec sélecteur multi-plantations et
**répartition de la quantité**, recherche, impression PDF.

Pomone : **aucune page Récoltes**. Les récoltes n'existent que dans le **détail
d'une plantation pérenne** (tableau année/attendu/réel/écart/notes) — donc rien
pour les cultures **annuelles**, qui sont pourtant le cœur du domaine comparé.

### 3.4 Liste de semis / plants — **page absente de Pomone**

QRop : onglets **Semis / Plants**, colonnes Culture/Variété/**Semencier**/Nombre/
Quantité (g/kg auto), regroupement Année/Trimestre/Mois, export CSV + PDF. C'est
l'écran qui produit le **bon de commande** de graines. **Rien d'équivalent** dans
Pomone.

### 3.5 Graphiques / statistiques — **page absente de Pomone**

QRop : `ChartsPage` = 4 cartes KPI (**Revenu estimé**, nb beds champ, nb beds
serre, nb cultivars) + table par culture (longueurs/rendements/**revenus** champ
& serre). Pomone : **aucun tableau de bord analytique**. Les seuls visuels sont
le Gantt de saison et la courbe d'occupation des beds (Accueil). Aucune notion de
revenu nulle part.

### 3.6 Carte des cultures

| Capacité | QRop | Pomone |
|---|---|---|
| Arbre de lieux hiérarchique + Gantt par ligne | ✅ | ⚠️ une ligne par lieu (pas d'arbre imbriqué dans la carte) |
| **Glisser-déposer** plantation → lieu | ✅ (Move/Copy, auto-expand, blocage conflits) | ❌ (clic sur barre + sélecteur « Déplacer vers… ») |
| Panneau des plantations **non assignées** (sources de drag) | ✅ | ❌ |
| Bascule couleur Culture/Famille | ✅ | ⚠️ couleur famille uniquement |
| **Détection de conflits de rotation** | ✅ (intervalle famille) | ❌ |
| Diviser une plantation entre lieux | ⚠️ via réassignation | ✅ (modal « Diviser » 2 parts) |
| Impression carte PDF | ✅ | ❌ |

### 3.7 Notes & photos — **absent de Pomone**

QRop : side-sheet de notes par plantation, texte libre + **pièces jointes photo**
(jpg/png/gif, visionneuse `PhotoPane`). Pomone : des champs `notes` texte
existent sur plusieurs entités, mais **aucune UI de prise de notes** ni **aucune
pièce jointe** ni recherche plein-texte.

---

## 4. Catalogues / entités de référence

| Entité de référence QRop | Champs | Présent dans Pomone ? |
|---|---|---|
| Famille | nom, couleur, **intervalle de rotation (0–9 ans)** | ⚠️ nom + couleur ; **intervalle de rotation absent** |
| Culture | nom, couleur (par culture), famille | ⚠️ oui, mais **couleur portée par la famille**, pas par culture |
| Variété | nom, **semencier**, défaut par culture | ⚠️ nom + description ; **pas de semencier, pas de défaut** |
| **Mots-clés / tags** | nom | ❌ **absent** |
| **Semenciers** | nom, défaut | ❌ **absent** |
| **Unités de mesure** | abréviation, nom complet | ❌ **absent** (surfaces en m², poids en kg codés en dur) |
| Type de tâche | nom, couleur | ✅ |
| **Méthode de tâche** | nom (sous type) | ❌ (modélisé en `pomone-domain`/`pomone-db`, **jamais exposé**) |
| **Outil / matériel (implement)** | nom (sous méthode) | ❌ (idem : type domaine présent, **UI absente, seed absent**) |
| Strate de végétation | — | ✅ **nouveau dans Pomone** (n'existe pas dans QRop) |

**Point d'attention technique** : `TaskMethod` et `TaskImplement` sont
**entièrement modélisés** dans `pomone-domain` (`task_type.rs`) et `pomone-db`
(traits `TaskMethodRepo`/`TaskImplementRepo`), et `Task` porte
`task_method_id`/`implement_id` — mais **aucune page, aucun champ de formulaire,
aucune donnée de seed** ne les utilise. C'est du code « mort en UI » : la
hiérarchie Type→Méthode→Outil de QRop est amorcée mais non finie.

**Préférences applicatives** : QRop a un écran de réglages riche (type de date,
langue, largeur de bed/passe-pied, ~10 interrupteurs d'affichage). Pomone n'a que
le choix de backend (aucune préférence d'affichage utilisateur ; même le **mode
sombre** existe en tokens mais **n'est pas commutable** dans l'UI).

---

## 5. Graphisme et style visuel

| | **QRop** | **Pomone** |
|---|---|---|
| Langage | Material Design | Identité propre « verger/jardin » |
| Couleurs | Primaire Teal-500, accent Blue-600, fond Grey-100 | Vert feuille #3C6E47, terracotta #B85C38, fond crème #FBF7F0 |
| Mode sombre | ❌ | ✅ en tokens (⚠️ **non commutable** dans l'UI) |
| Typo | Roboto / Roboto Condensed / Eczar ; **14px** dominant | Material Icons ; **18px** (moins dense) |
| Densité | Très dense (lignes 42px, table partout) | Moyenne (lignes 46px, cartes + un tableau) |
| Codage couleur | Manuel **par culture ET par famille** (bascule) | **Par famille** (pastille 2 lettres, issue #97) |
| Icônes | Material Icons (glyphes) | Material Icons (mêmes glyphes) |
| Avatars culture | Disque couleur + 2 initiales (`CropIcon`/`TextDisk`) | Pastille couleur + 2 initiales (équivalent) |
| Motif dominant | **Tables denses** triables + barres d'outils + side-sheets | **Cartes** + un tableau + master-detail |
| Chips | 3 variantes (choix/statique/removable) | Puces de filtre (calendrier) |

**Synthèse visuelle** : Pomone est plus **chaleureux, aéré et cohérent**
(palette maison, dark mode prévu, composants réutilisables soignés). QRop est
plus **dense et « tableur »**, taillé pour saisir/scanner beaucoup de lignes
rapidement. Ce sont deux partis pris opposés : Pomone privilégie la lisibilité,
QRop la densité d'information. L'utilisateur qui vient de QRop percevra Pomone
comme **plus joli mais moins « productif à la ligne »** (moins de colonnes, pas
de recherche, moins d'actions groupées).

---

## 6. Impression / export

| | QRop | Pomone |
|---|---|---|
| Export PDF | **5 rapports** : plan de culture (4 variantes), calendrier de tâches, carte des cultures, récoltes, liste de semis/plants | ❌ **aucun** |
| Export CSV | Plan de culture (aller-retour), liste de semis | ❌ **aucun** |
| Impression | `QPdfWriter` + moteur de table paginé (`tableprinter.cpp`) | ❌ (seul PDF : le **manuel** LaTeX ouvert par F1) |

C'est un pan entier absent de Pomone : **aucune sortie imprimable/exportable** des
données métier.

---

## 7. Tableau de synthèse des écarts (priorisé)

Légende : 🔴 absent & structurant · 🟠 absent mais contournable · 🟡 partiel · 🟢 présent (parité ou mieux)

| Domaine | Écart | Gravité |
|---|---|---|
| Économie (rendement, prix, **revenu**) | Aucune notion dans Pomone | 🔴 |
| Page **Récoltes** (annuelles) | Absente | 🔴 |
| **Successions** de plantation | Absentes | 🔴 |
| **Export/impression** PDF & CSV | Absents | 🔴 |
| Page **Liste de semis/plants** (bon de commande) | Absente | 🔴 |
| Page **Graphiques/stats** (KPI, revenu) | Absente | 🟠 |
| **Modèles de tâches** | Absents | 🟠 |
| **Recherche** (toutes pages) | Absente partout | 🟠 |
| Densité/semences dans le formulaire | Absentes | 🟠 |
| **Notes + photos** | UI absente | 🟠 |
| Catalogues **mots-clés / semenciers / unités** | Absents | 🟠 |
| Hiérarchie **Type→Méthode→Outil** de tâche | Modélisée, non exposée | 🟠 |
| **Intervalle de rotation** + détection de conflits | Absents | 🟠 |
| Glisser-déposer sur la **carte** | Remplacé par clic+sélecteur | 🟡 |
| Multi-sélection / actions groupées (dupliquer, terminer…) | Absentes | 🟡 |
| Colonnes riches + masquables (plantations) | Table réduite | 🟡 |
| Couleur **par culture** (pas seulement famille) | Partiel | 🟡 |
| **Deux bases** ouvertes en parallèle | Choix différent (multi-SGBD) | 🟡 |
| Mode sombre commutable | Tokens présents, non branché | 🟡 |
| Navigation groupée / rangée | — | 🟢 (mieux) |
| Backend SQLite **ou** MariaDB + migration | — | 🟢 (unique à Pomone) |
| Glisser-déposer sur le **calendrier** de tâches | — | 🟢 (unique à Pomone) |
| Cultures **pérennes / agroforesterie** | — | 🟢 (hors périmètre, unique à Pomone) |
| **Strates** de végétation | — | 🟢 (unique à Pomone) |

---

## 8. Lecture recommandée

Si l'objectif est de **rapprocher Pomone de QRop** à parité annuelle, l'ordre de
valeur décroissante est :

1. **L'économie** (unité + rendement/bed + prix → revenu) — débloque à la fois
   la valeur des colonnes de la table, la page Graphiques et l'intérêt métier.
2. **La page Récoltes** pour les annuelles (+ catalogue **unités**).
3. **Les successions** dans le formulaire de plantation.
4. **L'export PDF/CSV** (au moins plan de culture + liste de semis).
5. **La recherche** transversale (bon rapport valeur/effort, elle manque partout).
6. **Finir** la hiérarchie Type→Méthode→Outil déjà modélisée en base (dette
   latente à solder ou à retirer).
7. **Modèles de tâches**, **notes+photos**, **mots-clés/semenciers**, **rotation**
   ensuite selon les priorités agronomiques.

Les points 🟢 (backend multi-SGBD, calendrier drag-drop, pérennes, strates,
navigation rangée, palette) sont les **différenciateurs** à préserver : ils ne
sont pas des régressions mais des choix qui éloignent volontairement Pomone de
QRop.

---

*Sources : `../qrop-main/desktop/qml/*.qml`, `../qrop-main/core/print.cpp`,
`core/tableprinter.cpp` ; `crates/pomone-ui/ui/*.slint`,
`crates/pomone-app/src/*_view.rs`, `crates/pomone-domain/src/`.*
