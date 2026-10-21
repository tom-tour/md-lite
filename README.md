<img src="icons/icon.png" width="64" alt="" align="right">

# MD Lite

[![Download for Windows](https://img.shields.io/badge/Download-Windows%20(.exe)-0969DA?logo=windows&logoColor=white)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite.exe)
[![Download for Linux](https://img.shields.io/badge/Download-Linux%20(.tar.gz)-0969DA?logo=linux&logoColor=white)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite-linux-x86_64.tar.gz)
[![Latest release](https://img.shields.io/github/v/release/tom-tour/md-lite)](https://github.com/tom-tour/md-lite/releases/latest)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/tom_t)
[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3+-blue.svg)](LICENSE)

A very light Markdown reader and editor for Windows and Linux. Double-click a `.md` file and it opens: rendered preview, raw text, or both side by side.

**⬇ Download the latest version: [Windows (md-lite.exe)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite.exe) · [Linux (md-lite-linux-x86_64.tar.gz)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite-linux-x86_64.tar.gz)** (about 1 MB, no installation)

*[Version française plus bas](#français)*

## Features

- **Three views**: raw text, text + preview, preview only. Files open in preview mode by default (configurable).
- **Tabs**: open several files in one window. Opening a file from Explorer or your file manager adds a tab to the running window (or a new window, your choice). The last session can be restored.
- **Toolbar for every Markdown element**: headings, bold, italic, strikethrough, highlight, inline code, lists, task lists, quotes, links, images, tables (size picker), code blocks, footnotes, horizontal rules. Clicking again removes the formatting.
- **Rendering**: GitHub-style Markdown, tables, task lists (clickable in the preview), footnotes, syntax highlighting for about 20 languages, common README HTML (`<details>`, `<p align="center">`, `<img width>`, `<kbd>`…).
- **Images relative to the file** (`![](images/photo.png)`), paths from the git repository root (`/docs/img.png`) and absolute paths.
- **Links to other `.md` files** open in a new tab.
- **Safe**: scripts and dangerous HTML in documents are removed, and a content security policy blocks anything left.
- **Respects your files**: keeps the original encoding (UTF-8 with or without BOM, UTF-16, Windows-1252) and line endings, writes atomically, and detects changes made by other programs.
- Find and replace, zoom, print / export to PDF, light / dark / system theme, spell checker, all settings in one place.
- **Small**: a single executable of about 1 MB. It uses the web engine already on your system: WebView2 on Windows 10/11, WebKitGTK on Linux.

## Installation

### Windows

1. [Download `md-lite.exe`](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite.exe) (or build it, see below) and put it in a folder of your choice.
2. Run it once. A banner offers to make MD Lite the default app for `.md` files: click **Définir par défaut** (set as default), then pick MD Lite in the Windows dialog.
   You can also run `md-lite.exe --install` to register it, and `md-lite.exe --uninstall` to remove the registration.

If you move the executable, the file association updates itself the next time you start it.

The executable is not code-signed, so Windows SmartScreen may warn you the first time: click **More info**, then **Run anyway**.

### Linux

1. [Download `md-lite-linux-x86_64.tar.gz`](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite-linux-x86_64.tar.gz), extract it and put `md-lite` in a folder of your choice (for example `~/.local/bin`).
2. MD Lite needs WebKitGTK 4.1 (version 2.40 or later), already present on most desktops. Otherwise:
   - Debian, Ubuntu, Mint: `sudo apt install libwebkit2gtk-4.1-0`
   - Fedora: `sudo dnf install webkit2gtk4.1`
   - Arch: `sudo pacman -S webkit2gtk-4.1`
3. Run `./md-lite` once. A banner offers to make MD Lite the default app for `.md` files: click **Définir par défaut** (set as default). MD Lite also appears in your applications menu.
   You can also run `./md-lite --install` to add it to the menu, and `./md-lite --uninstall` to remove it.

Works on Ubuntu 22.04 or later and other distributions of the same age. If you move the executable, the menu entry updates itself the next time you start it.

The interface is in French for now.

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

Requirements: [Rust](https://rustup.rs).

- **Windows**: the MSVC toolchain and the Visual Studio Build Tools. Run `build.cmd`: it builds the release executable and copies `md-lite.exe` next to the script.
- **Linux**: the development packages of WebKitGTK and GTK (Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev`), then `cargo build --release`. The executable is `target/release/md-lite`.

Each release is built and tested automatically by GitHub Actions on both systems. The editor itself is `md-lite.html` (embedded in the executable at build time). It also works on its own in a browser, with fewer features.

## Support

If MD Lite is useful to you, you can support its development on [Ko-fi](https://ko-fi.com/tom_t).

## License

[GPL-3.0-or-later](LICENSE)

---

## Français

Un lecteur et éditeur Markdown très léger pour Windows et Linux. Un double-clic sur un fichier `.md` l'ouvre : aperçu mis en forme, texte brut, ou les deux côte à côte.

**⬇ Télécharger la dernière version : [Windows (md-lite.exe)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite.exe) · [Linux (md-lite-linux-x86_64.tar.gz)](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite-linux-x86_64.tar.gz)** (environ 1 Mo, sans installation)

### Fonctionnalités

- **Trois vues** : texte brut, texte + aperçu, aperçu seul. Les fichiers s'ouvrent en aperçu par défaut (réglable).
- **Onglets** : plusieurs fichiers dans une seule fenêtre. Un fichier ouvert depuis l'Explorateur ou le gestionnaire de fichiers s'ajoute en onglet à la fenêtre déjà ouverte (ou dans une nouvelle fenêtre, au choix). La dernière session peut être rouverte.
- **Une barre d'outils pour toutes les balises Markdown** : titres, gras, italique, barré, surligné, code, listes, listes de tâches, citations, liens, images, tableaux (choix de la taille), blocs de code, notes de bas de page, séparateurs. Un second clic enlève la mise en forme.
- **Rendu** : Markdown façon GitHub, tableaux, cases à cocher (cliquables dans l'aperçu), notes de bas de page, coloration du code pour une vingtaine de langages, HTML courant des README (`<details>`, `<p align="center">`, `<img width>`, `<kbd>`…).
- **Images relatives au fichier** (`![](images/photo.png)`), chemins depuis la racine du dépôt git (`/docs/img.png`) et chemins absolus.
- **Les liens vers d'autres `.md`** s'ouvrent dans un nouvel onglet.
- **Sûr** : les scripts et le HTML dangereux des documents sont retirés, et une politique de sécurité bloque le reste.
- **Respecte les fichiers** : conserve l'encodage d'origine (UTF-8 avec ou sans BOM, UTF-16, Windows-1252) et les fins de ligne, écrit sans risque de fichier à moitié enregistré, détecte les modifications faites par d'autres programmes.
- Rechercher et remplacer, zoom, impression et export PDF, thème clair / sombre / système, correcteur orthographique, tous les réglages au même endroit.
- **Léger** : un seul exécutable d'environ 1 Mo. Il utilise le moteur web déjà présent sur le système : WebView2 sous Windows 10/11, WebKitGTK sous Linux.

### Installation

#### Windows

1. [Télécharger `md-lite.exe`](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite.exe) (ou le compiler, voir plus bas) et le placer dans n'importe quel dossier.
2. Au premier lancement, un bandeau propose d'en faire l'application par défaut pour les `.md` : un clic sur **Définir par défaut** ouvre la fenêtre de Windows, où il suffit de choisir MD Lite.
   `md-lite.exe --install` fait aussi l'enregistrement, et `md-lite.exe --uninstall` le retire.

Si l'exécutable est déplacé, l'association se met à jour toute seule au lancement suivant.

L'exécutable n'est pas signé numériquement : au premier lancement, Windows SmartScreen peut afficher un avertissement. Il suffit alors de cliquer sur **Informations complémentaires**, puis **Exécuter quand même**.

#### Linux

1. [Télécharger `md-lite-linux-x86_64.tar.gz`](https://github.com/tom-tour/md-lite/releases/latest/download/md-lite-linux-x86_64.tar.gz), l'extraire et placer `md-lite` dans n'importe quel dossier (par exemple `~/.local/bin`).
2. MD Lite a besoin de WebKitGTK 4.1 (version 2.40 ou plus récente), déjà présent sur la plupart des bureaux. Sinon :
   - Debian, Ubuntu, Mint : `sudo apt install libwebkit2gtk-4.1-0`
   - Fedora : `sudo dnf install webkit2gtk4.1`
   - Arch : `sudo pacman -S webkit2gtk-4.1`
3. Au premier lancement de `./md-lite`, un bandeau propose d'en faire l'application par défaut pour les `.md` : un clic sur **Définir par défaut** suffit. MD Lite apparaît aussi dans le menu des applications.
   `./md-lite --install` l'ajoute aussi au menu, et `./md-lite --uninstall` l'en retire.

Fonctionne sur Ubuntu 22.04 ou plus récent et les autres distributions de la même époque. Si l'exécutable est déplacé, l'entrée du menu se met à jour toute seule au lancement suivant.

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

Prérequis : [Rust](https://rustup.rs).

- **Windows** : la chaîne MSVC et les Build Tools de Visual Studio. Le script `build.cmd` compile la version optimisée et copie `md-lite.exe` à côté de lui.
- **Linux** : les paquets de développement de WebKitGTK et GTK (Debian/Ubuntu : `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev`), puis `cargo build --release`. L'exécutable est `target/release/md-lite`.

Chaque version est compilée et testée automatiquement par GitHub Actions sur les deux systèmes. L'éditeur lui-même est `md-lite.html` (intégré à l'exécutable à la compilation). Il fonctionne aussi seul dans un navigateur, avec moins de fonctions.

### Soutien

Si MD Lite rend service, un soutien sur [Ko-fi](https://ko-fi.com/tom_t) m'aide à continuer à le développer.

### Licence

[GPL-3.0-or-later](LICENSE)
