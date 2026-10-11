#!/bin/sh
# ==============================================================================
#  MozInRustBackup — désinstallation
#
#  Retire le programme, le raccourci de bureau et l'icône.
#  Vos sauvegardes (.mozinrustbackup) et vos profils ne sont jamais touchés.
#
#  Usage :
#     ./uninstall.sh              retire l'installation personnelle
#     ./uninstall.sh --system     retire l'installation système
# ==============================================================================
set -eu

MODE=user
for arg in "$@"; do
    case "$arg" in
        --system) MODE=system ;;
        -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Option inconnue : $arg" >&2; exit 2 ;;
    esac
done

if [ "$(id -u)" = "0" ] || [ "$MODE" = "user" ]; then
    SUDO=""
elif command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
else
    echo "Cette désinstallation nécessite les droits administrateur." >&2
    exit 1
fi

if [ "$MODE" = "system" ]; then
    PREFIX=/usr/local
    APPLICATIONS=/usr/share/applications
    ICONS=/usr/share/icons/hicolor/scalable/apps
else
    PREFIX="$HOME/.local"
    APPLICATIONS="$HOME/.local/share/applications"
    ICONS="$HOME/.local/share/icons/hicolor/scalable/apps"
fi

echo "Désinstallation de MozInRustBackup ($MODE)…"
for fichier in \
    "$PREFIX/bin/mozinrustbackup" \
    "$APPLICATIONS/mozinrustbackup.desktop" \
    "$ICONS/mozinrustbackup.svg"
do
    if [ -e "$fichier" ]; then
        $SUDO rm -f "$fichier"
        echo "  supprimé : $fichier"
    fi
done

if command -v update-desktop-database >/dev/null 2>&1; then
    $SUDO update-desktop-database -q "$APPLICATIONS" 2>/dev/null || true
fi

echo
echo "Terminé. Vos sauvegardes (.mozinrustbackup) et vos profils sont intacts."
echo "Le dossier de configuration (~/.config/mozinrustbackup) peut être supprimé sans risque."
