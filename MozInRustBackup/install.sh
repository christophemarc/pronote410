#!/bin/sh
# ==============================================================================
#  MozInRustBackup — installation sous Linux (toutes distributions)
#
#  Le script :
#    1. vérifie que Rust est disponible (et propose de l'installer si besoin) ;
#    2. compile le programme (aucune dépendance externe, fonctionne hors ligne) ;
#    3. installe le binaire, le raccourci de bureau et l'icône ;
#    4. vérifie la présence des boîtes de dialogue natives (zenity/kdialog).
#
#  Usage :
#     ./install.sh                 installation pour l'utilisateur courant
#                                  (aucun mot de passe demandé)
#     ./install.sh --system        installation pour tous les utilisateurs
#                                  (droits administrateur)
#     ./install.sh --no-dialogs    ne propose pas d'installer zenity
#     ./install.sh --help
#
#  Désinstallation : ./uninstall.sh
# ==============================================================================
set -eu

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$DIR"

MODE=user
INSTALL_DIALOGS=1
for arg in "$@"; do
    case "$arg" in
        --system) MODE=system ;;
        --no-dialogs) INSTALL_DIALOGS=0 ;;
        -h|--help)
            sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "Option inconnue : $arg" >&2; exit 2 ;;
    esac
done

# ------------------------------------------------------------------------------
# Couleurs (désactivées si la sortie n'est pas un terminal)
# ------------------------------------------------------------------------------
if [ -t 1 ]; then
    B="\033[1m"; V="\033[32m"; J="\033[33m"; R="\033[31m"; N="\033[0m"
else
    B=""; V=""; J=""; R=""; N=""
fi
etape() { printf '%s→ %s%s\n' "$B" "$1" "$N"; }
ok()    { printf '%s✓ %s%s\n' "$V" "$1" "$N"; }
attention() { printf '%s! %s%s\n' "$J" "$1" "$N"; }
erreur() { printf '%s✗ %s%s\n' "$R" "$1" "$N" >&2; }

# ------------------------------------------------------------------------------
# Distribution et gestionnaire de paquets
# ------------------------------------------------------------------------------
if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
fi
DISTRO_ID=${ID:-inconnue}
DISTRO_NAME=${PRETTY_NAME:-Linux}
detecter_famille() {
    case "${ID:-}" in
        debian|ubuntu|linuxmint|lmde|zorin|pop|elementary|kali|raspbian|devuan|mx|antix|deepin|pureos) echo debian ;;
        arch|archarm|cachyos|manjaro|endeavouros|garuda|artix|arcolinux|omarchy|steamos) echo arch ;;
        fedora|rhel|centos|rocky|almalinux|oracle|nobara|bazzite) echo fedora ;;
        opensuse*|sles|suse) echo suse ;;
        nixos) echo nix ;;
        gentoo|funtoo) echo gentoo ;;
        alpine) echo alpine ;;
        void) echo void ;;
        solus) echo solus ;;
        slackware|salix) echo slackware ;;
        *)
            case " ${ID_LIKE:-} " in
                *" debian "*) echo debian ;;
                *" arch "*) echo arch ;;
                *" fedora "*|*" rhel "*) echo fedora ;;
                *" suse "*) echo suse ;;
                *" alpine "*) echo alpine ;;
                *" gentoo "*) echo gentoo ;;
                *) echo inconnue ;;
            esac ;;
    esac
}
FAMILLE=$(detecter_famille)

SUDO=""
if [ "$(id -u)" != "0" ]; then
    if command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
    else
        SUDO=""
    fi
fi

installer_paquet() {
    # $1 : nom du paquet
    paquet="$1"
    case "$FAMILLE" in
        debian)  $SUDO apt-get update -qq && $SUDO apt-get install -y "$paquet" ;;
        arch)    $SUDO pacman -S --needed --noconfirm "$paquet" ;;
        fedora)  $SUDO dnf install -y "$paquet" ;;
        suse)    $SUDO zypper --non-interactive install "$paquet" ;;
        alpine)  $SUDO apk add "$paquet" ;;
        void)    $SUDO xbps-install -y "$paquet" ;;
        solus)   $SUDO eopkg install -y "$paquet" ;;
        gentoo)  $SUDO emerge "$paquet" ;;
        nix)     $SUDO nix-env -iA "nixpkgs.$paquet" ;;
        *)       return 1 ;;
    esac
}
paquet_rust() {
    case "$FAMILLE" in
        debian)  echo "rustc" ;;
        arch)    echo "rust" ;;
        fedora)  echo "rust" ;;
        suse)    echo "rust" ;;
        alpine)  echo "rust" ;;
        void)    echo "rust" ;;
        solus)   echo "rust" ;;
        gentoo)  echo "dev-lang/rust-bin" ;;
        nix)     echo "rustc" ;;
        *)       echo "rustc" ;;
    esac
}
paquet_zenity() {
    case "$FAMILLE" in
        gentoo) echo "gnome-extra/zenity" ;;
        nix)    echo "zenity" ;;
        *)      echo "zenity" ;;
    esac
}

