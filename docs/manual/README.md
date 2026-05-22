# Manuel utilisateur Pomone

Sources LaTeX du manuel utilisateur. Document destiné aux utilisateurs et utilisatrices finales (maraîchères, jardiniers, agroforestières…), distinct de la documentation technique (`doc-latex/pomone.pdf`).

## Compilation locale

Le manuel utilise `xelatex` (fontspec + police Lato + Inconsolata).
Sur Debian/Ubuntu :

```sh
sudo apt install texlive-xetex texlive-fonts-extra texlive-lang-french
latexmk -xelatex manuel.tex
```

ou directement :

```sh
xelatex manuel.tex && xelatex manuel.tex   # 2 passes pour la TOC
```

## Build CI

Le workflow `.github/workflows/build-manual-pdf.yml` compile `manuel.tex` à chaque push sur `main` qui touche ce dossier, et publie le PDF en release rolling [`docs-latest`](https://github.com/guycorbaz/pomone/releases/tag/docs-latest) (aux côtés de `pomone.pdf`).

## Structure

- `manuel.tex` — document principal (préambule, styles, includes).
- `sections/` — un fichier `.tex` par chapitre :
  - `introduction.tex`
  - `installation.tex`
  - `premiers_pas.tex`
  - `plantations.tex`
  - `cultures.tex`
  - `lieux.tex`
  - `calendrier.tex`
  - `recoltes.tex`
  - `parametres.tex`
  - `annexes.tex`

## Convention

Chaque PR qui ajoute ou modifie une fonctionnalité utilisateur visible doit mettre à jour la section correspondante du manuel. Les marqueurs `\begin{todobox}…\end{todobox}` signalent les points encore à rédiger.
