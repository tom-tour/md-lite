# MD Lite

[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/tom_t)
[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3+-blue.svg)](LICENSE)

A very light Markdown reader and editor for Windows. Double-click a `.md` file and it opens: rendered preview, raw text, or both side by side.

*[Version française plus bas](#français)*

## Features

- **Three views**: raw text, text + preview, preview only. Files open in preview mode by default (configurable).
- **Tabs**: open several files in one window. Opening a file from Explorer adds a tab to the running window (or a new window, your choice). The last session can be restored.
- **Toolbar for every Markdown element**: headings, bold, italic, strikethrough, highlight, inline code, lists, task lists, quotes, links, images, tables (size picker), code blocks, footnotes, horizontal rules. Clicking again removes the formatting.
- **Rendering**: GitHub-style Markdown, tables, task lists (clickable in the preview), footnotes, syntax highlighting for about 20 languages, common README HTML (`<details>`, `<p align="center">`, `<img width>`, `<kbd>`…).
- **Images relative to the file** (`![](images/photo.png)`), paths from the git repository root (`/docs/img.png`) and absolute paths.
- **Links to other `.md` files** open in a new tab.
- **Safe**: scripts and dangerous HTML in documents are removed, and a content security policy blocks anything left.
- **Respects your files**: keeps the original encoding (UTF-8 with or without BOM, UTF-16, Windows-1252) and line endings, writes atomically, and detects changes made by other programs.
- Find and replace, zoom, print / export to PDF, light / dark / system theme, spell checker, all settings in one place.
- **Small**: a single executable under 1 MB. It uses the WebView2 runtime that ships with Windows 10/11.

## Installation

1. Download `md-lite.exe` (or build it, see below) and put it in a folder of your choice.
2. Run it once. A banner offers to make MD Lite the default app for `.md` files: click **Set as default**, then pick MD Lite in the Windows dialog.
   You can also run `md-lite.exe --install` to register it, and `md-lite.exe --uninstall` to remove the registration.

If you move the executable, the file association updates itself the next time you start it.

## Keyboard shortcuts

| Shortcut | Action |
| :-- | :-- |
| Ctrl+B / I / K / E | Bold / italic / link / code |
| Ctrl+F / Ctrl+H | Find / replace |
| Ctrl+S / Ctrl+Shift+S / Ctrl+O | Save / save as / open |
| Ctrl+T / Ctrl+W | New tab / close tab |
| Ctrl+Tab, Ctrl+1…9 | Switch tab |
| Ctrl+N | New window |
| Ctrl+, | Settings |
| Alt+1 / 2 / 3 | Text / both / preview |
| Ctrl+wheel, Ctrl+0 | Zoom |
| Ctrl+P | Print or export to PDF |

## Building

Requirements: Windows, [Rust](https://rustup.rs) with the MSVC toolchain, and the Visual Studio Build Tools.

```bat
build.cmd
```

This builds the release executable and copies `md-lite.exe` next to the script. The editor itself is `md-lite.html` (embedded in the executable at build time). It also works on its own in a browser, with fewer features.

## Support

If MD Lite is useful to you, you can support its development on [Ko-fi](https://ko-fi.com/tom_t).

## License

[GPL-3.0-or-later](LICENSE)

---

## Français

Un lecteur et éditeur Markdown très léger pour Windows. Double-clique sur un fichier `.md` et il s'ouvre : aperçu mis en forme, texte brut, ou les deux côte à côte.

### Fonctionnalités

- **Trois vues** : texte brut, texte + aperçu, aperçu seul. Les fichiers s'ouvrent en aperçu par défaut (réglable).
- **Onglets** : plusieurs fichiers dans une seule fenêtre. Un fichier ouvert depuis l'Explorateur s'ajoute en onglet à la fenêtre déjà ouverte (ou dans une nouvelle fenêtre, au choix). La dernière session peut être rouverte.
- **Une barre d'outils pour toutes les balises Markdown** : titres, gras, italique, barré, surligné, code, listes, listes de tâches, citations, liens, images, tableaux (choix de la taille), blocs de code, notes de bas de page, séparateurs. Un second clic enlève la mise en forme.
- **Rendu** : Markdown façon GitHub, tableaux, cases à cocher (cliquables dans l'aperçu), notes de bas de page, coloration du code pour une vingtaine de langages, HTML courant des README (`<details>`, `<p align="center">`, `<img width>`, `<kbd>`…).
- **Images relatives au fichier** (`![](images/photo.png)`), chemins depuis la racine du dépôt git (`/docs/img.png`) et chemins absolus.
- **Les liens vers d'autres `.md`** s'ouvrent dans un nouvel onglet.
- **Sûr** : les scripts et le HTML dangereux des documents sont retirés, et une politique de sécurité bloque le reste.
- **Respecte tes fichiers** : conserve l'encodage d'origine (UTF-8 avec ou sans BOM, UTF-16, Windows-1252) et les fins de ligne, écrit sans risque de fichier à moitié enregistré, détecte les modifications faites par d'autres programmes.
- Rechercher et remplacer, zoom, impression et export PDF, thème clair / sombre / système, correcteur orthographique, tous les réglages au même endroit.
- **Léger** : un seul exécutable de moins de 1 Mo. Il utilise le runtime WebView2 fourni avec Windows 10/11.

### Installation

1. Télécharge `md-lite.exe` (ou compile-le, voir plus bas) et place-le dans le dossier de ton choix.
2. Lance-le une fois. Un bandeau propose d'en faire l'application par défaut pour les `.md` : clique sur **Définir par défaut**, puis choisis MD Lite dans la fenêtre de Windows.
   Tu peux aussi lancer `md-lite.exe --install` pour l'enregistrer, et `md-lite.exe --uninstall` pour retirer l'enregistrement.

Si tu déplaces l'exécutable, l'association se met à jour toute seule au lancement suivant.

### Raccourcis

| Raccourci | Action |
| :-- | :-- |
| Ctrl+B / I / K / E | Gras / italique / lien / code |
| Ctrl+F / Ctrl+H | Rechercher / remplacer |
| Ctrl+S / Ctrl+Maj+S / Ctrl+O | Enregistrer / enregistrer sous / ouvrir |
| Ctrl+T / Ctrl+W | Nouvel onglet / fermer l'onglet |
| Ctrl+Tab, Ctrl+1…9 | Changer d'onglet |
| Ctrl+N | Nouvelle fenêtre |
| Ctrl+, | Paramètres |
| Alt+1 / 2 / 3 | Texte / les deux / aperçu |
| Ctrl+molette, Ctrl+0 | Zoom |
| Ctrl+P | Imprimer ou exporter en PDF |

### Compilation

Prérequis : Windows, [Rust](https://rustup.rs) avec la chaîne MSVC, et les Build Tools de Visual Studio.

```bat
build.cmd
```

Le script compile la version optimisée et copie `md-lite.exe` à côté de lui. L'éditeur lui-même est `md-lite.html` (intégré à l'exécutable à la compilation). Il fonctionne aussi seul dans un navigateur, avec moins de fonctions.

### Soutien

Si MD Lite te rend service, tu peux soutenir son développement sur [Ko-fi](https://ko-fi.com/tom_t).

### Licence

[GPL-3.0-or-later](LICENSE)