printf '\n%sMozInRustBackup — installation%s\n' "$B" "$N"
echo "Distribution détectée : $DISTRO_NAME"
echo

# ------------------------------------------------------------------------------
# 1. Rust
# ------------------------------------------------------------------------------
etape "Vérification de Rust"
BESOIN_RUST=1
if command -v cargo >/dev/null 2>&1; then
    VERSION=$(cargo --version | awk '{print $2}')
    MAJEUR=$(echo "$VERSION" | cut -d. -f1)
    MINEUR=$(echo "$VERSION" | cut -d. -f2)
    if [ "$MAJEUR" -gt 1 ] || { [ "$MAJEUR" -eq 1 ] && [ "$MINEUR" -ge 70 ]; }; then
        ok "cargo $VERSION détecté"
        BESOIN_RUST=0
    else
        attention "cargo $VERSION est trop ancien (Rust 1.70 minimum requis)"
    fi
elif command -v rustc >/dev/null 2>&1; then
    VERSION=$(rustc --version | awk '{print $2}')
    MAJEUR=$(echo "$VERSION" | cut -d. -f1)
    MINEUR=$(echo "$VERSION" | cut -d. -f2)
    if [ "$MAJEUR" -gt 1 ] || { [ "$MAJEUR" -eq 1 ] && [ "$MINEUR" -ge 70 ]; }; then
        ok "rustc $VERSION détecté (sans cargo)"
        BESOIN_RUST=0
    else
        attention "rustc $VERSION est trop ancien (Rust 1.70 minimum requis)"
    fi
else
    attention "Rust n'est pas installé"
fi

if [ "$BESOIN_RUST" = "1" ]; then
    echo
    echo "MozInRustBackup est écrit en Rust (le même langage que Firefox)."
    echo "Il faut un compilateur Rust pour construire le programme."
    echo
    echo "  1) Installer Rust avec le paquet de la distribution ($(paquet_rust))"
    echo "  2) Installer Rust avec rustup (site officiel, ~/.cargo)"
    echo "  3) Quitter"
    printf 'Votre choix [1] : '
    read -r choix || choix=1
    [ -z "$choix" ] && choix=1
    case "$choix" in
        1)
            etape "Installation de $(paquet_rust) via le gestionnaire de paquets"
            if installer_paquet "$(paquet_rust)"; then
                ok "Rust installé"
            else
                erreur "L'installation automatique a échoué."
                echo "Installez Rust manuellement, puis relancez ce script."
                exit 1
            fi
            ;;
        2)
            etape "Installation de rustup"
            if command -v curl >/dev/null 2>&1; then
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
                # shellcheck disable=SC1091
                [ -r "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
                ok "Rust installé dans ~/.cargo"
            else
                erreur "curl est nécessaire pour rustup (installez-le puis réessayez)."
                exit 1
            fi
            ;;
        *) echo "Installation interrompue."; exit 0 ;;
    esac
fi

# ------------------------------------------------------------------------------
# 2. Compilation
# ------------------------------------------------------------------------------
etape "Compilation du programme"
if command -v cargo >/dev/null 2>&1; then
    cargo build --release --locked 2>/dev/null || cargo build --release
    BIN_SOURCE="$DIR/target/release/mozinrustbackup"
else
    mkdir -p build
    rustc --edition 2021 -C opt-level=2 --crate-type lib --crate-name mozinrustbackup \
        src/lib.rs --out-dir build
    rustc --edition 2021 -C opt-level=2 --crate-name mozinrustbackup \
        --extern mozinrustbackup=build/libmozinrustbackup.rlib \
        -L build src/main.rs -o build/mozinrustbackup
    BIN_SOURCE="$DIR/build/mozinrustbackup"
