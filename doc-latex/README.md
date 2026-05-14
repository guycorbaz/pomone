# Documentation LaTeX

Sources de la documentation technique complète de Pomone.

## Compilation locale

```sh
latexmk -pdf pomone.tex
```

ou directement :

```sh
pdflatex pomone.tex && pdflatex pomone.tex   # 2 passes pour la TOC
```

## Build CI

Le workflow `.github/workflows/build-doc-pdf.yml` compile `pomone.tex` à chaque push sur `main` qui touche ce dossier, et publie le PDF en release rolling [`docs-latest`](https://github.com/guycorbaz/pomone/releases/tag/docs-latest).

## Structure

- `pomone.tex` — document principal (préambule, styles, includes).
- `sections/` — chaque section dans un fichier dédié pour faciliter les diffs.
