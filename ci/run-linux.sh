#!/usr/bin/env bash
# Lance MD Lite sur un écran virtuel et le pilote avec ci/selftest.js.
# Usage : ci/run-linux.sh <binaire md-lite> <dossier pour les captures>
set -u
BIN=$(realpath "$1")
SHOTS=$(realpath -m "$2")
CI=$(dirname "$(realpath "$0")")
mkdir -p "$SHOTS"

export HOME=$(mktemp -d) XDG_RUNTIME_DIR=$(mktemp -d)
export XDG_CONFIG_HOME=$HOME/.config XDG_DATA_HOME=$HOME/.local/share
export MDLITE_SELFTEST=$(mktemp -d)
cp "$CI/selftest.js" "$MDLITE_SELFTEST/"
WORK=$(mktemp -d)
cp "$CI/test.md" "$CI/second.md" "$WORK/"
mkdir "$WORK/img" && cp "$CI/../icons/icon.png" "$WORK/img/"

export DISPLAY=:99
Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 &
sleep 1

shot() { import -window root "$SHOTS/$1.png"; }
# Sans gestionnaire de fenêtres, on donne le focus à la fenêtre la plus récente (la boîte de dialogue).
escape() { W=$(xdotool search --onlyvisible --name . | sort -n | tail -1); xdotool windowfocus --sync "$W" key Escape; }
wait_for() { for _ in $(seq 300); do [ -e "$MDLITE_SELFTEST/$1.out" ] && return 0; sleep 0.1; done; echo "délai dépassé : $1"; return 1; }
fail() { echo "ÉCHEC : $*"; shot echec; cat "$MDLITE_SELFTEST/result.out" 2>/dev/null; kill "$PID" 2>/dev/null; exit 1; }

dbus-run-session -- "$BIN" "$WORK/test.md" > "$SHOTS/stdout.txt" 2>&1 &
PID=$!

wait_for ready || fail "démarrage"
sleep 1; shot 1-fenetre
timeout 10 "$BIN" "$WORK/second.md" || fail "second lancement (devrait rendre la main tout de suite)"
echo "ok   second lancement rendu à la première fenêtre"
wait_for dialog || fail "étape dialogue"
sleep 2; shot 2-dialogue
escape
wait_for pick || fail "étape ouverture"
sleep 2; shot 3-ouvrir
escape
wait_for dark || fail "étape thème sombre"
sleep 1; shot 4-sombre

wait_for result || fail "résultat"
cat "$MDLITE_SELFTEST/result.out"; echo
for _ in $(seq 100); do kill -0 "$PID" 2>/dev/null || break; sleep 0.1; done
kill -0 "$PID" 2>/dev/null && fail "la fenêtre ne s'est pas fermée"
wait "$PID"; CODE=$?
[ "$CODE" = 0 ] || fail "code de sortie $CODE"
grep -q '^FAIL' "$MDLITE_SELFTEST/result.out" && fail "vérifications"
grep -q 'Ligne ajoutée par le test.' "$WORK/test.md" || fail "texte non enregistré dans le fichier"
grep -q 'test.md' "$HOME/.config/md-lite/session.json" || { ls -la "$HOME/.config/md-lite"; cat "$HOME/.config/md-lite/"*.json; fail "session non enregistrée"; }
[ "$(xdg-mime query default text/markdown)" = md-lite.desktop ] || fail "association .md"
echo "ok   fichier enregistré, session sauvée, fermeture propre"

# Installation en ligne de commande : fichier .desktop et icône (déjà créés par « Définir par défaut »).
"$BIN" --install --quiet || fail "--install"
DESKTOP=$HOME/.local/share/applications/md-lite.desktop
desktop-file-validate "$DESKTOP" || fail ".desktop invalide"
grep -qx "Exec=\"$BIN\" %F" "$DESKTOP" || fail "ligne Exec"
[ -s "$HOME/.local/share/icons/hicolor/256x256/apps/md-lite.png" ] || fail "icône"
echo "ok   installation .desktop"
"$BIN" --uninstall --quiet && [ ! -e "$DESKTOP" ] || fail "--uninstall"
echo "ok   désinstallation"
echo "TOUS LES TESTS SONT PASSÉS"