fi
[ -x "$BIN_SOURCE" ] || { erreur "La compilation a échoué."; exit 1; }
ok "Compilation réussie"

# ------------------------------------------------------------------------------
# 3. Installation des fichiers
# ------------------------------------------------------------------------------
etape "Installation des fichiers"
if [ "$MODE" = "system" ]; then
    PREFIX=/usr/local
    APPLICATIONS=/usr/share/applications
    ICONS=/usr/share/icons/hicolor/scalable/apps
    [ -n "$SUDO" ] || [ "$(id -u)" = "0" ] || {
        erreur "L'installation système nécessite les droits administrateur."
        exit 1
    }
else
    PREFIX="$HOME/.local"
    APPLICATIONS="$HOME/.local/share/applications"
    ICONS="$HOME/.local/share/icons/hicolor/scalable/apps"
fi

$SUDO install -d "$PREFIX/bin" "$APPLICATIONS" "$ICONS"
$SUDO install -m 0755 "$BIN_SOURCE" "$PREFIX/bin/mozinrustbackup"
$SUDO install -m 0644 packaging/mozinrustbackup.svg "$ICONS/mozinrustbackup.svg"
# Le champ Exec est ajusté si l'installation est personnelle.
sed "s|^Exec=mozinrustbackup|Exec=$PREFIX/bin/mozinrustbackup|" \
    packaging/mozinrustbackup.desktop > /tmp/mozinrustbackup.desktop.$$
$SUDO install -m 0644 /tmp/mozinrustbackup.desktop.$$ "$APPLICATIONS/mozinrustbackup.desktop"
rm -f /tmp/mozinrustbackup.desktop.$$
ok "Programme : $PREFIX/bin/mozinrustbackup"
ok "Raccourci : $APPLICATIONS/mozinrustbackup.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    $SUDO update-desktop-database -q "$APPLICATIONS" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    $SUDO gtk-update-icon-cache -q -t -f "$(dirname "$ICONS")/.." 2>/dev/null || true
fi

# ------------------------------------------------------------------------------
# 4. Boîtes de dialogue natives
# ------------------------------------------------------------------------------
if [ "$INSTALL_DIALOGS" = "1" ]; then
    etape "Vérification des boîtes de dialogue"
    OUTIL=""
    for candidat in zenity yad kdialog; do
        if command -v "$candidat" >/dev/null 2>&1; then
            OUTIL="$candidat"
            break
        fi
    done
    if [ -n "$OUTIL" ]; then
        ok "$OUTIL est installé : fenêtres natives du bureau"
    else
        attention "Aucune boîte de dialogue graphique détectée."
        echo "  Le programme fonctionnera dans le terminal (mode texte)."
        echo "  Pour les fenêtres habituelles de votre bureau :"
        echo "     paquet à installer : $(paquet_zenity)"
        printf 'Installer maintenant ? [O/n] '
        read -r reponse || reponse=n
        case "$reponse" in
            n|N|non|Non) ;;
            *)
                if installer_paquet "$(paquet_zenity)"; then
                    ok "zenity installé"
                else
                    attention "Installation impossible automatiquement : installez $(paquet_zenity) à la main."
                fi ;;
        esac
    fi
fi

# ------------------------------------------------------------------------------
# Résumé
# ------------------------------------------------------------------------------
echo
printf '%sInstallation terminée.%s\n' "$B" "$N"
echo
echo "Lancement :"
if [ "$MODE" = "user" ]; then
    case ":$PATH:" in
        *":$HOME/.local/bin:"*) echo "   • tapez « mozinrustbackup » dans un terminal" ;;
        *) echo "   • tapez « $PREFIX/bin/mozinrustbackup »"
           echo "     (ajoutez « export PATH=\"\$HOME/.local/bin:\$PATH\" » à votre ~/.profile"
           echo "      pour la commande courte, puis reconnectez-vous)" ;;
    esac
else
    echo "   • tapez « mozinrustbackup » dans un terminal"
fi
echo "   • ou passez par le menu de votre bureau : « MozInRustBackup »"
echo
echo "En ligne de commande :"
echo "   mozinrustbackup --liste            profils trouvés sur ce disque"
echo "   mozinrustbackup --aide             toutes les options"
echo
echo "Désinstallation : $DIR/uninstall.sh"
