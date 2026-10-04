#!/usr/bin/env bash
# ==============================================================================
# Installation Pronote Client 2026 - Script universel Linux v4.10
# Support natif : Debian, Ubuntu, Mint, Fedora, Arch, CachyOS, Manjaro,
#                 Omarchy, EndeavourOS, Garuda, openSUSE, NixOS, GLF OS,
#                 Solus, Alpine Linux, Void Linux, Slackware (et Salix),
#                 Gentoo (et Funtoo)
# Ce script n'est pas écrit ni supporté par Index Education.
# Il n'y a pas de support technique lié à son utilisation.
# Support automatique étendu : toute distribution dérivée (via ID_LIKE)
#
# Préfixes Wine :
#   64 bits : ~/.local/share/wineprefixes/pronote-2026
#   32 bits : ~/.local/share/wineprefixes/pronote-2026-32
#
# Changelog v4.10 :
#   - Fix Ubuntu 26.04 / Mint (installation de Wine) :
#     • Sélection automatique de la branche WineHQ : le dépôt WineHQ ne propose
#       plus « winehq-stable » pour certaines bases (Ubuntu 26.04 « resolute » :
#       winehq-stable absent du dépôt, winehq-devel 11 disponible). Le script
#       choisit désormais tout seul un paquet Wine 10/11 : winehq-stable puis
#       winehq-devel puis winehq-staging (chaque branche proposant Wine 10
#       ou plus), puis repli sur le Wine de la distribution. Plus besoin de
#       corriger le script à chaque retrait de version du dépôt WineHQ.
#     • Filet de sécurité version : après installation, la version de Wine est
#       vérifiée. Pronote refusant de se lancer avec Wine 9 ou inférieur, le
#       script retente automatiquement le dépôt WineHQ si un Wine trop ancien
#       est en place (cas Mint 22.3 : Wine 9.0 installé depuis les dépôts) et
#       explique clairement l'échec le cas échéant (contournement expert :
#       PRONOTE_IGNORE_WINE_VERSION=1).
#     • Verrou dpkg : un apt-get encore actif (essai précédent interrompu,
#       session Live) bloquait toute nouvelle installation (« Could not get
#       lock /var/lib/dpkg/lock-frontend »). Le script attend désormais la fin
#       du processus en cours (au plus 5 minutes, affichage explicite) et apt
#       attend lui-même via DPkg::Lock::Timeout=600. Message clair sur la
#       longue phase de décompression après le téléchargement (surtout en
#       session Live) pour ne plus croire l'installation « figée ».
#   - Nouveau : ouverture des pièces jointes du cahier de textes avec les
#     applications Linux (via le registre Wine, indépendant de l'environnement
#     de bureau : KDE, GNOME, XFCE, Cinnamon, MATE, LXQt, COSMIC, Hyprland...).
#     Images, PDF, vidéos... s'ouvrent dans le navigateur par défaut ; les
#     documents bureautiques (Word, Excel, PowerPoint, OpenDocument, RTF,
#     texte...) dans la suite bureautique installée par défaut (LibreOffice,
#     OnlyOffice...) ; si aucune suite n'est installée, rien n'est ouvert.
#     Les associations sont écrites dans le registre du préfixe Wine : elles
#     disparaissent avec lui et n'atteignent pas le reste du système.
#   - Nouveau : écrans haute résolution (QHD / 4K). Wine n'applique aucune
#     mise à l'échelle par défaut : l'Arial 11 de Pronote y devient minuscule.
#     Si un grand écran est détecté (QHD, 4K ou forte densité de pixels), le
#     script propose simplement d'agrandir les caractères (choix O/N, réglage
#     DPI équivalent à Windows). Sur un écran 1080p ou plus petit, rien n'est
#     demandé : la taille par défaut convient.
#   - Nouveau : support Gentoo / Funtoo (emerge), avec avertissement clair sur
#     la durée (Wine y est compilé depuis les sources).
#   - Revue des familles peu testées (Void, Alpine, Solus, Slackware) : pas de
#     changement de comportement, vérification des commandes et des paquets.
#   - Aucun changement des autres correctifs précédents (Ubuntu/Zorin/NixOS,
#     conffiles dpkg, force-overwrite, préfixes séparés, désinstallateurs) :
#     mêmes choix 32/64 bits, mêmes chemins, mêmes lanceurs.
#
# Changelog v4.9 :
#   - Fix NixOS (bug majeur : les deux architectures s'écrasaient) : la
#     variable WINEPREFIX était définie ET exportée dès le démarrage du
#     script, avec la valeur par défaut = préfixe 64 bits. Or NixOS relance
#     le script dans nix-shell (exec) : le second processus héritait donc de
#     WINEPREFIX, ce que le script interprétait comme « préfixe forcé par
#     l'utilisateur ». Résultat : l'installation 32 bits était faite DANS le
#     préfixe 64 bits (d'où la question inattendue « Réutiliser ce préfixe
#     existant ? »), la version 64 bits devenait morte, et désinstaller la
#     version 64 bits cassait aussi la 32 bits (préfixe partagé).
#     Désormais WINEPREFIX n'est plus exporté avant d'avoir été choisi, un
#     préfixe forcé par l'utilisateur est transmis explicitement au re-exec
#     NixOS (PRONOTE_FORCED_WINEPREFIX), et l'environnement est nettoyé.
#   - Suppression de la question « Réutiliser ce préfixe existant ? [O/n] » :
#     plus aucune décision n'est demandée après le choix 32/64 bits. Un
#     préfixe existant de même architecture est réutilisé ; s'il appartient à
#     l'autre architecture, le script choisit lui-même un autre chemin. Un
#     préfixe n'est plus jamais déplacé ni supprimé automatiquement (cette
#     suppression était la cause de la perte de la version déjà installée).
#   - Fix Zorin OS / dérivés Debian (blocage de l'installation) :
#     DEBIAN_FRONTEND=noninteractive ne suffit pas à empêcher la question
#     dpkg sur les fichiers de configuration (« *** settings.ini.real
#     (Y/I/N/O/D/Z) [défaut=N] ? »), posée par dpkg lui-même lorsque le
#     paquet libgtk-3-0t64:i386 apporte une nouvelle version d'un conffile
#     absent du système. Tous les appels apt passent maintenant par un
#     wrapper non interactif : --force-confdef, --force-confmiss (conffile
#     manquant restauré, équivalent de « Y ») et --force-confold (conffile
#     modifié conservé). Plus aucune question, choix identique à celui d'un
#     administrateur. Ajouts : réparation préalable « dpkg --configure -a »,
#     NEEDRESTART_MODE=a, APT_LISTCHANGES_FRONTEND=none, et second essai
#     ciblé --force-overwrite sur l'erreur Zorin « trying to overwrite
#     shared '/etc/gtk-3.0/settings.ini' ».
#   - Désinstallateurs : garde-fou si deux versions partagent le même préfixe
#     (situation héritée d'une installation antérieure) : la désinstallation
#     Wine interne est refusée au lieu de casser l'autre version.
#   - Sondage WINEARCH=win32 effectué seulement quand il est utile (plus de
#     préfixe de test créé pour rien lorsqu'un préfixe 32 bits existe déjà).
#   - Aucune régression volontaire : choix 32/64 bits, chemins des préfixes,
#     dépôt WineHQ, lanceurs, icônes et désinstallateurs inchangés.
#
# Changelog v4.8 :
#   - Fix Kubuntu/Ubuntu/Debian/Mint (échec au 1er lancement, « Le préfixe
#     Wine est incomplet : system.reg manquant », succès au 2e lancement) :
#     avec le Wine des dépôts Debian/Ubuntu, « wineserver » n'est pas dans le
#     PATH mais dans /usr/lib/<multiarch>/wine/. Le script ne le trouvait pas,
#     n'attendait donc pas l'arrêt du wineserver, et vérifiait system.reg
#     avant que Wine ne l'ait écrit sur disque (le registre n'est enregistré
#     qu'à l'arrêt du wineserver). Correctifs valables pour toutes les
#     distributions : découverte de wineserver dans /usr/lib, /usr/lib64,
#     /opt/wine-*, /usr/libexec ; attente de la fin de session Wine via /proc
#     si wineserver reste introuvable ; contrôle du préfixe patient (jusqu'à
#     30 s) puis second essai d'initialisation avant tout abandon.
#   - Fix lenteur de fin d'installation (CachyOS, Fedora : 5 à 10 minutes de
#     silence après « Installation Pronote : terminé », régression v4.7) :
#     le script attendait « wineserver -w » pendant 300 s alors que le service
#     de mise à jour Index Éducation lancé par l'installateur ne s'arrête
#     jamais seul. Désormais : fermeture douce de Pronote s'il a été lancé,
#     attente bornée à 10 s, puis « wineserver -k » (SIGINT : le registre est
#     sauvegardé, les processus arrêtés). Chaque attente est visible (barre
#     animée + consigne « ne fermez pas cette fenêtre »). Les autres attentes
#     (init du préfixe, winetricks, regedit) sont bornées de la même façon.
#     Le temps de fin d'installation redevient celui de la v4.6.
#   - Retour du réglage v4.6 WINEDLLOVERRIDES=mscoree,mshtml= pendant la
#     préparation du préfixe : plus de fenêtre « Wine Mono / Gecko » sur les
#     distributions qui ne fournissent pas ces paquets (Debian/Ubuntu).
#   - run_with_timeout : repli interne (surveillance + kill) si la commande
#     « timeout » est absente ; téléchargements bornés (curl --max-time).
#   - Slackware 15.0 / -current / Salix : installation automatique de Wine
#     (slapt-get, slackpkg+ dépôt alienbob, ou paquets Alien Bob téléchargés
#     avec leurs dépendances .dep : FAudio + wine), outils via sbopkg.
#   - openSUSE : installation des outils sans « zypper search » préalable ;
#     Void : mise à jour de xbps avant l'installation ; Alpine : paquets
#     rsvg-convert, font-dejavu, xdg-user-dirs, gtk-update-icon-cache ;
#     Solus : la liste ne mentionne plus « wine-32bit » (n'existe plus).
#   - Aucune modification du choix 32/64 bits, des préfixes séparés, des
#     désinstallateurs, ni du dépôt WineHQ sur Debian/Ubuntu/Mint.
#
# Changelog v4.7 :
#   - Fix Solus (bug majeur) : le paquet « wine-32bit » n'existe plus.
#     Solus construit désormais Wine 11 en WoW64 intégré
#     (--enable-archs=x86_64,i386) : le paquet « wine » couvre 32 et 64 bits.
#     Les paquets sont installés un par un ; ceux absents du dépôt sont
#     ignorés au lieu de faire échouer toute la transaction eopkg.
#     En session Live, l'option --ignore-safety évite d'engorger la RAM avec
#     la mise à jour forcée de system.base.
#   - Fix Fedora (bug majeur) : les erreurs constatées (error=112,
#     « No space left on device », « wine: could not load kernel32.dll ») sont
#     dues au remplissage de la couche d'écriture du Live CD. Le script mesure
#     désormais l'espace libre avant d'installer Wine, avant le préfixe et
#     avant Pronote, avertit clairement en session Live et s'arrête avec un
#     message explicite au lieu de poursuivre sur un préfixe corrompu.
#     Installation dnf en deux temps (wine obligatoire, outils optionnels
#     séparés) : plus d'arrêt silencieux après « Complete! ».
#   - Fix Fedora GNOME (icône « n » au lieu du papillon) : téléchargements
#     compatibles wget2 (Fedora ≥ 40), busybox (Alpine) et curl, suppression
#     de l'option -L, conversion via magick ou convert, repli rsvg-convert,
#     extraction depuis l'exécutable Pronote (icoutils installé partout),
#     icône de secours SVG sans texte (plus de rendu de police aléatoire).
#   - Fix NixOS (32 bits bloqué sur « Forçage de la version Windows ») :
#     toutes les opérations Wine hors installateur sont bornées par un délai
#     (timeout), le forçage du registre passe par un seul import regedit,
#     wineserver est arrêté proprement en cas de blocage, et le choix
#     « messages Wine » est transmis au nix-shell (plus de double question).
#   - Fix Ubuntu MATE (Pronote absent du menu Éducation) : fichier de menu
#     XDG fusionné (~/.config/menus/applications-merged) qui place Pronote
#     explicitement dans Éducation (MATE, XFCE, Cinnamon, Budgie, KDE, LXQt),
#     winemenubuilder désactivé pendant l'installateur Pronote, nettoyage
#     récursif des entrées Wine résiduelles (.desktop + .menu).
#   - openSUSE / Void / Alpine : installation paquet par paquet (un paquet
#     manquant n'annule plus la transaction), dépôt community Alpine activé
#     si besoin, wget busybox géré.
#   - Aucune modification volontaire du comportement Debian/Ubuntu/Mint et
#     Arch/CachyOS : WineHQ (.asc + .key), ordre WineHQ → winetricks,
#     multilib, préfixes séparés 32/64, désinstallateurs identiques.
#
# Changelog v4.6 :
#   - Fix Kubuntu/Ubuntu 26.04 (resolute) : erreur 127 « wineboot introuvable ».
#     WineHQ 11 place les binaires dans /opt/wine-stable/bin sans toujours
#     créer les liens /usr/bin. Le script découvre wine, wineboot, wineserver
#     et winecfg, les injecte dans le PATH, et n'utilise plus « env wineboot »
#     (qui provoquait exactement le code 127 constaté).
#   - Fix Ubuntu 26.04 : clé WineHQ enregistrée aussi en .asc (APT 3.2 refuse
#     une clé ASCII sous une extension .key).
#   - Fix ordre Debian/Ubuntu : WineHQ est installé AVANT winetricks.
#     Avec WineHQ, winetricks est le script officiel autonome.
#   - Fix 32 bits / Wine 10+ / Wine 11 : « WINEARCH is set to win32 but this
#     is not supported in wow64 mode ». Le script sonde le support win32.
#     S'il est absent, Pronote 32 bits s'installe dans un préfixe WoW64 dédié.
#   - Les versions 32 et 64 bits restent installables en parallèle partout.
#   - Fix NixOS / GLF OS : nix-shell -p sur le nixpkgs système, avec repli
#     wineWow64Packages.stable → wineWowPackages.stable → wine64 → wine,
#     puis « nix shell nixpkgs#... » (flakes) si le canal est absent.
#   - Support Solus, Alpine Linux, Void Linux, openSUSE renforcé, sudo/doas.
#
# Changelog v4.5 :
#   - Fix Debian/Ubuntu/Mint : Windows 10 forcé sur la famille debian.
#   - Préfixes séparés par architecture + désinstallateurs 32/64.
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Configuration générale
# ------------------------------------------------------------------------------

readonly SCRIPT_VERSION="4.10"
readonly DEFAULT_YEAR="2026"
readonly DEFAULT_VERSION="2026.2.6"
# Pronote 2026 refuse de se lancer avec Wine 9 ou une version inférieure :
# la version minimale exigée est donc Wine 10.0.
readonly MIN_WINE_VERSION="10.0"
readonly PRONOTE_DOWNLOAD_PAGE="https://www.index-education.com/fr/telecharger-pronote.php"
readonly PRONOTE_ICON_URL="https://img.icons8.com/doodle/1200/pronote-logo.jpg"
readonly PRONOTE_ICON_URLS=(
    "https://img.icons8.com/doodle/1200/pronote-logo.jpg"
    "https://img.icons8.com/doodle/480/pronote-logo.png"
)
readonly HTTP_USER_AGENT="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) PronoteInstaller/4.10"
readonly ICON_SIZES=(16 22 24 32 48 64 128 256)
readonly ICON_BASE_DIR="$HOME/.local/share/icons/hicolor"
readonly ICON_DIR="$ICON_BASE_DIR/scalable/apps"
readonly PIXMAP_DIR="$HOME/.local/share/pixmaps"
readonly BIN_DIR="$HOME/.local/bin"
readonly APP_DIR="$HOME/.local/share/applications"
readonly MENU_MERGE_DIR="$HOME/.config/menus/applications-merged"
readonly LOG_DIR="$HOME/.local/share/pronote-installer"
readonly LOG_FILE="$LOG_DIR/install-$(date +%Y%m%d-%H%M%S).log"

# Espace disque (en Mo) : seuils d'avertissement et de blocage
readonly DISK_WARN_SYSTEM_MB=3000     # installation des paquets Wine
readonly DISK_WARN_HOME_MB=2500       # préfixe Wine + Pronote
readonly DISK_MIN_HOME_MB=1200        # en dessous : arrêt (échec certain)
readonly DISK_MIN_TMP_MB=400          # téléchargement de l'installateur

readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly RED='\033[0;31m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

# URLs de base (seront potentiellement mises à jour par le scraping)
URL_32="https://tele3.index-education.com/telechargement/pn/v${DEFAULT_YEAR}.0/exe/Install_PRNclient_FR_${DEFAULT_VERSION}_win32.exe"
URL_64="https://tele3.index-education.com/telechargement/pn/v${DEFAULT_YEAR}.0/exe/Install_PRNclient_FR_${DEFAULT_VERSION}_win64.exe"
DEFAULT_URL="$URL_64"

# ------------------------------------------------------------------------------
# Variables globales
# ------------------------------------------------------------------------------

DISTRO_ID="unknown"
DISTRO_NAME="Inconnue"
DISTRO_FAMILY="unknown"

PRONOTE_URL="$DEFAULT_URL"
PRONOTE_YEAR="$DEFAULT_YEAR"
PRONOTE_VERSION="$DEFAULT_VERSION"
PRONOTE_ARCH="64"

PRONOTE_URL_FROM_OPTION=""
PRONOTE_ARCH_FROM_OPTION=""

SHOW_WINE_LOGS="0"
ASSUME_YES="0"
UNINSTALL_ONLY="0"
UNINSTALL_ARCH=""
IN_NIX_SHELL="0"
IS_LIVE_SESSION="0"

# Mise à l'échelle HiDPI appliquée (pourcentage, vide si non appliquée)
HIDPI_APPLIED=""

WIN_VERSION="win11"

# Mode du préfixe : win64 (Pronote 64 bits), win32 (préfixe 32 bits natif)
# ou wow64 (Pronote 32 bits dans un préfixe 64 bits WoW64 dédié)
PREFIX_MODE="win64"

# Binaires Wine résolus (corrigent l'erreur 127 et wineserver not found)
WINE_BIN=""
WINEBOOT_BIN=""
WINECFG_BIN=""
WINESERVER_BIN=""
WINE_WIN32_PREFIX_OK="unknown"

# Outil timeout détecté (gnu / busybox / none)
TIMEOUT_KIND=""

# Arguments d'origine (reprise NixOS)
ALL_ARGS=("$@")

# ------------------------------------------------------------------------------
# Préfixe Wine : variable d'environnement et hygiène du re-exec
# ------------------------------------------------------------------------------
# L'utilisateur peut forcer un préfixe avec « WINEPREFIX=/chemin bash pronote.sh ».
# Cette valeur est mémorisée dans WINEPREFIX_USER, puis WINEPREFIX est retirée
# de l'environnement : elle ne doit JAMAIS être héritée par un re-exec du
# script. C'est précisément ce qui cassait NixOS : le script se relance dans
# nix-shell (exec) et le second processus voyait alors un WINEPREFIX déjà
# rempli (la valeur par défaut = préfixe 64 bits), croyait qu'il était forcé
# par l'utilisateur, et installait le 32 bits dans le préfixe du 64 bits.
# WINEPREFIX est exportée par set_wineprefix(), une fois le chemin choisi.
# ------------------------------------------------------------------------------
WINEPREFIX_USER="${PRONOTE_FORCED_WINEPREFIX:-${WINEPREFIX:-}}"
unset WINEPREFIX
WINEPREFIX=""

TMP_DIR="$(mktemp -d /tmp/pronote-installer-XXXXXX)"
trap 'rm -rf "$TMP_DIR" 2>/dev/null || true' EXIT

RUN_QUIET_ACTIVITY_DIR=""
RUN_QUIET_FINALIZE_MSG=""

# ==============================================================================
# Affichage / journalisation  (toujours sur stderr)
# ==============================================================================

log_info()    { echo -e "${BLUE}ℹ️  $*${NC}" >&2; }
log_success() { echo -e "${GREEN}✅ $*${NC}" >&2; }
log_warning() { echo -e "${YELLOW}⚠️  $*${NC}" >&2; }
log_error()   { echo -e "${RED}❌ $*${NC}" >&2; }

repeat_char() {
    local char="$1"
    local count="$2"
    local out=""
    local i

    for ((i = 0; i < count; i++)); do
        out+="$char"
    done

    printf "%s" "$out"
}

box_title() {
    local title="$1"
    local width="${2:-62}"
    local inner=$((width - 2))

    printf "╔"
    repeat_char "═" "$width"
    printf "╗\n"

    printf "║ %-*s ║\n" "$inner" "$title"

    printf "╚"
    repeat_char "═" "$width"
    printf "╝\n"
}

box_two_lines() {
    local line1="$1"
    local line2="$2"
    local width="${3:-62}"
    local inner=$((width - 2))

    printf "╔"
    repeat_char "═" "$width"
    printf "╗\n"

    printf "║ %-*s ║\n" "$inner" "$line1"
    printf "║ %-*s ║\n" "$inner" "$line2"

    printf "╚"
    repeat_char "═" "$width"
    printf "╝\n"
}

# ==============================================================================
# sudo / doas (Alpine, Void minimal, images live)
# ==============================================================================

ensure_privilege_helper() {
    if [ "$(id -u)" -eq 0 ]; then
        sudo() { "$@"; }
        return 0
    fi

    if command -v sudo >/dev/null 2>&1; then
        return 0
    fi

    if command -v doas >/dev/null 2>&1; then
        sudo() { doas "$@"; }
        log_info "sudo absent : utilisation de doas."
        return 0
    fi

    return 1
}

require_privilege_helper() {
    if ensure_privilege_helper; then
        return 0
    fi
    log_error "Ni sudo ni doas n'est disponible. Installez l'un des deux, ou relancez en root."
    exit 1
}

# ==============================================================================
# Aide
# ==============================================================================

usage() {
    cat << 'EOF'
Usage: pronote.sh [options]

Options:
  --help                 Afficher cette aide
  --yes                  Mode non-interactif (répond oui aux défauts)
  --arch 32|64           Forcer l'architecture de l'installateur
  --url URL              Utiliser une URL personnalisée d'installateur
  --uninstall [32|64]    Lancer le désinstallateur Pronote (32 ou 64 bits)
  --in-nix-shell         Utilisé en interne pour NixOS (ne pas utiliser)

Variables d'environnement facultatives :
  WINEPREFIX=/chemin     Forcer le préfixe Wine utilisé
  PRONOTE_IGNORE_DISK=1  Ignorer les contrôles d'espace disque
  PRONOTE_IGNORE_WINE_VERSION=1
                         Installer même si Wine < 10 est détecté (déconseillé :
                         Pronote refuse de démarrer avec Wine 9 ou inférieur)

Notes :
  • Les versions 32 et 64 bits utilisent des préfixes Wine distincts et
    peuvent être installées en même temps.
  • Désinstallation : pronote-desinstaller-32 ou pronote-desinstaller-64
  • Wine 10+/11 en mode WoW64 : Pronote 32 bits s'installe dans un préfixe
    WoW64 dédié (WINEARCH=win32 n'est plus créé, car non supporté).
EOF
}

# ==============================================================================
# Animation de progression + exécution silencieuse
# ==============================================================================

progress_pulse() {
    local label="$1"
    local pid="$2"
    local activity_dir="${3:-}"
    local finalize_msg="${4:-Finalisation en cours, veuillez patienter quelques secondes...}"

    local width=24
    local block=7
    local pos=0
    local direction=1
    local start_time=$SECONDS

    local check_activity=0
    [ -n "$activity_dir" ] && check_activity=1

    local last_size=-1
    local stable_count=0
    local finalized=0
    local sample_tick=0
    local current_label="$label"

    if [ ! -t 1 ]; then
        while kill -0 "$pid" 2>/dev/null; do
            sleep 1
        done
        return
    fi

    while kill -0 "$pid" 2>/dev/null; do
        local bar=""
        local i
        local elapsed=$((SECONDS - start_time))
        local min=$((elapsed / 60))
        local sec=$((elapsed % 60))

        if [ "$check_activity" -eq 1 ] && [ "$finalized" -eq 0 ]; then
            sample_tick=$((sample_tick + 1))

            if (( sample_tick >= 8 )); then
                sample_tick=0
                local cur_size
                cur_size="$(du -sk "$activity_dir" 2>/dev/null | cut -f1)"
                cur_size="${cur_size:-0}"

                if [ "$cur_size" = "$last_size" ]; then
                    stable_count=$((stable_count + 1))
                else
                    stable_count=0
                fi
                last_size="$cur_size"

                if (( stable_count >= 3 && elapsed >= 5 )); then
                    finalized=1
                    current_label="$finalize_msg"
                fi
            fi
        fi

        for ((i = 0; i < width; i++)); do
            if (( i >= pos && i < pos + block )); then
                bar+="█"
            else
                bar+=" "
            fi
        done

        printf "\r\033[K%s [%s] %02d:%02d" "$current_label" "$bar" "$min" "$sec"

        pos=$((pos + direction))

        if (( pos <= 0 )); then
            pos=0
            direction=1
        elif (( pos + block >= width )); then
            pos=$((width - block))
            direction=-1
        fi

        sleep 0.12
    done

    printf "\r\033[K"
}

# run_quiet n'utilise pas « env COMMANDE » : env renvoyait 127 dès que
# wineboot/wineserver n'étaient pas dans le PATH (cas WineHQ 11 / Ubuntu 26.04).
# Un sous-shell bash exporte WINEDEBUG puis lance la commande, y compris
# lorsqu'il s'agit d'une fonction du script.
run_quiet() {
    local label="$1"
    shift

    if [[ "$SHOW_WINE_LOGS" == "1" ]]; then
        log_info "$label"
        echo "Mode expert actif : les messages Wine sont affichés."
        export WINEDEBUG="${WINEDEBUG:-}"
        "$@"
        return $?
    fi

    mkdir -p "$LOG_DIR"

    (
        export WINEDEBUG="${WINEDEBUG_QUIET:--all}"
        "$@"
    ) >>"$LOG_FILE" 2>&1 &

    local pid=$!
    progress_pulse "$label" "$pid" "$RUN_QUIET_ACTIVITY_DIR" "$RUN_QUIET_FINALIZE_MSG"

    set +e
    wait "$pid"
    local status=$?
    set -e

    if [ "$status" -eq 0 ]; then
        log_success "$label : terminé"
    else
        log_warning "$label : terminé avec un code retour $status"
        echo "Les détails techniques sont enregistrés ici :"
        echo "  $LOG_FILE"
    fi

    return "$status"
}

# ------------------------------------------------------------------------------
# Petit helper pour les questions (support --yes)
# ------------------------------------------------------------------------------

ask_user() {
    local prompt="$1"
    local default="$2"

    if [ "$ASSUME_YES" = "1" ]; then
        echo "$default"
        return 0
    fi

    local answer
    read -rp "$prompt" answer
    echo "${answer:-$default}"
}

ask_wine_output_mode() {
    if [ "$ASSUME_YES" = "1" ]; then
        SHOW_WINE_LOGS="0"
        return 0
    fi

    # NixOS : le choix a déjà été fait avant la relance dans nix-shell.
    if [ "$IN_NIX_SHELL" = "1" ] && [ -n "${PRONOTE_SHOW_WINE_LOGS:-}" ]; then
        SHOW_WINE_LOGS="$PRONOTE_SHOW_WINE_LOGS"
        if [ "$SHOW_WINE_LOGS" = "1" ]; then
            log_warning "Mode expert conservé : les logs Wine seront affichés."
        else
            log_info "Mode silencieux conservé : les logs Wine seront masqués."
        fi
        return 0
    fi

    echo ""
    echo "Par défaut, les messages techniques de Wine seront masqués."
    echo "Ils sont utiles uniquement pour diagnostiquer un problème."
    read -rp "Afficher les messages techniques de Wine ? [o/N] : " answer

    case "$answer" in
        o|O|oui|Oui|OUI|y|Y|yes|YES)
            SHOW_WINE_LOGS="1"
            log_warning "Mode expert activé : les logs Wine seront affichés."
            ;;
        *)
            SHOW_WINE_LOGS="0"
            log_info "Mode silencieux activé : les logs Wine seront masqués."
            ;;
    esac

    return 0
}

# ==============================================================================
# Détection système
# ==============================================================================

detect_distro() {
    if [ -f /etc/os-release ]; then
        # shellcheck source=/dev/null
        . /etc/os-release

        DISTRO_ID="${ID:-unknown}"
        DISTRO_NAME="${NAME:-Inconnue}"

        case "$DISTRO_ID" in
            ubuntu|debian|linuxmint|pop|elementary|neon|zorin|kali|kubuntu)
                DISTRO_FAMILY="debian"
                ;;
            fedora|nobara|ultramarine|rhel|centos|almalinux|rocky)
                DISTRO_FAMILY="fedora"
                ;;
            arch|cachyos|manjaro|endeavouros|garuda|omarchy)
                DISTRO_FAMILY="arch"
                ;;
            opensuse*|suse|opensuse-tumbleweed|opensuse-leap|opensuse-slowroll)
                DISTRO_FAMILY="suse"
                ;;
            nixos|glfos|glf-os)
                DISTRO_FAMILY="nixos"
                ;;
            slackware|salix)
                DISTRO_FAMILY="slackware"
                ;;
            solus)
                DISTRO_FAMILY="solus"
                ;;
            alpine)
                DISTRO_FAMILY="alpine"
                ;;
            void)
                DISTRO_FAMILY="void"
                ;;
            gentoo|funtoo)
                DISTRO_FAMILY="gentoo"
                ;;
            *)
                DISTRO_FAMILY="unknown"
                ;;
        esac

        if [ "$DISTRO_FAMILY" = "unknown" ] && [ -n "${ID_LIKE:-}" ]; then
            case " ${ID_LIKE} " in
                *" debian "*|*" ubuntu "*)   DISTRO_FAMILY="debian" ;;
                *" fedora "*|*" rhel "*)     DISTRO_FAMILY="fedora" ;;
                *" arch "*)                  DISTRO_FAMILY="arch" ;;
                *" suse "*|*" opensuse "*)   DISTRO_FAMILY="suse" ;;
                *" nixos "*)                 DISTRO_FAMILY="nixos" ;;
                *" slackware "*)             DISTRO_FAMILY="slackware" ;;
                *" solus "*)                 DISTRO_FAMILY="solus" ;;
                *" alpine "*)                DISTRO_FAMILY="alpine" ;;
                *" void "*)                  DISTRO_FAMILY="void" ;;
                *" gentoo "*)                DISTRO_FAMILY="gentoo" ;;
            esac

            if [ "$DISTRO_FAMILY" != "unknown" ]; then
                log_info "Distribution dérivée détectée via ID_LIKE (${ID_LIKE}) → famille : $DISTRO_FAMILY"
            fi
        fi
    else
        DISTRO_ID="unknown"
        DISTRO_NAME="Inconnue"
        DISTRO_FAMILY="unknown"
    fi
}

# Session Live (ISO/USB) : la couche d'écriture est limitée (RAM / overlay).
# C'est la cause des échecs « No space left on device » vus sur Fedora Live.
detect_live_session() {
    IS_LIVE_SESSION="0"

    if grep -qE 'rd\.live\.image|boot=live|boot=casper|live-media|rd\.live\.overlay|overlayroot|archisobasedir|root=live:' /proc/cmdline 2>/dev/null; then
        IS_LIVE_SESSION="1"
    fi

    local d
    for d in /run/initramfs/live /run/live /lib/live/mount /run/archiso /run/initramfs/isoscan /iso; do
        if [ -d "$d" ]; then
            IS_LIVE_SESSION="1"
        fi
    done

    case "${USER:-$(id -un 2>/dev/null || true)}" in
        liveuser|live)
            IS_LIVE_SESSION="1"
            ;;
    esac

    if [ "$IS_LIVE_SESSION" = "1" ]; then
        log_info "Environnement Live détecté (système démarré depuis une image ISO/USB)."
    fi

    return 0
}

check_command() {
    command -v "$1" >/dev/null 2>&1
}

prepend_path() {
    local dir="$1"
    [ -d "$dir" ] || return 0
    case ":$PATH:" in
        *":$dir:"*) ;;
        *) export PATH="$dir:$PATH" ;;
    esac
}

get_wine_version() {
    local bin="${WINE_BIN:-}"
    if [ -z "$bin" ]; then
        bin="$(command -v wine 2>/dev/null || true)"
    fi
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        "$bin" --version 2>/dev/null | grep -Eo '[0-9]+([.][0-9]+)+' | head -n1 || echo "0"
    else
        echo "0"
    fi
}

version_lt() {
    local v1="$1"
    local v2="$2"

    [ "$v1" != "$v2" ] && [ "$(printf '%s\n%s\n' "$v1" "$v2" | sort -V | head -n1)" = "$v1" ]
}

# ------------------------------------------------------------------------------
# Exécution bornée dans le temps (évite les blocages Wine : NixOS 32 bits)
# ------------------------------------------------------------------------------

detect_timeout_tool() {
    if [ -n "$TIMEOUT_KIND" ]; then
        return 0
    fi

    if check_command timeout; then
        if timeout --version >/dev/null 2>&1; then
            TIMEOUT_KIND="gnu"
        else
            TIMEOUT_KIND="busybox"
        fi
    else
        TIMEOUT_KIND="none"
    fi

    return 0
}

# run_with_timeout SECONDES commande [args...]
# Renvoie 124 (ou 137) si le délai est dépassé.
# Sans commande « timeout » (systèmes minimaux), un repli interne surveille
# la commande et l'arrête lui-même : aucune attente n'est jamais illimitée.
run_with_timeout() {
    local secs="$1"
    shift

    detect_timeout_tool

    case "$TIMEOUT_KIND" in
        gnu)
            timeout -k 15 "$secs" "$@"
            ;;
        busybox)
            timeout "$secs" "$@"
            ;;
        *)
            local pid waited=0 st=0
            "$@" &
            pid=$!

            while kill -0 "$pid" 2>/dev/null && [ "$waited" -lt "$secs" ]; do
                sleep 1
                waited=$((waited + 1))
            done

            if kill -0 "$pid" 2>/dev/null; then
                kill -TERM "$pid" 2>/dev/null || true
                sleep 2
                kill -KILL "$pid" 2>/dev/null || true
                wait "$pid" 2>/dev/null || true
                return 124
            fi

            wait "$pid" || st=$?
            return "$st"
            ;;
    esac
}

status_is_timeout() {
    local st="$1"
    [ "$st" -eq 124 ] || [ "$st" -eq 137 ]
}

# ------------------------------------------------------------------------------
# Téléchargements : GNU wget, wget2 (Fedora ≥ 40), busybox wget (Alpine), curl
# ------------------------------------------------------------------------------
# wget2 et busybox n'acceptent pas toutes les options de GNU wget (ex. -L).
# Un échec silencieux ici était à l'origine de l'icône « n » sous Fedora.
# ------------------------------------------------------------------------------

wget_flavor() {
    if ! check_command wget; then
        echo "none"
        return 0
    fi

    local v
    v="$(wget --version 2>&1 | head -n1 || true)"

    case "$v" in
        *[Bb]usy[Bb]ox*) echo "busybox" ;;
        *[Ww]get2*)      echo "wget2" ;;
        *)               echo "gnu" ;;
    esac
}

# download_file URL DESTINATION [1 = afficher la progression]
# Les petits fichiers (icône, clés, listes) sont bornés court ; le gros
# téléchargement visible (installateur Pronote) dispose d'une heure.
download_file() {
    local url="$1"
    local dest="$2"
    local visible="${3:-0}"
    local flavor
    local ok=1

    local wget_timeout=20 wget_tries=2 curl_max_time=120
    if [ "$visible" = "1" ]; then
        wget_timeout=30
        wget_tries=3
        curl_max_time=3600
    fi

    flavor="$(wget_flavor)"

    set +e
    case "$flavor" in
        busybox)
            wget -q -T "$wget_timeout" -U "$HTTP_USER_AGENT" -O "$dest" "$url" 2>/dev/null
            ok=$?
            ;;
        gnu|wget2)
            if [ "$visible" = "1" ]; then
                wget --timeout="$wget_timeout" --tries="$wget_tries" --user-agent="$HTTP_USER_AGENT" -O "$dest" "$url"
                ok=$?
            else
                wget -q --timeout="$wget_timeout" --tries="$wget_tries" --user-agent="$HTTP_USER_AGENT" -O "$dest" "$url" 2>/dev/null
                ok=$?
            fi
            ;;
        *)
            ok=1
            ;;
    esac

    if [ "$ok" -ne 0 ] && check_command curl; then
        if [ "$visible" = "1" ]; then
            curl -fL --retry 3 --connect-timeout 30 --max-time "$curl_max_time" -A "$HTTP_USER_AGENT" -o "$dest" "$url"
            ok=$?
        else
            curl -fsSL --retry 2 --connect-timeout 20 --max-time "$curl_max_time" -A "$HTTP_USER_AGENT" -o "$dest" "$url" 2>/dev/null
            ok=$?
        fi
    fi
    set -e

    if [ "$ok" -ne 0 ] || [ ! -s "$dest" ]; then
        rm -f "$dest" 2>/dev/null || true
        return 1
    fi

    return 0
}

# fetch_url_stdout URL  → contenu sur la sortie standard
fetch_url_stdout() {
    local url="$1"
    local flavor
    flavor="$(wget_flavor)"

    case "$flavor" in
        busybox)
            wget -q -T 30 -U "$HTTP_USER_AGENT" -O - "$url" 2>/dev/null && return 0
            ;;
        gnu|wget2)
            wget -q --timeout=30 --tries=2 --user-agent="$HTTP_USER_AGENT" -O - "$url" 2>/dev/null && return 0
            ;;
    esac

    if check_command curl; then
        curl -fsSL --connect-timeout 30 --max-time 120 -A "$HTTP_USER_AGENT" "$url" 2>/dev/null && return 0
    fi

    return 1
}

# url_exists URL  → 0 si la ressource répond (HEAD / spider)
url_exists() {
    local url="$1"
    local flavor
    flavor="$(wget_flavor)"

    case "$flavor" in
        busybox)
            wget -q --spider -T 20 -U "$HTTP_USER_AGENT" "$url" >/dev/null 2>&1 && return 0
            ;;
        gnu|wget2)
            wget -q --spider --timeout=20 --tries=2 --user-agent="$HTTP_USER_AGENT" "$url" >/dev/null 2>&1 && return 0
            ;;
    esac

    if check_command curl; then
        curl -fsIL --connect-timeout 20 -A "$HTTP_USER_AGENT" "$url" >/dev/null 2>&1 && return 0
        # Certains CDN refusent les requêtes HEAD : un GET partiel suffit
        # alors à prouver que la ressource existe (fichiers .sources WineHQ).
        curl -fsL --connect-timeout 20 --max-time 40 -r 0-31 \
            -A "$HTTP_USER_AGENT" "$url" >/dev/null 2>&1 && return 0
    fi

    return 1
}

# ------------------------------------------------------------------------------
# Espace disque
# ------------------------------------------------------------------------------

# free_mb_at CHEMIN → Mo libres sur le système de fichiers (vide si inconnu)
free_mb_at() {
    local path="$1"

    while [ -n "$path" ] && [ ! -d "$path" ] && [ "$path" != "/" ]; do
        path="$(dirname "$path")"
    done

    local mb
    mb="$(df -Pm "$path" 2>/dev/null | awk 'NR==2 {print $4}' || true)"

    case "$mb" in
        ''|*[!0-9]*) echo "" ;;
        *)           echo "$mb" ;;
    esac
}

fmt_mb() {
    local mb="$1"

    if [ -z "$mb" ]; then
        echo "inconnu"
        return 0
    fi

    if [ "$mb" -ge 1024 ]; then
        awk -v m="$mb" 'BEGIN { printf "%.1f Go", m / 1024 }'
    else
        echo "${mb} Mo"
    fi
}

print_disk_hint_if_low() {
    local mb
    mb="$(free_mb_at "$HOME")"

    if [ -n "$mb" ] && [ "$mb" -lt 500 ]; then
        log_error "Espace disque quasiment nul dans $HOME ($(fmt_mb "$mb")) : c'est très probablement la cause."
        if [ "$IS_LIVE_SESSION" = "1" ]; then
            echo "Session Live : la couche d'écriture (RAM) est pleine. Installez la distribution"
            echo "sur disque, ou démarrez le Live avec plus de RAM / un overlay persistant."
        fi
    fi

    return 0
}

# Avant l'installation de Wine : avertissement (avec confirmation) si l'espace
# est faible. Bloquant uniquement si l'utilisateur refuse de continuer.
check_disk_space_before_wine() {
    local sys_path="/usr"
    if [ -d /nix/store ]; then
        sys_path="/nix/store"
    fi

    local sys_mb home_mb tmp_mb
    sys_mb="$(free_mb_at "$sys_path")"
    home_mb="$(free_mb_at "$HOME")"
    tmp_mb="$(free_mb_at "$TMP_DIR")"

    log_info "Espace disque libre : système $(fmt_mb "$sys_mb") | dossier personnel $(fmt_mb "$home_mb") | /tmp $(fmt_mb "$tmp_mb")"

    if [ "$IS_LIVE_SESSION" = "1" ]; then
        log_warning "Session Live : la couche d'écriture (RAM/overlay) est limitée."
        echo "  Wine et ses dépendances (~1,5 Go), le préfixe Wine (~0,7 Go) et l'installateur"
        echo "  Pronote (~0,3 Go) doivent tenir dans cet espace. Un manque de place provoque"
        echo "  des erreurs « No space left on device » / « error=112 » et un Wine inutilisable."
    fi

    local wine_present=0
    if wine_is_available; then
        wine_present=1
    fi

    local -a problems=()

    if [ -n "$sys_mb" ] && [ "$wine_present" -eq 0 ] && [ "$sys_mb" -lt "$DISK_WARN_SYSTEM_MB" ]; then
        problems+=("système ($sys_path) : $(fmt_mb "$sys_mb") libres, ${DISK_WARN_SYSTEM_MB} Mo recommandés pour installer Wine")
    fi
    if [ -n "$home_mb" ] && [ "$home_mb" -lt "$DISK_WARN_HOME_MB" ]; then
        problems+=("dossier personnel : $(fmt_mb "$home_mb") libres, ${DISK_WARN_HOME_MB} Mo recommandés (préfixe Wine + Pronote)")
    fi
    if [ -n "$tmp_mb" ] && [ "$tmp_mb" -lt "$DISK_MIN_TMP_MB" ]; then
        problems+=("/tmp : $(fmt_mb "$tmp_mb") libres, ${DISK_MIN_TMP_MB} Mo nécessaires au téléchargement")
    fi

    if [ "${#problems[@]}" -eq 0 ]; then
        return 0
    fi

    echo ""
    log_warning "Espace disque potentiellement insuffisant :"
    local p
    for p in "${problems[@]}"; do
        echo "  • $p"
    done
    echo ""

    if [ "${PRONOTE_IGNORE_DISK:-0}" = "1" ] || [ "$ASSUME_YES" = "1" ]; then
        log_warning "Poursuite malgré l'avertissement (PRONOTE_IGNORE_DISK / --yes)."
        return 0
    fi

    local answer
    answer="$(ask_user "Continuer malgré tout ? [o/N] : " "N")"
    case "$answer" in
        o|O|oui|Oui|OUI|y|Y|yes|YES)
            log_warning "Poursuite à vos risques : surveillez les messages d'erreur d'espace disque."
            return 0
            ;;
    esac

    log_error "Installation interrompue : libérez de l'espace (ou installez la distribution sur disque) puis relancez."
    exit 1
}

# Avant le préfixe / Pronote : blocage net si l'échec est certain.
check_disk_space_before_pronote() {
    local home_mb tmp_mb
    home_mb="$(free_mb_at "$WINEPREFIX")"
    tmp_mb="$(free_mb_at "$TMP_DIR")"

    local fatal=0

    if [ -n "$home_mb" ] && [ "$home_mb" -lt "$DISK_MIN_HOME_MB" ]; then
        log_error "Espace insuffisant pour Pronote dans $(dirname "$WINEPREFIX") : $(fmt_mb "$home_mb") libres (minimum ${DISK_MIN_HOME_MB} Mo)."
        fatal=1
    fi
    if [ -n "$tmp_mb" ] && [ "$tmp_mb" -lt "$DISK_MIN_TMP_MB" ]; then
        log_error "Espace insuffisant dans /tmp pour télécharger l'installateur : $(fmt_mb "$tmp_mb") libres (minimum ${DISK_MIN_TMP_MB} Mo)."
        fatal=1
    fi

    if [ "$fatal" -eq 1 ] && [ "${PRONOTE_IGNORE_DISK:-0}" != "1" ]; then
        echo "L'installation échouerait avec des erreurs « No space left on device »."
        if [ "$IS_LIVE_SESSION" = "1" ]; then
            echo "Session Live : installez la distribution sur disque, ou démarrez avec plus de RAM."
        fi
        echo "Libérez de l'espace puis relancez (ou PRONOTE_IGNORE_DISK=1 pour forcer)."
        exit 1
    fi

    return 0
}

# ------------------------------------------------------------------------------
# Résolution des binaires Wine (erreur 127 / wineserver not found)
# ------------------------------------------------------------------------------
# WineHQ 11 sous Ubuntu 26.04 installe :
#   /opt/wine-stable/bin/wine
#   /opt/wine-stable/bin/wineboot
#   /opt/wine-stable/bin/wineserver
# sans garantir les liens dans /usr/bin. winetricks cherche wineserver dans
# le PATH. On reconstitue donc un PATH cohérent, et on mémorise les chemins.
# ------------------------------------------------------------------------------

discover_wine_paths() {
    local dir candidate resolved wine_dir

    for dir in \
        /opt/wine-stable/bin \
        /opt/wine-devel/bin \
        /opt/wine-staging/bin \
        /usr/lib/wine \
        /usr/lib64/wine \
        "$HOME/.local/bin" \
        /usr/local/bin \
        /usr/bin
    do
        prepend_path "$dir"
    done

    hash -r 2>/dev/null || true

    WINE_BIN=""
    WINEBOOT_BIN=""
    WINECFG_BIN=""
    WINESERVER_BIN=""

    for candidate in wine wine64; do
        if check_command "$candidate"; then
            WINE_BIN="$(command -v "$candidate")"
            break
        fi
    done

    if [ -z "$WINE_BIN" ]; then
        for candidate in \
            /opt/wine-stable/bin/wine \
            /opt/wine-devel/bin/wine \
            /opt/wine-staging/bin/wine \
            /usr/bin/wine \
            /usr/bin/wine64
        do
            if [ -x "$candidate" ]; then
                WINE_BIN="$candidate"
                prepend_path "$(dirname "$candidate")"
                break
            fi
        done
    fi

    if [ -n "$WINE_BIN" ]; then
        resolved="$(readlink -f "$WINE_BIN" 2>/dev/null || echo "$WINE_BIN")"
        wine_dir="$(dirname "$resolved")"
        prepend_path "$wine_dir"

        [ -x "$wine_dir/wineboot" ]    && WINEBOOT_BIN="$wine_dir/wineboot"
        [ -x "$wine_dir/winecfg" ]     && WINECFG_BIN="$wine_dir/winecfg"
        [ -x "$wine_dir/wineserver" ]  && WINESERVER_BIN="$wine_dir/wineserver"
    fi

    if [ -z "$WINEBOOT_BIN" ]   && check_command wineboot;   then WINEBOOT_BIN="$(command -v wineboot)"; fi
    if [ -z "$WINECFG_BIN" ]    && check_command winecfg;    then WINECFG_BIN="$(command -v winecfg)"; fi
    if [ -z "$WINESERVER_BIN" ] && check_command wineserver; then WINESERVER_BIN="$(command -v wineserver)"; fi

    # Debian / Ubuntu / Mint (Wine des dépôts) : wineserver n'est volontairement
    # pas exposé dans /usr/bin mais rangé dans /usr/lib/<multiarch>/wine/.
    # Sans lui, aucune attente de fin de session n'était possible : c'était la
    # cause du « préfixe Wine incomplet » au premier lancement sous Kubuntu.
    if [ -z "$WINESERVER_BIN" ]; then
        for candidate in \
            "${wine_dir:-/nonexistent}/wineserver" \
            "${wine_dir:-/nonexistent}/wineserver64" \
            /usr/lib/x86_64-linux-gnu/wine/wineserver \
            /usr/lib/x86_64-linux-gnu/wine/x86_64-unix/wineserver \
            /usr/lib/wine/wineserver64 \
            /usr/lib/wine/wineserver \
            /usr/lib64/wine/wineserver \
            /usr/lib64/wine/x86_64-unix/wineserver \
            /usr/libexec/wine/wineserver \
            /usr/lib/i386-linux-gnu/wine/wineserver \
            /usr/lib/wine/wineserver32 \
            /opt/wine-stable/bin/wineserver \
            /opt/wine-devel/bin/wineserver \
            /opt/wine-staging/bin/wineserver
        do
            if [ -x "$candidate" ]; then
                WINESERVER_BIN="$candidate"
                break
            fi
        done
    fi

    # Le wrapper « wine » (script shell) peut mentionner le chemin du serveur.
    if [ -z "$WINESERVER_BIN" ] && [ -n "${resolved:-}" ] && [ -f "$resolved" ] \
        && head -c 2 "$resolved" 2>/dev/null | grep -q '#!'; then
        candidate="$(grep -oE '/[A-Za-z0-9_./-]*wineserver[0-9]*' "$resolved" 2>/dev/null | head -n1 || true)"
        if [ -n "$candidate" ] && [ -x "$candidate" ]; then
            WINESERVER_BIN="$candidate"
        fi
    fi

    # Dernier recours : recherche bornée dans les emplacements système.
    if [ -z "$WINESERVER_BIN" ]; then
        candidate="$(find /usr/lib /usr/lib64 /usr/libexec /opt -maxdepth 4 -type f -name 'wineserver*' -perm -u+x 2>/dev/null | head -n1 || true)"
        if [ -n "$candidate" ] && [ -x "$candidate" ]; then
            WINESERVER_BIN="$candidate"
        fi
    fi

    hash -r 2>/dev/null || true

    if [ -n "$WINE_BIN" ]; then
        export WINE="$WINE_BIN"
    fi
    if [ -n "$WINESERVER_BIN" ]; then
        export WINESERVER="$WINESERVER_BIN"
    fi

    return 0
}

wine_is_available() {
    discover_wine_paths
    [ -n "$WINE_BIN" ] && [ -x "$WINE_BIN" ]
}

# Lance wineboot même si le wrapper n'est pas dans le PATH.
# Wine 11 : « wine wineboot --init » fonctionne (wineboot.exe est un builtin).
run_wineboot_tool() {
    if [ -n "${WINEBOOT_BIN:-}" ] && [ -x "$WINEBOOT_BIN" ]; then
        "$WINEBOOT_BIN" "$@"
        return $?
    fi
    if [ -n "${WINE_BIN:-}" ] && [ -x "$WINE_BIN" ]; then
        "$WINE_BIN" wineboot "$@"
        return $?
    fi
    log_error "wineboot introuvable (ni wrapper, ni binaire wine)."
    return 127
}

run_winecfg_tool() {
    if [ -n "${WINECFG_BIN:-}" ] && [ -x "$WINECFG_BIN" ]; then
        "$WINECFG_BIN" "$@"
        return $?
    fi
    if [ -n "${WINE_BIN:-}" ] && [ -x "$WINE_BIN" ]; then
        "$WINE_BIN" winecfg "$@"
        return $?
    fi
    log_warning "winecfg introuvable."
    return 1
}

run_wineserver_tool() {
    if [ -n "${WINESERVER_BIN:-}" ] && [ -x "$WINESERVER_BIN" ]; then
        "$WINESERVER_BIN" "$@"
        return $?
    fi
    if check_command wineserver; then
        wineserver "$@"
        return $?
    fi
    return 0
}

run_wine_bin() {
    if [ -n "${WINE_BIN:-}" ] && [ -x "$WINE_BIN" ]; then
        "$WINE_BIN" "$@"
        return $?
    fi
    wine "$@"
}

# ------------------------------------------------------------------------------
# Fin de session Wine : attente courte, puis fermeture rapide et sûre
# ------------------------------------------------------------------------------
# Le registre Wine (system.reg, user.reg) n'est écrit sur disque que par le
# wineserver : périodiquement et à son arrêt. Deux conséquences :
#   • après « wineboot --init », il faut attendre l'arrêt du wineserver pour
#     que system.reg existe (sinon « préfixe incomplet », vu sous Kubuntu) ;
#   • après l'installateur Pronote, le service de mise à jour Index Éducation
#     reste actif : le wineserver ne s'arrête jamais de lui-même. Attendre
#     (v4.7 : 5 minutes) est inutile. « wineserver -k » envoie SIGINT au
#     wineserver, qui sauvegarde le registre puis arrête ses processus.
# Les attentes sont bornées, visibles, et fonctionnent même lorsque le
# binaire wineserver reste introuvable (observation de /proc).
# ------------------------------------------------------------------------------

# prefix_wine_pids [MOTIF]
# PID des processus Wine (wineserver, *.exe) rattachés au préfixe courant,
# identifiés par WINEPREFIX dans leur environnement (même utilisateur).
# Les commandes de service du script (wineserver -w/-k, timeout) sont exclues.
prefix_wine_pids() {
    local pattern="${1:-}"
    local d cmd
    local -a parts=()

    for d in /proc/[0-9]*; do
        [ -r "$d/cmdline" ] || continue
        parts=()
        mapfile -d '' parts < "$d/cmdline" 2>/dev/null || continue
        cmd="${parts[*]:-}"
        [ -n "$cmd" ] || continue

        case "$cmd" in
            *timeout*)                 continue ;;
            *wineserver\ -[wkp]*)      continue ;;
            *wineserver*|*.exe*|*.EXE*) ;;
            *)                         continue ;;
        esac

        if [ -n "$pattern" ]; then
            case "$cmd" in
                *"$pattern"*) ;;
                *) continue ;;
            esac
        fi

        if tr '\0' '\n' < "$d/environ" 2>/dev/null | grep -qxF "WINEPREFIX=$WINEPREFIX"; then
            echo "${d#/proc/}"
        fi
    done

    return 0
}

wine_session_alive() {
    [ -n "$(prefix_wine_pids)" ]
}

# close_wine_session SECONDES [MODE]
#   Attend au plus SECONDES que la session Wine du préfixe se termine seule
#   (wineserver -w, ou observation de /proc sans binaire wineserver).
#   MODE=kill (défaut) : passé ce délai, wineserver -k (registre sauvegardé,
#                        processus arrêtés), puis SIGKILL en ultime recours.
#   MODE=soft          : passé ce délai, on continue sans rien fermer.
close_wine_session() {
    local secs="${1:-20}"
    local mode="${2:-kill}"
    local bin="${WINESERVER_BIN:-}"

    if [ -z "$bin" ]; then
        bin="$(command -v wineserver 2>/dev/null || true)"
    fi

    local st=0 waited=0 pid i

    if [ -n "$bin" ]; then
        set +e
        run_with_timeout "$secs" "$bin" -w >/dev/null 2>&1
        st=$?
        set -e
        if ! status_is_timeout "$st"; then
            return 0
        fi
        echo "wineserver encore actif après ${secs}s (mode $mode)."
    else
        while [ "$waited" -lt "$secs" ] && wine_session_alive; do
            sleep 1
            waited=$((waited + 1))
        done
        if ! wine_session_alive; then
            return 0
        fi
        echo "Processus Wine encore actifs après ${secs}s, wineserver introuvable (mode $mode)."
    fi

    if [ "$mode" = "soft" ]; then
        return 0
    fi

    # Fermeture : SIGINT au wineserver → sauvegarde du registre + arrêt des clients
    if [ -n "$bin" ]; then
        run_with_timeout 30 "$bin" -k >/dev/null 2>&1 || true
    else
        local -a ws_pids=()
        mapfile -t ws_pids < <(prefix_wine_pids wineserver)

        if [ "${#ws_pids[@]}" -gt 0 ]; then
            for pid in "${ws_pids[@]}"; do
                kill -INT "$pid" 2>/dev/null || true
            done
        else
            # Aucun wineserver : les processus restants sont orphelins, arrêt direct
            for pid in $(prefix_wine_pids); do
                kill -TERM "$pid" 2>/dev/null || true
            done
            sleep 1
        fi
    fi

    # Attente de la disparition effective (≤ 15 s), puis arrêt brutal
    for ((i = 0; i < 30; i++)); do
        wine_session_alive || return 0
        sleep 0.5
    done

    for pid in $(prefix_wine_pids); do
        kill -KILL "$pid" 2>/dev/null || true
    done
    sleep 1

    return 0
}

# Attente douce et silencieuse (rien n'est fermé si le délai est dépassé).
settle_wine_session() {
    close_wine_session "${1:-15}" soft
}

# Version visible : barre animée + consigne de ne pas fermer la fenêtre.
# finish_wine_session SECONDES [LIBELLÉ] [MODE]
finish_wine_session() {
    local secs="${1:-20}"
    local label="${2:-Fermeture des processus Wine}"
    local mode="${3:-kill}"

    RUN_QUIET_ACTIVITY_DIR=""
    RUN_QUIET_FINALIZE_MSG=""

    run_quiet "$label" close_wine_session "$secs" "$mode" || true
}

print_wine_diagnostics() {
    echo ""
    echo "Diagnostic Wine :"
    echo "  PATH        = $PATH"
    echo "  wine        = ${WINE_BIN:-introuvable}"
    echo "  wineboot    = ${WINEBOOT_BIN:-introuvable (repli : wine wineboot)}"
    echo "  winecfg     = ${WINECFG_BIN:-introuvable}"
    echo "  wineserver  = ${WINESERVER_BIN:-introuvable}"
    echo "  version     = $(get_wine_version)"
    echo "  WINEPREFIX  = $WINEPREFIX"
    echo "  WINEARCH    = ${WINEARCH:-non défini}"
    echo "  mode préfixe= $PREFIX_MODE"
    echo "  espace libre= $(fmt_mb "$(free_mb_at "$WINEPREFIX")") (préfixe) / $(fmt_mb "$(free_mb_at /tmp)") (/tmp)"
    echo ""
}

# ------------------------------------------------------------------------------
# winetricks autonome (GitHub) — évite le paquet Debian qui dépend de « wine »
# ------------------------------------------------------------------------------

install_winetricks_standalone() {
    log_info "Installation autonome de winetricks (script officiel Winetricks)..."
    mkdir -p "$BIN_DIR"
    prepend_path "$BIN_DIR"

    local wt="$BIN_DIR/winetricks"
    local url="https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks"

    if ! download_file "$url" "$wt"; then
        log_warning "Téléchargement de winetricks impossible."
        return 1
    fi

    chmod +x "$wt"

    if ! head -n1 "$wt" | grep -qE '^#!' || ! grep -q "winetricks" "$wt"; then
        log_error "Le fichier winetricks téléchargé est invalide."
        rm -f "$wt"
        return 1
    fi

    hash -r 2>/dev/null || true
    log_success "winetricks installé dans $wt"
    return 0
}

# Garantit un winetricks utilisable : paquet de la distribution s'il est
# récent, sinon version autonome dans ~/.local/bin (prioritaire dans le PATH).
ensure_winetricks() {
    prepend_path "$BIN_DIR"
    hash -r 2>/dev/null || true

    if check_command winetricks; then
        local wt_ver=""
        wt_ver="$(winetricks --version 2>/dev/null | grep -oE '20[0-9]{6}' | head -n1 || true)"

        if [ -n "$wt_ver" ] && [ "$wt_ver" -lt 20230101 ] 2>/dev/null; then
            log_warning "winetricks trop ancien ($wt_ver) pour Wine récent : installation d'une version à jour."
            install_winetricks_standalone || log_warning "winetricks conservé dans sa version actuelle."
        else
            log_success "winetricks disponible : $(command -v winetricks)${wt_ver:+ (version $wt_ver)}"
        fi
        return 0
    fi

    if ! install_winetricks_standalone; then
        log_warning "winetricks indisponible : les composants Windows optionnels (polices) seront ignorés."
    fi

    return 0
}

# ------------------------------------------------------------------------------
# Version Windows adaptée à la famille de distribution
# ------------------------------------------------------------------------------
# Sur Debian/Ubuntu/Mint, Wine est souvent livré en paquets séparés
# (wine32/wine64) et, avec « Windows 11 », l'installateur Index Éducation
# refuse de s'installer. Windows 10 contourne le refus.
# Arch / Fedora / SUSE / Solus / Void / Alpine / NixOS conservent win11.
# ------------------------------------------------------------------------------

set_target_windows_version() {
    case "$DISTRO_FAMILY" in
        debian)
            WIN_VERSION="win10"
            log_info "Famille Debian détectée : la version Windows annoncée sera Windows 10."
            log_info "(contournement du refus d'installation 32/64 bits sur Debian/Ubuntu/Mint)"
            ;;
        *)
            WIN_VERSION="win11"
            ;;
    esac
}

win_version_label() {
    case "$WIN_VERSION" in
        win10) echo "Windows 10" ;;
        win11) echo "Windows 11" ;;
        *)     echo "$WIN_VERSION" ;;
    esac
}

win_version_product_name() {
    case "$WIN_VERSION" in
        win10) echo "Windows 10 Pro" ;;
        *)     echo "Windows 11 Pro" ;;
    esac
}

win_version_build() {
    case "$WIN_VERSION" in
        win10) echo "19045" ;;
        *)     echo "22000" ;;
    esac
}

detect_prefix_arch_at() {
    local prefix="$1"

    if [ ! -d "$prefix" ]; then
        echo "none"
        return
    fi

    if [ -f "$prefix/system.reg" ]; then
        if grep -qE '^#arch=win64' "$prefix/system.reg" 2>/dev/null; then
            echo "64"
            return
        fi
        if grep -qE '^#arch=win32' "$prefix/system.reg" 2>/dev/null; then
            echo "32"
            return
        fi
    fi

    echo "64"
}

detect_prefix_arch() {
    detect_prefix_arch_at "$WINEPREFIX"
}

# ------------------------------------------------------------------------------
# Préfixes séparés par architecture
# ------------------------------------------------------------------------------
# 64 bits : ~/.local/share/wineprefixes/pronote-ANNEE  (chemin historique)
# 32 bits : ~/.local/share/wineprefixes/pronote-ANNEE-32
# Même lorsque Wine refuse WINEARCH=win32 (mode WoW64), le préfixe 32 bits
# reste un dossier distinct : les deux clients coexistent.
# ------------------------------------------------------------------------------

set_wineprefix() {
    if [ -n "$WINEPREFIX_USER" ]; then
        WINEPREFIX="$WINEPREFIX_USER"
    else
        local base="$HOME/.local/share/wineprefixes/pronote-$PRONOTE_YEAR"

        if [ "$PRONOTE_ARCH" = "32" ]; then
            if [ ! -d "${base}-32" ] && [ "$(detect_prefix_arch_at "$base")" = "32" ]; then
                WINEPREFIX="$base"
            else
                WINEPREFIX="${base}-32"
            fi
        else
            WINEPREFIX="$base"
        fi
    fi

    export WINEPREFIX
    mkdir -p "$(dirname "$WINEPREFIX")"
}

# ------------------------------------------------------------------------------
# Résolution des conflits de préfixe (aucune question posée)
# ------------------------------------------------------------------------------
# prefix_is_reusable CHEMIN ARCH : 0 si le chemin peut accueillir un préfixe de
# l'architecture demandée, sans jamais écraser ce qui existe.
# ------------------------------------------------------------------------------

prefix_is_reusable() {
    local prefix="$1"
    local arch="$2"

    if [ ! -e "$prefix" ]; then
        return 0                                    # chemin libre
    fi

    if [ -d "$prefix" ] && [ -z "$(ls -A "$prefix" 2>/dev/null)" ]; then
        return 0                                    # dossier vide
    fi

    if [ ! -f "$prefix/system.reg" ] && [ ! -d "$prefix/drive_c" ]; then
        return 1                                    # pas un préfixe Wine : on n'y touche pas
    fi

    [ "$(detect_prefix_arch_at "$prefix")" = "$arch" ]
}

# choose_free_prefix ARCH BASE → premier chemin utilisable (toujours trouvé)
choose_free_prefix() {
    local arch="$1"
    local base="$2"
    local -a candidates=()
    local candidate=""
    local n=2

    if [ "$arch" = "32" ]; then
        candidates=("${base}-32" "$base")
    else
        candidates=("$base" "${base}-64")
    fi

    for candidate in "${candidates[@]}"; do
        if prefix_is_reusable "$candidate" "$arch"; then
            echo "$candidate"
            return 0
        fi
    done

    while :; do
        candidate="${base}-${arch}-${n}"
        if prefix_is_reusable "$candidate" "$arch"; then
            echo "$candidate"
            return 0
        fi
        n=$((n + 1))
    done
}

# Un préfixe existant de même architecture est réutilisé tel quel.
# S'il contient l'autre architecture, le script choisit lui-même un autre
# chemin : il ne déplace ni ne supprime jamais un préfixe (ce déplacement
# automatique détruisait la version déjà installée sous NixOS).
resolve_prefix_conflict() {
    if [ ! -f "$WINEPREFIX/system.reg" ]; then
        return 0
    fi

    local existing_arch
    existing_arch="$(detect_prefix_arch)"
    log_info "Préfixe Wine existant détecté : $WINEPREFIX (${existing_arch} bits)"

    if [ "$existing_arch" = "$PRONOTE_ARCH" ]; then
        log_info "Architecture identique : le préfixe existant est réutilisé."
        return 0
    fi

    local base="$HOME/.local/share/wineprefixes/pronote-$PRONOTE_YEAR"
    local alt
    alt="$(choose_free_prefix "$PRONOTE_ARCH" "$base")"

    if [ -z "$alt" ] || [ "$alt" = "$WINEPREFIX" ]; then
        log_error "Aucun préfixe disponible pour Pronote ${PRONOTE_ARCH} bits."
        echo "Le préfixe $WINEPREFIX contient la version ${existing_arch} bits."
        exit 1
    fi

    log_warning "Le préfixe $WINEPREFIX contient Pronote ${existing_arch} bits : il est conservé tel quel."
    log_info "Pronote ${PRONOTE_ARCH} bits utilisera un préfixe séparé : $alt"

    WINEPREFIX="$alt"
    export WINEPREFIX
    mkdir -p "$(dirname "$WINEPREFIX")"
    return 0
}

# ------------------------------------------------------------------------------
# Sondage WINEARCH=win32 (Wine 11 / nouveau WoW64)
# ------------------------------------------------------------------------------

probe_win32_prefix_support() {
    if [ "$WINE_WIN32_PREFIX_OK" = "1" ]; then
        return 0
    fi
    if [ "$WINE_WIN32_PREFIX_OK" = "0" ]; then
        return 1
    fi

    discover_wine_paths
    if [ -z "$WINE_BIN" ]; then
        WINE_WIN32_PREFIX_OK="0"
        return 1
    fi

    log_info "Vérification du support des préfixes Wine 32 bits natifs (WINEARCH=win32)..."

    local test_prefix="$TMP_DIR/wine-win32-probe"
    rm -rf "$test_prefix"
    mkdir -p "$test_prefix"

    local out=""
    local status=0

    set +e
    out="$(
        export WINEPREFIX="$test_prefix"
        export WINEARCH="win32"
        export WINEDEBUG="-all"
        export WINEDLLOVERRIDES="mscoree,mshtml,winemenubuilder.exe="
        if [ -n "${WINEBOOT_BIN:-}" ] && [ -x "$WINEBOOT_BIN" ]; then
            run_with_timeout 600 "$WINEBOOT_BIN" --init 2>&1
        else
            run_with_timeout 600 "$WINE_BIN" wineboot --init 2>&1
        fi
    )"
    status=$?

    (
        export WINEPREFIX="$test_prefix"
        export WINEARCH="win32"
        run_with_timeout 30 "${WINESERVER_BIN:-wineserver}" -k >/dev/null 2>&1
    ) || true
    set -e

    rm -rf "$test_prefix" 2>/dev/null || true

    if grep -qi "not supported in wow64" <<< "$out"; then
        WINE_WIN32_PREFIX_OK="0"
        log_info "Wine fonctionne en mode WoW64 : les préfixes WINEARCH=win32 ne sont pas supportés."
        log_info "Pronote 32 bits sera installé dans un préfixe WoW64 dédié, séparé du 64 bits."
        return 1
    fi

    if [ "$status" -ne 0 ]; then
        WINE_WIN32_PREFIX_OK="0"
        log_warning "Création d'un préfixe 32 bits natif impossible (code $status) : repli sur un préfixe WoW64 dédié."
        return 1
    fi

    WINE_WIN32_PREFIX_OK="1"
    log_success "Préfixes Wine 32 bits natifs supportés : Pronote 32 bits utilisera WINEARCH=win32."
    return 0
}

# ==============================================================================
# Version Pronote : détection en ligne, choix 32/64 bits, URL personnalisée
# ==============================================================================

check_latest_version() {
    log_info "Vérification de la dernière version de Pronote disponible..."

    local page=""
    page="$(fetch_url_stdout "$PRONOTE_DOWNLOAD_PAGE" || true)"

    if [ -z "$page" ]; then
        log_warning "Impossible de vérifier la dernière version (site inaccessible). Version par défaut conservée : $DEFAULT_VERSION"
        return 0
    fi

    local found_url64="" found_url32="" found_ver=""

    found_url64="$(grep -oE 'https?://[A-Za-z0-9./_-]*Install_PRNclient_FR_[0-9]+\.[0-9]+\.[0-9]+_win64\.exe' <<< "$page" | head -n1 || true)"
    found_url32="$(grep -oE 'https?://[A-Za-z0-9./_-]*Install_PRNclient_FR_[0-9]+\.[0-9]+\.[0-9]+_win32\.exe' <<< "$page" | head -n1 || true)"

    if [ -n "$found_url64" ]; then
        found_ver="$(sed -E 's/.*Install_PRNclient_FR_([0-9]+\.[0-9]+\.[0-9]+)_win64\.exe.*/\1/' <<< "$found_url64")"
    else
        # Nom de fichier sans URL complète
        local fname=""
        fname="$(grep -oE 'Install_PRNclient_FR_[0-9]+\.[0-9]+\.[0-9]+_win64\.exe' <<< "$page" | head -n1 || true)"
        if [ -n "$fname" ]; then
            found_ver="$(sed -E 's/Install_PRNclient_FR_([0-9]+\.[0-9]+\.[0-9]+)_win64\.exe/\1/' <<< "$fname")"
        fi
    fi

    if [ -z "$found_ver" ]; then
        # Texte de la page : « CLIENT PRONOTE 2026 - 2.6 »
        local txt="" year="" minor=""
        txt="$(grep -oE 'CLIENT PRONOTE[[:space:]]+[0-9]{4}[[:space:]]*-[[:space:]]*[0-9]+\.[0-9]+' <<< "$page" | head -n1 || true)"
        if [ -n "$txt" ]; then
            year="$(grep -oE '[0-9]{4}' <<< "$txt" | head -n1 || true)"
            minor="$(grep -oE '[0-9]+\.[0-9]+$' <<< "$txt" | head -n1 || true)"
            if [ -n "$year" ] && [ -n "$minor" ]; then
                found_ver="${year}.${minor}"
            fi
        fi
    fi

    if [ -z "$found_ver" ]; then
        log_warning "Version non détectée sur la page de téléchargement ; version par défaut conservée : $DEFAULT_VERSION"
        return 0
    fi

    if [ "$found_ver" = "$DEFAULT_VERSION" ]; then
        log_success "Version Pronote à jour : $found_ver"
        if [ -n "$found_url64" ]; then URL_64="$found_url64"; fi
        if [ -n "$found_url32" ]; then URL_32="$found_url32"; fi
        DEFAULT_URL="$URL_64"
        return 0
    fi

    log_info "Version détectée sur le site : $found_ver (version intégrée au script : $DEFAULT_VERSION)"
    PRONOTE_VERSION="$found_ver"
    PRONOTE_YEAR="${found_ver%%.*}"

    if [ -n "$found_url64" ]; then
        URL_64="$found_url64"
    else
        URL_64="https://tele3.index-education.com/telechargement/pn/v${PRONOTE_YEAR}.0/exe/Install_PRNclient_FR_${PRONOTE_VERSION}_win64.exe"
    fi
    if [ -n "$found_url32" ]; then
        URL_32="$found_url32"
    else
        URL_32="https://tele3.index-education.com/telechargement/pn/v${PRONOTE_YEAR}.0/exe/Install_PRNclient_FR_${PRONOTE_VERSION}_win32.exe"
    fi
    DEFAULT_URL="$URL_64"

    log_success "Le script utilisera Pronote $PRONOTE_VERSION."
    return 0
}

get_pronote_url() {
    echo ""
    box_title "Choix de la version de Pronote" 46
    echo ""

    if [ -n "$PRONOTE_ARCH_FROM_OPTION" ]; then
        PRONOTE_ARCH="$PRONOTE_ARCH_FROM_OPTION"
        log_info "Architecture imposée par --arch : $PRONOTE_ARCH bits"
    else
        echo "Quelle version de Pronote Client souhaitez-vous installer ?"
        echo ""
        echo "  1) 64 bits (recommandé)"
        echo "  2) 32 bits"
        echo ""
        echo "Les deux versions peuvent être installées l'une après l'autre :"
        echo "chacune possède son propre préfixe Wine et son propre désinstallateur."
        echo ""

        local choice
        choice="$(ask_user "Votre choix [1/2] [Entrée=1] : " "1")"

        case "$choice" in
            2|32)
                PRONOTE_ARCH="32"
                ;;
            *)
                PRONOTE_ARCH="64"
                ;;
        esac
    fi

    if [ "$PRONOTE_ARCH" = "32" ]; then
        PRONOTE_URL="$URL_32"
    else
        PRONOTE_URL="$URL_64"
    fi

    log_info "Version retenue : Pronote $PRONOTE_VERSION - $PRONOTE_ARCH bits"
}

process_custom_url() {
    PRONOTE_URL="$PRONOTE_URL_FROM_OPTION"
    log_info "URL personnalisée : $PRONOTE_URL"

    local fname
    fname="$(basename "$PRONOTE_URL")"

    if [ -n "$PRONOTE_ARCH_FROM_OPTION" ]; then
        PRONOTE_ARCH="$PRONOTE_ARCH_FROM_OPTION"
    else
        case "$fname" in
            *win32*|*x86.exe|*_32*)
                PRONOTE_ARCH="32"
                ;;
            *)
                PRONOTE_ARCH="64"
                ;;
        esac
    fi

    local ver=""
    ver="$(grep -oE '[0-9]{4}\.[0-9]+\.[0-9]+' <<< "$fname" | head -n1 || true)"
    if [ -n "$ver" ]; then
        PRONOTE_VERSION="$ver"
        PRONOTE_YEAR="${ver%%.*}"
    fi

    log_info "Architecture : $PRONOTE_ARCH bits — version : $PRONOTE_VERSION"
}

# ------------------------------------------------------------------------------
# apt / dpkg entièrement non interactif
# ------------------------------------------------------------------------------
# DEBIAN_FRONTEND=noninteractive ne suffit PAS : la question sur les fichiers
# de configuration
#     « *** settings.ini.real (Y/I/N/O/D/Z) [défaut=N] ? »
# est posée par dpkg lui-même (pas par debconf) et bloque l'installation.
# C'est le blocage constaté sur Zorin OS : Zorin livre sa propre copie de
# /etc/gtk-3.0/settings.ini(.real), et la variante i386 de libgtk-3-0t64
# apporte une nouvelle version d'un conffile absent du système.
#
#   --force-confdef  : appliquer l'action par défaut définie par le paquet ;
#   --force-confmiss : réinstaller un conffile MANQUANT (= répondre « Y ») ;
#   --force-confold  : conserver la version locale d'un conffile MODIFIÉ.
#
# Le choix automatique est celui d'un administrateur : un fichier supprimé
# est restauré, une personnalisation existante est conservée.
# ------------------------------------------------------------------------------

apt_run() {
    sudo env \
        DEBIAN_FRONTEND=noninteractive \
        DEBCONF_NONINTERACTIVE_SEEN=true \
        APT_LISTCHANGES_FRONTEND=none \
        NEEDRESTART_MODE=a \
        apt-get \
        -o Dpkg::Options::=--force-confdef \
        -o Dpkg::Options::=--force-confmiss \
        -o Dpkg::Options::=--force-confold \
        -o APT::Get::Assume-Yes=true \
        -o Dpkg::Use-Pty=0 \
        -o DPkg::Lock::Timeout=600 \
        "$@"
}

# apt_install [-v] PAQUETS...
#   -v : sortie affichée (installation principale de Wine, plusieurs minutes)
# En cas d'échec, un second essai ciblé est tenté sur l'erreur Zorin
# « trying to overwrite shared '/etc/gtk-3.0/settings.ini' » : les variantes
# amd64 et i386 de libgtk-3-0t64 se disputent ce fichier livré par Zorin.
apt_install() {
    local visible=0
    if [ "${1:-}" = "-v" ]; then
        visible=1
        shift
    fi

    local logfile="$TMP_DIR/apt-output.log"
    local st=0

    set +e
    if [ "$visible" = "1" ]; then
        apt_run install "$@" 2>&1 | tee "$logfile"
        st=${PIPESTATUS[0]}
    else
        apt_run install "$@" >"$logfile" 2>&1
        st=$?
    fi
    set -e

    if [ "$st" -eq 0 ]; then
        return 0
    fi

    # En mode visible, la sortie a déjà été affichée par « tee » :
    # on ne la répète pas une seconde fois.
    if [ "$visible" = "0" ] && [ -s "$logfile" ]; then
        cat "$logfile" >&2
    fi

    if grep -qi "trying to overwrite" "$logfile" 2>/dev/null; then
        log_warning "Conflit de fichiers partagés (Zorin OS) : nouvel essai en autorisant l'écrasement."
        apt_run install -o Dpkg::Options::=--force-overwrite "$@"
        return $?
    fi

    return 1
}

# Un dpkg laissé « half-configured » (interruption précédente, Live CD)
# bloquerait toute installation : on le répare avant de commencer.
debian_repair_dpkg() {
    sudo env DEBIAN_FRONTEND=noninteractive \
        dpkg --configure -a \
        --force-confdef --force-confmiss --force-confold >/dev/null 2>&1 || true
}

# Processus apt/dpkg encore actifs (retour vide s'il n'y en a pas).
debian_apt_holders() {
    ps -eo pid=,comm= 2>/dev/null \
        | awk '$2 ~ /^(apt-get|apt|dpkg|unattended-upgr|aptd|aptdcon)$/ {printf "%s (%s) ", $1, $2}'
}

# Attend (au plus SECONDES, par pas de 10 s) qu'un apt-get / dpkg déjà lancé
# libère le verrou dpkg. Cas réel (Ubuntu 26.04, session Live) : le premier
# essai interrompu laissait un apt-get actif, et l'essai suivant échouait sur
# « Could not get lock /var/lib/dpkg/lock-frontend ». On attend proprement la
# fin du processus en cours au lieu d'échouer (apt attend lui-même via
# DPkg::Lock::Timeout=600 ; ceci couvre aussi dpkg --configure -a).
debian_wait_for_dpkg_lock() {
    local max_secs="${1:-300}"
    local waited=0 holders

    holders="$(debian_apt_holders)"
    [ -n "$holders" ] || return 0

    log_warning "Un autre gestionnaire de paquets est encore en cours : $holders"
    log_info "Attente de sa fin (au plus $((max_secs / 60)) min) : ne fermez pas cette fenêtre..."

    while [ "$waited" -lt "$max_secs" ]; do
        sleep 10
        waited=$((waited + 10))
        holders="$(debian_apt_holders)"
        if [ -z "$holders" ]; then
            log_info "Le gestionnaire de paquets précédent a terminé : l'installation continue."
            return 0
        fi
    done

    log_warning "Le verrou dpkg est toujours occupé après $((max_secs / 60)) min ($holders)."
    log_warning "Si l'installation reste bloquée, redémarrez la machine puis relancez ce script."
    return 1
}

# Version candidate (apt) du paquet indiqué, vide si aucune.
apt_candidate_version() {
    apt-cache policy "$1" 2>/dev/null | sed -n 's/^ *Candidate: //p' | head -n1
}

# Numéro de version MAJEUR du paquet candidat (0 si absent).
# « 11.0.0.0~noble-1 » donne 11, « 10.0~repack-12ubuntu1 » donne 10.
apt_candidate_major() {
    local v major
    v="$(apt_candidate_version "$1")"

    case "$v" in
        ""|"(none)")
            echo "0"
            return 0
            ;;
    esac

    v="${v#*:}"            # époque de version éventuelle (« 2:1.0 » devient « 1.0 »)
    major="${v%%[.~+-]*}"

    case "$major" in
        ''|*[!0-9]*) echo "0" ;;
        *)           echo "$major" ;;
    esac
}

# ==============================================================================
# Installation de Wine : famille Debian / Ubuntu / Mint  (WineHQ)
# ==============================================================================
# Comportement v4.10 :
#   1. dépôt WineHQ configuré AVANT winetricks (clé .asc + .key, conservé) ;
#   2. choix AUTOMATIQUE de la branche WineHQ proposant Wine 10/11 :
#      winehq-stable puis winehq-devel puis winehq-staging (voir plus bas) ;
#   3. repli sur le Wine des dépôts de la distribution ;
#   4. vérification finale de la version (Pronote refuse Wine 9 ou moins).
# ------------------------------------------------------------------------------

# Choisit et installe automatiquement un paquet WineHQ dont la version est
# 10 ou 11, quelle que soit la branche qui le propose : « winehq-stable »
# d'abord, puis « winehq-devel », puis « winehq-staging ». Résout le cas
# Ubuntu 26.04 (« resolute ») où WineHQ n'offre PAS winehq-stable mais
# propose winehq-devel en version 11 : auparavant le script retombait alors
# sur le Wine de la distribution. Une branche ne proposant que du Wine 9 ou
# moins est ignorée (Pronote refuse de démarrer avec Wine 9 ou inférieur).
install_wine_from_winehq() {
    local pkg v

    for pkg in winehq-stable winehq-devel winehq-staging; do
        v="$(apt_candidate_major "$pkg")"

        if [ "$v" -lt 10 ]; then
            if [ "$v" -gt 0 ]; then
                log_info "$pkg : version proposée $v (< 10) : branche ignorée."
            fi
            continue
        fi

        log_info "Installation de $pkg (Wine $v) depuis WineHQ..."
        log_info "Téléchargement important (plusieurs centaines de Mo), puis décompression"
        log_info "pouvant rester silencieuse plusieurs minutes (surtout en Live) : patientez."

        if apt_install -v --install-recommends "$pkg"; then
            log_success "$pkg installé (Wine $v)."
            return 0
        fi

        log_warning "Échec de l'installation de $pkg : essai de la branche WineHQ suivante..."
    done

    log_warning "Aucun paquet WineHQ 10/11 n'a pu être installé."
    return 1
}

# Filet de sécurité : Pronote exige Wine 10 minimum. Si le Wine installé est
# trop ancien (ex. Wine 9.0 des dépôts Mint 22.3 alors que le dépôt WineHQ
# n'a pas abouti plus haut), on tente une dernière fois le dépôt WineHQ
# (stable puis devel puis staging). Sinon, on arrête avec une explication
# claire : installer Pronote avec Wine 9 donnerait un logiciel incapable de
# démarrer. Contournement expert : PRONOTE_IGNORE_WINE_VERSION=1.
ensure_wine_minimum_version_debian() {
    discover_wine_paths

    local v
    v="$(get_wine_version)"

    if [ "$v" = "0" ]; then
        log_error "Wine est introuvable après l'installation des paquets."
        print_wine_diagnostics
        exit 1
    fi

    if ! version_lt "$v" "$MIN_WINE_VERSION"; then
        log_success "Wine installé : version $v (>= $MIN_WINE_VERSION, compatible Pronote)."
        return 0
    fi

    log_warning "Wine $v est installé, mais Pronote ${PRONOTE_YEAR} exige Wine $MIN_WINE_VERSION au minimum."
    log_info "Nouvelle tentative d'obtention d'un Wine récent via le dépôt WineHQ..."

    debian_wait_for_dpkg_lock 300 || true
    if setup_winehq_repo && install_wine_from_winehq; then
        discover_wine_paths
        v="$(get_wine_version)"
        if ! version_lt "$v" "$MIN_WINE_VERSION"; then
            log_success "Wine $v installé via WineHQ : version compatible Pronote."
            return 0
        fi
    fi

    if [ "${PRONOTE_IGNORE_WINE_VERSION:-0}" = "1" ]; then
        log_warning "Wine $v conservé malgré tout (PRONOTE_IGNORE_WINE_VERSION=1)."
        return 0
    fi

    log_error "Impossible d'obtenir Wine $MIN_WINE_VERSION ou plus récent sur ce système."
    echo ""
    echo "Pronote ${PRONOTE_YEAR} refuse de démarrer avec Wine 9 ou une version inférieure :"
    echo "l'installation est arrêtée ici pour éviter d'installer un logiciel inutilisable."
    echo ""
    echo "Suggestions :"
    echo "  • Vérifiez la connexion Internet, puis relancez ce script."
    echo "  • Si un gestionnaire de paquets semble resté bloqué, redémarrez la machine"
    echo "    puis relancez ce script."
    echo "  • Pour installer quand même, en connaissance de cause (déconseillé) :"
    echo "    PRONOTE_IGNORE_WINE_VERSION=1 bash $0"
    print_disk_hint_if_low
    exit 1
}

# Affiche « base codename » (ex. « ubuntu noble », « debian trixie ») ou rien.
debian_winehq_target() {
    local base="" codename=""
    local os_id="" os_id_like="" os_ubuntu_codename="" os_version_codename=""

    if [ -f /etc/os-release ]; then
        os_id="$(. /etc/os-release; echo "${ID:-}")"
        os_id_like="$(. /etc/os-release; echo "${ID_LIKE:-}")"
        os_ubuntu_codename="$(. /etc/os-release; echo "${UBUNTU_CODENAME:-}")"
        os_version_codename="$(. /etc/os-release; echo "${VERSION_CODENAME:-}")"
    fi

    case "$os_id" in
        ubuntu)
            base="ubuntu"
            codename="${os_version_codename:-$os_ubuntu_codename}"
            ;;
        debian)
            base="debian"
            codename="$os_version_codename"
            ;;
        *)
            if [ -n "$os_ubuntu_codename" ]; then
                # Mint, Pop!_OS, Zorin, elementary, KDE neon...
                base="ubuntu"
                codename="$os_ubuntu_codename"
            else
                case " $os_id_like " in
                    *" ubuntu "*)
                        base="ubuntu"
                        codename="$os_version_codename"
                        ;;
                    *" debian "*)
                        base="debian"
                        codename=""
                        ;;
                esac
            fi
            ;;
    esac

    if [ "$base" = "debian" ]; then
        local known=" bullseye bookworm trixie forky sid "
        case "$known" in
            *" $codename "*) ;;
            *) codename="" ;;
        esac

        # LMDE, MX, Kali, Debian testing : déduction via /etc/debian_version
        if [ -z "$codename" ] && [ -f /etc/debian_version ]; then
            local dv=""
            dv="$(cat /etc/debian_version 2>/dev/null || true)"
            case "$dv" in
                11*|bullseye*) codename="bullseye" ;;
                12*|bookworm*) codename="bookworm" ;;
                13*|trixie*)   codename="trixie" ;;
                14*|forky*)    codename="forky" ;;
                *sid*)         codename="sid" ;;
            esac
        fi
    fi

    if [ -n "$base" ] && [ -n "$codename" ]; then
        echo "$base $codename"
    fi

    return 0
}

setup_winehq_repo() {
    log_info "Configuration du dépôt WineHQ (Wine stable récent)..."

    local target base codename
    target="$(debian_winehq_target)"

    if [ -z "$target" ]; then
        log_warning "Impossible de déterminer le nom de code de la distribution."
        log_warning "Le dépôt WineHQ ne sera pas configuré ; Wine des dépôts standards sera utilisé."
        return 1
    fi

    base="${target%% *}"
    codename="${target##* }"
    log_info "Base : $base / nom de code : $codename"

    local sources_url="https://dl.winehq.org/wine-builds/${base}/dists/${codename}/winehq-${codename}.sources"

    if ! url_exists "$sources_url"; then
        log_warning "WineHQ ne propose pas (encore) de dépôt pour ${base} « ${codename} »."
        log_warning "Wine des dépôts standards sera utilisé."
        return 1
    fi

    debian_repair_dpkg
    apt_install wget gnupg ca-certificates >/dev/null 2>&1 || true
    sudo mkdir -p /etc/apt/keyrings

    # Ubuntu 26.04 / APT 3.2 : une clé ASCII doit s'appeler .asc, pas .key.
    local key_tmp="$TMP_DIR/winehq.key"
    if ! download_file "https://dl.winehq.org/wine-builds/winehq.key" "$key_tmp"; then
        log_warning "Impossible de télécharger la clé WineHQ (.asc)."
        return 1
    fi
    sudo install -m 0644 "$key_tmp" /etc/apt/keyrings/winehq-archive.asc

    # Copie déarmorée sous le nom historique, pour les fichiers .sources WineHQ.
    if check_command gpg && gpg --dearmor --yes -o "$TMP_DIR/winehq-archive.key" "$key_tmp" >/dev/null 2>&1; then
        sudo install -m 0644 "$TMP_DIR/winehq-archive.key" /etc/apt/keyrings/winehq-archive.key
    else
        sudo install -m 0644 "$key_tmp" /etc/apt/keyrings/winehq-archive.key
    fi

    local sources_tmp="$TMP_DIR/winehq-${codename}.sources"
    if ! download_file "$sources_url" "$sources_tmp"; then
        log_warning "Impossible de télécharger le fichier de dépôt WineHQ."
        return 1
    fi

    # Le fichier WineHQ référence winehq-archive.key : on pointe vers la clé
    # ASCII .asc, acceptée par toutes les versions d'APT (2.4 → 3.2).
    sed -i 's|/etc/apt/keyrings/winehq-archive.key|/etc/apt/keyrings/winehq-archive.asc|g' "$sources_tmp" 2>/dev/null || true

    # Nettoyage d'éventuels anciens dépôts WineHQ (autre nom de code)
    sudo rm -f /etc/apt/sources.list.d/winehq-*.sources /etc/apt/sources.list.d/winehq*.list 2>/dev/null || true
    sudo install -m 0644 "$sources_tmp" "/etc/apt/sources.list.d/winehq-${codename}.sources"

    apt_run update || log_warning "apt-get update a signalé des erreurs (autres dépôts ?) : vérification de WineHQ..."

    # Le dépôt est jugé utilisable dès qu'il propose UNE branche WineHQ, même
    # si « winehq-stable » en est absent (Ubuntu 26.04 « resolute » :
    # winehq-stable absent, winehq-devel/winehq-staging présents en 11.x).
    # Le choix du paquet (version 10/11 exigée) se fait ensuite dans
    # install_wine_from_winehq.
    local pkg
    local -a branches=()
    for pkg in winehq-stable winehq-devel winehq-staging; do
        if [ "$(apt_candidate_major "$pkg")" -gt 0 ]; then
            branches+=("$pkg")
        fi
    done

    if [ "${#branches[@]}" -gt 0 ]; then
        log_success "Dépôt WineHQ configuré pour ${base} ${codename} (branches proposées : ${branches[*]})."
        return 0
    fi

    log_warning "Aucun paquet WineHQ (stable/devel/staging) n'est proposé par le dépôt : dépôt WineHQ retiré."
    sudo rm -f "/etc/apt/sources.list.d/winehq-${codename}.sources" 2>/dev/null || true
    apt_run update >/dev/null 2>&1 || true
    return 1
}

install_debian_optional_packages() {
    if apt_install "$@" >/dev/null 2>&1; then
        return 0
    fi

    # Un paquet manquant ne doit pas bloquer les autres
    local p
    for p in "$@"; do
        if dpkg -s "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! apt_install "$p" >/dev/null 2>&1; then
            log_warning "Paquet optionnel non installé : $p"
        fi
    done

    return 0
}

install_wine_debian() {
    require_privilege_helper
    log_info "Famille Debian/Ubuntu détectée : installation de Wine..."

    if ! dpkg --print-foreign-architectures 2>/dev/null | grep -qx i386; then
        log_info "Activation de l'architecture i386 (nécessaire aux applications 32 bits)..."
        sudo dpkg --add-architecture i386 || log_warning "Impossible d'activer l'architecture i386."
    fi

    # Un apt-get précédent peut encore tourner (essai interrompu, session
    # Live Ubuntu 26.04...) : on attend le verrou dpkg au lieu d'échouer
    # immédiatement dessus.
    debian_wait_for_dpkg_lock 300 || true

    debian_repair_dpkg

    local winehq_ok=0

    if setup_winehq_repo; then
        if install_wine_from_winehq; then
            winehq_ok=1
        else
            log_warning "WineHQ n'a pas fourni de Wine installable : repli sur Wine des dépôts standards."
        fi
    fi

    if [ "$winehq_ok" -eq 0 ]; then
        apt_run update >/dev/null 2>&1 || true
        debian_wait_for_dpkg_lock 300 || true
        log_info "Installation de Wine depuis les dépôts de la distribution..."
        log_info "(Après le téléchargement, la décompression de Wine peut rester silencieuse"
        log_info " plusieurs minutes, surtout en session Live : c'est normal, patientez.)"
        if ! apt_install -v --install-recommends wine; then
            apt_install -v wine64 || true
        fi

        # wine32 (préfixes 32 bits natifs) : facultatif
        if ! apt_install wine32:i386 >/dev/null 2>&1; then
            if ! apt_install wine32 >/dev/null 2>&1; then
                log_info "wine32 non installé (facultatif : le mode WoW64 sera utilisé pour le 32 bits si Wine le permet)."
            fi
        fi
    fi

    # Pronote exige Wine 10 minimum : vérification + dernier recours WineHQ.
    ensure_wine_minimum_version_debian

    log_info "Installation des outils complémentaires (cabextract, icoutils, ImageMagick...)..."
    install_debian_optional_packages cabextract icoutils imagemagick desktop-file-utils wget xdg-utils librsvg2-bin file

    # winetricks : version autonome (le paquet apt dépend du « wine » Ubuntu,
    # ce qui casserait WineHQ). Géré ensuite par ensure_winetricks.
    return 0
}

# ==============================================================================
# Installation de Wine : famille Fedora / RHEL  (dnf)
# ==============================================================================
# Deux temps : wine (obligatoire, sortie visible) puis outils optionnels.
# Un paquet optionnel absent n'interrompt plus le script.
# ------------------------------------------------------------------------------

fedora_pkg_manager() {
    if check_command dnf; then
        echo "dnf"
    elif check_command yum; then
        echo "yum"
    else
        echo ""
    fi
}

fedora_is_dnf5() {
    dnf --version 2>/dev/null | head -n1 | grep -qE '^dnf5|^5\.' && return 0
    dnf --version 2>/dev/null | grep -qiE 'dnf5|^5\.' && return 0
    return 1
}

install_fedora_optional_packages() {
    local pm
    pm="$(fedora_pkg_manager)"
    [ -n "$pm" ] || return 0

    if [ "$pm" = "dnf" ] && fedora_is_dnf5; then
        if sudo dnf install -y --skip-unavailable "$@"; then
            return 0
        fi
    else
        if sudo "$pm" install -y --setopt=strict=0 "$@"; then
            return 0
        fi
    fi

    local p
    for p in "$@"; do
        if rpm -q "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! sudo "$pm" install -y "$p" >/dev/null 2>&1; then
            log_warning "Paquet optionnel non installé : $p"
        fi
    done

    return 0
}

install_wine_fedora() {
    require_privilege_helper
    log_info "Famille Fedora/RHEL détectée : installation de Wine (dnf)..."

    local pm
    pm="$(fedora_pkg_manager)"
    if [ -z "$pm" ]; then
        log_error "Ni dnf ni yum n'est disponible."
        exit 1
    fi

    case "$DISTRO_ID" in
        rhel|centos|almalinux|rocky)
            log_info "Activation d'EPEL et du dépôt CRB (nécessaires à Wine sur RHEL et dérivées)..."
            sudo "$pm" install -y epel-release >/dev/null 2>&1 || true
            sudo "$pm" config-manager --set-enabled crb >/dev/null 2>&1 \
                || sudo "$pm" config-manager setopt crb.enabled=1 >/dev/null 2>&1 \
                || sudo crb enable >/dev/null 2>&1 \
                || true
            ;;
    esac

    log_info "Installation de wine (plusieurs minutes, ~1,5 Go téléchargés)..."
    if ! sudo "$pm" install -y wine; then
        log_error "Installation de wine impossible avec $pm."
        print_disk_hint_if_low
        exit 1
    fi
    log_success "Paquet wine installé."

    log_info "Installation des outils complémentaires (winetricks, cabextract, icoutils, ImageMagick...)..."
    install_fedora_optional_packages winetricks cabextract icoutils ImageMagick desktop-file-utils wget xdg-utils librsvg2-tools file

    return 0
}

# ==============================================================================
# Installation de Wine : famille Arch / CachyOS / Manjaro  (pacman)
# ==============================================================================

install_wine_arch() {
    require_privilege_helper
    log_info "Famille Arch détectée : installation de Wine (pacman)..."

    if ! grep -qE '^\[multilib\]' /etc/pacman.conf 2>/dev/null; then
        if grep -qE '^#\[multilib\]' /etc/pacman.conf 2>/dev/null; then
            log_info "Activation du dépôt multilib dans /etc/pacman.conf..."
            sudo sed -i '/^#\[multilib\]/,/^#Include/ s/^#//' /etc/pacman.conf
        else
            log_info "Ajout du dépôt multilib à /etc/pacman.conf..."
            printf '\n[multilib]\nInclude = /etc/pacman.d/mirrorlist\n' | sudo tee -a /etc/pacman.conf >/dev/null
        fi
    fi

    sudo pacman -Sy --noconfirm || log_warning "Rafraîchissement des dépôts pacman incomplet."

    log_info "Installation de wine..."
    if ! sudo pacman -S --needed --noconfirm wine; then
        log_error "Installation de wine impossible avec pacman."
        exit 1
    fi
    log_success "Paquet wine installé."

    log_info "Installation des outils complémentaires..."
    local p
    for p in winetricks wine-mono wine-gecko cabextract icoutils imagemagick desktop-file-utils wget xdg-utils librsvg file; do
        if pacman -Qi "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! sudo pacman -S --needed --noconfirm "$p" >/dev/null 2>&1; then
            log_warning "Paquet optionnel non installé : $p"
        fi
    done

    return 0
}

# ==============================================================================
# Installation de Wine : famille openSUSE  (zypper)
# ==============================================================================

install_wine_suse() {
    require_privilege_helper
    log_info "Famille openSUSE détectée : installation de Wine (zypper)..."

    sudo zypper --non-interactive --gpg-auto-import-keys refresh || log_warning "Rafraîchissement zypper incomplet."

    log_info "Installation de wine..."
    if ! sudo zypper --non-interactive install --auto-agree-with-licenses wine; then
        log_error "Installation de wine impossible avec zypper."
        exit 1
    fi
    log_success "Paquet wine installé."

    # Paquet par paquet : un paquet absent (ex. wine-32bit sur un Wine WoW64
    # intégré) n'annule pas les autres. Pas de « zypper search » préalable :
    # zypper install signale lui-même l'absence, plus vite et sans dépendre
    # de l'état du cache des dépôts.
    log_info "Installation des outils complémentaires (paquet par paquet)..."
    local p
    for p in wine-32bit winetricks cabextract icoutils ImageMagick desktop-file-utils wget xdg-utils xdg-user-dirs rsvg-convert file; do
        if rpm -q "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! sudo zypper --non-interactive install --auto-agree-with-licenses "$p" >/dev/null 2>&1; then
            log_info "Paquet non installé (absent des dépôts ou optionnel) : $p"
        fi
    done

    return 0
}

# ==============================================================================
# Installation de Wine : NixOS / GLF OS  (relance dans nix-shell)
# ==============================================================================
# Pas de fetchTarball ni de liens WineHQ : nix-shell -p sur le nixpkgs système,
# repli wineWow64Packages.stable → wineWowPackages.stable → wine64 → wine,
# puis « nix shell nixpkgs#... » (flakes) si le canal est absent.
# Les choix déjà faits sont transmis via PRONOTE_SHOW_WINE_LOGS et
# PRONOTE_FORCED_WINEPREFIX ; WINEPREFIX n'est jamais exportée ici, sinon le
# re-exec installerait le 32 bits dans le préfixe du 64 bits (bug NixOS v4.8).
# ------------------------------------------------------------------------------

install_wine_nixos() {
    log_info "NixOS / GLF OS détecté : préparation d'un environnement nix-shell avec Wine..."

    local script_path=""
    script_path="$(readlink -f "$0" 2>/dev/null || true)"

    if [ -z "$script_path" ] || [ ! -f "$script_path" ]; then
        log_error "Impossible de localiser le fichier du script (\$0 = $0)."
        echo "Enregistrez le script dans un fichier (ex. ~/pronote.sh) puis relancez : bash ~/pronote.sh"
        exit 1
    fi

    # Choix déjà faits : ils sont transmis au re-exec, puis l'environnement est
    # nettoyé pour que le second processus reparte d'une page blanche.
    export PRONOTE_SHOW_WINE_LOGS="$SHOW_WINE_LOGS"

    # Un préfixe forcé par l'utilisateur est transmis via cette variable dédiée
    # (jamais via WINEPREFIX : voir l'en-tête du script).
    if [ -n "$WINEPREFIX_USER" ]; then
        export PRONOTE_FORCED_WINEPREFIX="$WINEPREFIX_USER"
    fi
    unset WINEPREFIX

    local -a wine_candidates=(wineWow64Packages.stable wineWowPackages.stable wine64 wine)
    local -a tools=(winetricks cabextract icoutils imagemagick wget curl desktop-file-utils xdg-utils librsvg file)

    local run_cmd="" a
    printf -v run_cmd 'bash %q --in-nix-shell' "$script_path"
    for a in ${ALL_ARGS[@]+"${ALL_ARGS[@]}"}; do
        printf -v run_cmd '%s %q' "$run_cmd" "$a"
    done

    local cand

    if check_command nix-shell && nix-instantiate --eval -E 'with import <nixpkgs> {}; lib.version' >/dev/null 2>&1; then
        for cand in "${wine_candidates[@]}"; do
            if nix-instantiate --eval -E "with import <nixpkgs> {}; ($cand).name" >/dev/null 2>&1; then
                log_info "Paquet Wine retenu : $cand"
                log_info "Nix va télécharger Wine et les outils si nécessaire (plusieurs minutes possibles)..."
                exec nix-shell -p "$cand" "${tools[@]}" --run "$run_cmd"
            fi
        done
        log_warning "Aucun paquet Wine évaluable dans <nixpkgs>."
    else
        log_info "Canal <nixpkgs> indisponible : tentative via flakes (nix shell nixpkgs#...)."
    fi

    if check_command nix; then
        local -a flake_pkgs=()
        local t
        for cand in "${wine_candidates[@]}"; do
            if nix --extra-experimental-features 'nix-command flakes' eval --raw "nixpkgs#${cand}.name" >/dev/null 2>&1; then
                log_info "Paquet Wine retenu (flakes) : nixpkgs#$cand"
                flake_pkgs=("nixpkgs#$cand")
                for t in "${tools[@]}"; do
                    flake_pkgs+=("nixpkgs#$t")
                done
                exec nix --extra-experimental-features 'nix-command flakes' shell "${flake_pkgs[@]}" --command bash -c "$run_cmd"
            fi
        done
    fi

    log_error "Impossible de préparer Wine via Nix (ni nix-shell, ni flakes)."
    echo "Essai manuel possible :"
    echo "  nix-shell -p wineWow64Packages.stable winetricks cabextract icoutils imagemagick wget --run \"$run_cmd\""
    exit 1
}

# ==============================================================================
# Installation de Wine : Solus  (eopkg)
# ==============================================================================
# Solus construit désormais Wine 11 avec --enable-archs=x86_64,i386 :
# un seul paquet « wine » couvre le 32 et le 64 bits. Le paquet « wine-32bit »
# n'existe plus dans l'index (« Repo item wine-32bit not found »).
# Installation paquet par paquet : un paquet absent est simplement ignoré.
# ------------------------------------------------------------------------------

solus_pkg_available() {
    local pkg="$1"
    local out=""
    out="$(eopkg info "$pkg" 2>&1 || true)"

    if [ -z "$out" ]; then
        return 1
    fi
    if grep -qi "not found" <<< "$out"; then
        return 1
    fi
    return 0
}

install_wine_solus() {
    require_privilege_helper
    log_info "Solus détecté : installation de Wine (eopkg)..."
    log_info "Solus fournit un paquet « wine » unique en WoW64 intégré (32 + 64 bits) :"
    log_info "le paquet « wine-32bit » n'existe plus dans le dépôt et n'est plus nécessaire."

    local -a eopkg_opts=()
    if [ "$IS_LIVE_SESSION" = "1" ]; then
        # En Live, la mise à jour forcée de system.base remplirait la RAM.
        eopkg_opts+=(--ignore-safety)
        log_info "Session Live : --ignore-safety (pas de mise à jour forcée de system.base)."
    fi

    sudo eopkg update-repo >/dev/null 2>&1 || sudo eopkg ur >/dev/null 2>&1 || log_warning "Mise à jour de l'index eopkg impossible (on continue)."

    log_info "Installation de wine..."
    if ! sudo eopkg install -y ${eopkg_opts[@]+"${eopkg_opts[@]}"} wine; then
        log_error "Installation de wine impossible via eopkg."
        print_disk_hint_if_low
        exit 1
    fi
    log_success "Paquet wine installé."

    # « wine-32bit » n'est plus proposé : le paquet « wine » couvre déjà le 32 bits.
    log_info "Installation des outils complémentaires (un par un, les paquets absents sont ignorés)..."
    local p
    for p in winetricks cabextract icoutils imagemagick desktop-file-utils wget xdg-utils xdg-user-dirs librsvg file; do
        if ! solus_pkg_available "$p"; then
            log_info "Paquet absent du dépôt Solus (ignoré) : $p"
            continue
        fi
        if ! sudo eopkg install -y ${eopkg_opts[@]+"${eopkg_opts[@]}"} "$p" >/dev/null 2>&1; then
            log_warning "Paquet optionnel non installé : $p"
        fi
    done

    return 0
}

# ==============================================================================
# Installation de Wine : Alpine Linux  (apk, dépôt community)
# ==============================================================================

install_wine_alpine() {
    require_privilege_helper
    log_info "Alpine Linux détecté : installation de Wine (apk, dépôt community)..."

    if ! grep -qE '^[^#].*/community/?$' /etc/apk/repositories 2>/dev/null; then
        if grep -qE '^#.*/community/?$' /etc/apk/repositories 2>/dev/null; then
            log_info "Activation du dépôt community..."
            sudo sed -i 's|^#\(.*/community/\{0,1\}\)$|\1|' /etc/apk/repositories
        else
            local main_line="" community_line=""
            main_line="$(grep -E '^[^#].*/main/?$' /etc/apk/repositories 2>/dev/null | head -n1 || true)"
            if [ -n "$main_line" ]; then
                community_line="$(sed 's|/main/\{0,1\}$|/community|' <<< "$main_line")"
                log_info "Ajout du dépôt community : $community_line"
                echo "$community_line" | sudo tee -a /etc/apk/repositories >/dev/null
            fi
        fi
    fi

    sudo apk update || log_warning "apk update incomplet."

    log_info "Installation de wine..."
    if ! sudo apk add wine; then
        log_error "Installation de wine impossible via apk."
        exit 1
    fi
    log_success "Paquet wine installé."

    # Alpine sépare rsvg-convert de librsvg ; la police DejaVu s'appelle
    # font-dejavu (ttf-dejavu n'est qu'un ancien nom, conservé par sécurité).
    log_info "Installation des outils complémentaires..."
    local p
    for p in bash coreutils wget curl cabextract icoutils imagemagick desktop-file-utils xdg-utils xdg-user-dirs \
             librsvg rsvg-convert gtk-update-icon-cache file font-dejavu ttf-dejavu; do
        if apk info -e "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! sudo apk add "$p" >/dev/null 2>&1; then
            log_info "Paquet non installé (absent ou optionnel) : $p"
        fi
    done

    # winetricks n'est pas empaqueté : version autonome via ensure_winetricks
    return 0
}

# ==============================================================================
# Installation de Wine : Void Linux  (xbps)
# ==============================================================================

install_wine_void() {
    require_privilege_helper
    log_info "Void Linux détecté : installation de Wine (xbps)..."

    # Void refuse toute installation tant que xbps lui-même n'est pas à jour
    # (« The 'xbps' package must be updated ») : on le met à jour d'abord.
    sudo xbps-install -Syu xbps >/dev/null 2>&1 || log_info "xbps déjà à jour (ou mise à jour différée)."

    if ! xbps-query -l 2>/dev/null | grep -q ' void-repo-multilib-'; then
        log_info "Activation du dépôt multilib (void-repo-multilib)..."
        sudo xbps-install -Sy void-repo-multilib \
            || log_warning "Dépôt multilib indisponible (musl ou architecture non x86_64 ?) : Pronote 32 bits pourrait ne pas fonctionner."
    fi

    sudo xbps-install -S || log_warning "Synchronisation xbps incomplète."

    log_info "Installation de wine..."
    if ! sudo xbps-install -y wine; then
        log_error "Installation de wine impossible via xbps."
        exit 1
    fi
    log_success "Paquet wine installé."

    log_info "Installation des outils complémentaires..."
    local p
    for p in wine-32bit wine-mono wine-gecko winetricks cabextract icoutils ImageMagick desktop-file-utils wget xdg-utils librsvg-utils file; do
        if xbps-query "$p" >/dev/null 2>&1; then
            continue
        fi
        if ! sudo xbps-install -y "$p" >/dev/null 2>&1; then
            log_info "Paquet non installé (absent ou optionnel) : $p"
        fi
    done

    return 0
}

# ==============================================================================
# Installation de Wine : Slackware 15.0 / -current (et Salix)
# ==============================================================================
# Wine n'est pas dans les dépôts officiels. Méthodes, dans l'ordre :
#   1. Wine déjà présent (paquet Alien Bob, SlackBuilds.org, multilib...) ;
#   2. slapt-get (Salix) ou slackpkg+ avec le dépôt « alienbob » configuré ;
#   3. téléchargement direct des paquets d'Eric Hameleers (Alien Bob) :
#      le paquet « wine » le plus récent et ses dépendances déclarées dans le
#      fichier .dep (FAudio), installés avec upgradepkg --install-new.
# Les outils graphiques (ImageMagick, librsvg, xdg-utils, desktop-file-utils,
# file) font partie d'une installation Slackware complète. cabextract et
# icoutils (SlackBuilds.org) sont installés via sbopkg s'il est présent.
# ------------------------------------------------------------------------------

slackware_pkg_installed() {
    local name="$1"
    ls /var/lib/pkgtools/packages/"$name"-[0-9]* /var/log/packages/"$name"-[0-9]* >/dev/null 2>&1
}

# Répertoires de version à essayer chez Alien Bob : « 15.0 » ou « current »
# (Slackware -current se présente comme « Slackware 15.0+ »).
slackware_release_dirs() {
    local ver=""
    ver="$(sed -nE 's/^Slackware ([0-9]+\.[0-9]+).*$/\1/p' /etc/slackware-version 2>/dev/null | head -n1 || true)"
    [ -n "$ver" ] || ver="15.0"

    if grep -q '+' /etc/slackware-version 2>/dev/null; then
        echo "current $ver"
    else
        echo "$ver current"
    fi
}

slackware_pkg_subdir() {
    case "$(uname -m)" in
        x86_64) echo "pkg64" ;;
        *)      echo "pkg" ;;
    esac
}

# install_alien_package NOM : télécharge et installe le paquet le plus récent
# de NOM depuis les miroirs d'Alien Bob, avec ses dépendances (.dep).
install_alien_package() {
    local name="$1"
    local subdir rel mirror url listing file deps dep
    subdir="$(slackware_pkg_subdir)"

    for rel in $(slackware_release_dirs); do
        for mirror in \
            "https://slackware.nl/people/alien/slackbuilds" \
            "https://slackware.uk/people/alien/slackbuilds" \
            "http://www.slackware.com/~alien/slackbuilds"
        do
            url="${mirror}/${name}/${subdir}/${rel}/"
            listing="$(fetch_url_stdout "$url" || true)"
            [ -n "$listing" ] || continue

            file="$(grep -oE "${name}-[^\"'<> ]+\.t[xg]z" <<< "$listing" | sort -uV | tail -n1 || true)"
            [ -n "$file" ] || continue

            # Dépendances déclarées (un nom de paquet par ligne)
            deps="$(fetch_url_stdout "${url}${file%.*}.dep" || true)"
            for dep in $deps; do
                case "$dep" in ''|"$name") continue ;; esac
                if slackware_pkg_installed "$dep"; then
                    continue
                fi
                log_info "Dépendance de $name : $dep"
                install_alien_package "$dep" || log_warning "Dépendance non installée : $dep"
            done

            log_info "Téléchargement de $file (Slackware $rel)..."
            if ! download_file "${url}${file}" "$TMP_DIR/$file" 1; then
                log_warning "Téléchargement impossible depuis $mirror : essai d'un autre miroir."
                continue
            fi

            if sudo upgradepkg --install-new --reinstall "$TMP_DIR/$file"; then
                log_success "Paquet installé : $file"
                rm -f "$TMP_DIR/$file" 2>/dev/null || true
                return 0
            fi

            log_warning "upgradepkg a échoué pour $file."
        done
    done

    return 1
}

slackware_install_extra_tools() {
    local p
    local -a need=()

    if ! slackware_pkg_installed cabextract && ! check_command cabextract; then
        need+=(cabextract)
    fi
    if ! slackware_pkg_installed icoutils && ! check_command wrestool; then
        need+=(icoutils)
    fi

    [ "${#need[@]}" -gt 0 ] || return 0

    if check_command sbopkg; then
        log_info "Installation via sbopkg (SlackBuilds.org) : ${need[*]}..."
        sudo sbopkg -r >/dev/null 2>&1 || true
        for p in "${need[@]}"; do
            if ! sudo sbopkg -B -e continue -i "$p" >/dev/null 2>&1; then
                log_warning "Paquet optionnel non installé : $p"
            fi
        done
    elif check_command slpkg; then
        log_info "Installation via slpkg : ${need[*]}..."
        for p in "${need[@]}"; do
            sudo slpkg -y install "$p" >/dev/null 2>&1 || log_warning "Paquet optionnel non installé : $p"
        done
    else
        log_info "Outils facultatifs absents (${need[*]}) : disponibles sur SlackBuilds.org (sbopkg)."
        log_info "Sans eux : polices Microsoft ignorées, icône extraite via ImageMagick uniquement."
    fi

    return 0
}

install_wine_slackware() {
    log_info "Slackware détecté : Wine n'est pas dans les dépôts officiels, installation automatique..."

    if wine_is_available; then
        log_success "Wine déjà présent : $WINE_BIN"
        slackware_install_extra_tools
        return 0
    fi

    require_privilege_helper

    # 1) Salix : slapt-get
    if check_command slapt-get; then
        log_info "slapt-get détecté (Salix) : tentative d'installation de wine..."
        sudo slapt-get -u >/dev/null 2>&1 || true
        sudo slapt-get -y -i wine >/dev/null 2>&1 || true
    fi

    # 2) slackpkg+ avec le dépôt alienbob
    if ! wine_is_available && check_command slackpkg \
        && grep -qsE '^[^#]*alienbob' /etc/slackpkg/slackpkgplus.conf; then
        log_info "slackpkg+ (dépôt alienbob) : installation de FAudio et wine..."
        sudo slackpkg -batch=on -default_answer=y update >/dev/null 2>&1 || true
        sudo slackpkg -batch=on -default_answer=y install FAudio wine >/dev/null 2>&1 || true
    fi

    # 3) Paquets Alien Bob téléchargés directement
    if ! wine_is_available; then
        log_info "Téléchargement des paquets Wine d'Alien Bob (Eric Hameleers)..."
        if ! install_alien_package wine; then
            log_error "Impossible d'installer Wine automatiquement."
            echo "Installez Wine manuellement (paquets Alien Bob : FAudio + wine, ou SlackBuilds.org),"
            echo "puis relancez ce script."
            exit 1
        fi
    fi

    if ! wine_is_available; then
        log_error "Wine reste introuvable après l'installation des paquets."
        print_wine_diagnostics
        exit 1
    fi

    log_success "Wine installé : $WINE_BIN"
    slackware_install_extra_tools
    return 0
}

# ==============================================================================
# Installation de Wine : Gentoo / Funtoo  (emerge / Portage)
# ==============================================================================
# Gentoo compile les paquets depuis les sources : l'installation de Wine peut
# durer de 30 minutes à plusieurs heures selon la machine. Le paquet
# « app-emulation/wine » suit la version stable de l'arbre Portage (Wine 10/11
# selon le moment) et couvre le 32 et le 64 bits (WoW64) par défaut.
# ------------------------------------------------------------------------------

install_wine_gentoo() {
    require_privilege_helper
    log_info "Famille Gentoo détectée : installation de Wine (emerge)..."

    if ! check_command emerge; then
        if wine_is_available; then
            log_success "Wine déjà présent : $WINE_BIN — on continue."
            return 0
        fi
        log_error "emerge (Portage) est introuvable : impossible d'installer Wine automatiquement."
        exit 1
    fi

    if wine_is_available; then
        log_success "Wine déjà installé : $WINE_BIN (version $(get_wine_version))"
    else
        log_warning "Gentoo compile Wine depuis les sources : l'installation peut prendre"
        log_warning "de 30 minutes à plusieurs heures selon la machine. Laissez-la se terminer."
        log_info "Synchronisation de l'arbre Portage (peut prendre quelques minutes)..."
        sudo emerge --sync >/dev/null 2>&1 \
            || log_warning "Synchronisation impossible (on continue avec l'arbre Portage actuel)."

        log_info "Compilation et installation de app-emulation/wine..."
        if ! sudo emerge --noreplace app-emulation/wine; then
            log_error "Installation de Wine impossible avec emerge."
            echo "Vérifiez les USE flags de app-emulation/wine (X, opengl sont conseillés)"
            echo "et l'espace disque disponible, puis relancez ce script."
            print_disk_hint_if_low
            exit 1
        fi
        log_success "Paquet wine installé."
    fi

    log_info "Installation des outils complémentaires (compilation, un par un)..."
    local p
    for p in app-emulation/winetricks app-arch/cabextract media-gfx/icoutils \
             media-gfx/imagemagick x11-misc/xdg-utils dev-util/desktop-file-utils \
             gnome-base/librsvg net-misc/wget; do
        sudo emerge --noreplace --quiet "$p" >/dev/null 2>&1 \
            || log_info "Paquet optionnel non installé (absent ou masqué) : $p"
    done

    return 0
}

# ==============================================================================
# Distribution inconnue
# ==============================================================================

install_wine_unknown() {
    log_warning "Distribution non reconnue ($DISTRO_ID) : le script ne peut pas installer Wine automatiquement."

    if wine_is_available; then
        log_success "Wine déjà présent : $WINE_BIN — on continue."
        return 0
    fi

    log_error "Wine est introuvable. Installez Wine (≥ $MIN_WINE_VERSION), winetricks, cabextract, icoutils et ImageMagick, puis relancez."
    exit 1
}

# ==============================================================================
# Vérifications après installation des paquets
# ==============================================================================

verify_wine_installation() {
    if ! wine_is_available; then
        log_error "Wine est introuvable après l'installation des paquets."
        print_wine_diagnostics
        exit 1
    fi

    local v
    v="$(get_wine_version)"
    log_success "Wine détecté : $WINE_BIN (version $v)"

    if version_lt "$v" "$MIN_WINE_VERSION"; then
        log_warning "Wine $v est plus ancien que la version minimale exigée par Pronote ($MIN_WINE_VERSION)."
        log_warning "Pronote ${PRONOTE_YEAR} refuse de se lancer avec Wine 9 ou une version inférieure :"
        log_warning "le logiciel risque de ne pas démarrer. Une mise à jour de Wine est fortement"
        log_warning "conseillée (dépôt WineHQ, ou distribution mise à jour) avant d'utiliser Pronote."
    fi

    return 0
}

verify_optional_tools() {
    local -a missing=()

    if ! check_command cabextract; then
        missing+=("cabextract (polices Microsoft via winetricks)")
    fi
    if ! check_command wrestool && ! check_command icotool; then
        missing+=("icoutils (extraction de l'icône depuis Pronote)")
    fi
    if ! check_command magick && ! check_command convert && ! check_command rsvg-convert; then
        missing+=("ImageMagick ou rsvg-convert (génération des icônes PNG)")
    fi
    if ! check_command update-desktop-database; then
        missing+=("desktop-file-utils (rafraîchissement du menu)")
    fi

    if [ "${#missing[@]}" -gt 0 ]; then
        log_warning "Outils facultatifs absents :"
        local m
        for m in "${missing[@]}"; do
            echo "  • $m"
        done
        echo "L'installation continue ; seuls des détails cosmétiques peuvent être affectés."
    fi

    return 0
}

# ==============================================================================
# Dépendances : point d'entrée par famille de distribution
# ==============================================================================

ensure_dependencies() {
    echo ""
    box_title "Verification des dependances" 46
    echo ""

    detect_live_session
    check_disk_space_before_wine

    case "$DISTRO_FAMILY" in
        debian)     install_wine_debian ;;
        fedora)     install_wine_fedora ;;
        arch)       install_wine_arch ;;
        suse)       install_wine_suse ;;
        nixos)      install_wine_nixos ;;          # relance le script (exec)
        nixos-done) log_info "Environnement nix-shell actif : utilisation du Wine fourni par Nix." ;;
        solus)      install_wine_solus ;;
        alpine)     install_wine_alpine ;;
        void)       install_wine_void ;;
        slackware)  install_wine_slackware ;;
        gentoo)     install_wine_gentoo ;;
        *)          install_wine_unknown ;;
    esac

    verify_wine_installation
    ensure_winetricks
    verify_optional_tools

    return 0
}

# ==============================================================================
# Configuration de Wine : préfixe, version Windows, composants
# ==============================================================================

wine_prefix_init() {
    local st=0

    if [ -n "${WINEBOOT_BIN:-}" ] && [ -x "$WINEBOOT_BIN" ]; then
        run_with_timeout 1500 "$WINEBOOT_BIN" --init || st=$?
    else
        run_with_timeout 1500 "$WINE_BIN" wineboot --init || st=$?
    fi

    return "$st"
}

winecfg_set_version() {
    local st=0

    if [ -n "${WINECFG_BIN:-}" ] && [ -x "$WINECFG_BIN" ]; then
        run_with_timeout 300 "$WINECFG_BIN" /v "$WIN_VERSION" || st=$?
    else
        run_with_timeout 300 "$WINE_BIN" winecfg /v "$WIN_VERSION" || st=$?
    fi

    if status_is_timeout "$st"; then
        run_wineserver_tool -k >/dev/null 2>&1 || true
    fi

    return "$st"
}

# Version Windows écrite dans les deux vues du registre (32 et 64 bits) via
# un unique import regedit borné dans le temps (fin du blocage NixOS 32 bits).
force_windows_version_registry() {
    local product build
    product="$(win_version_product_name)"
    build="$(win_version_build)"

    log_info "Forçage de la version Windows ($product) dans les deux vues de registre (32/64 bits)..."

    local reg_dir="$WINEPREFIX/drive_c/windows/temp"
    mkdir -p "$reg_dir" 2>/dev/null || true
    local reg_file="$reg_dir/pronote-winver.reg"
    local reg_win_path='C:\windows\temp\pronote-winver.reg'

    {
        printf 'Windows Registry Editor Version 5.00\r\n\r\n'
        local key
        for key in \
            'HKEY_LOCAL_MACHINE\Software\Microsoft\Windows NT\CurrentVersion' \
            'HKEY_LOCAL_MACHINE\Software\Wow6432Node\Microsoft\Windows NT\CurrentVersion'
        do
            printf '[%s]\r\n' "$key"
            printf '"CurrentVersion"="6.3"\r\n'
            printf '"CurrentMajorVersionNumber"=dword:0000000a\r\n'
            printf '"CurrentMinorVersionNumber"=dword:00000000\r\n'
            printf '"CurrentBuild"="%s"\r\n' "$build"
            printf '"CurrentBuildNumber"="%s"\r\n' "$build"
            printf '"ProductName"="%s"\r\n' "$product"
            printf '"CSDVersion"=""\r\n'
            printf '\r\n'
        done
        printf '[HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Control\\ProductOptions]\r\n'
        printf '"ProductType"="WinNT"\r\n\r\n'
        printf '[HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Control\\Windows]\r\n'
        printf '"CSDVersion"=dword:00000000\r\n\r\n'
    } > "$reg_file" 2>/dev/null || {
        log_warning "Impossible d'écrire le fichier de registre (disque plein ?) : forçage ignoré."
        print_disk_hint_if_low
        return 0
    }

    local st=0
    set +e
    if [ "$SHOW_WINE_LOGS" = "1" ]; then
        run_with_timeout 180 "$WINE_BIN" regedit /S "$reg_win_path"
        st=$?
    else
        (
            export WINEDEBUG="-all"
            run_with_timeout 180 "$WINE_BIN" regedit /S "$reg_win_path"
        ) >>"$LOG_FILE" 2>&1
        st=$?
    fi
    set -e

    if status_is_timeout "$st"; then
        log_warning "regedit n'a pas répondu dans le délai imparti : arrêt du wineserver et poursuite."
        log_warning "(la version Windows a déjà été définie par winecfg ; ce forçage est une sécurité)"
        run_wineserver_tool -k >/dev/null 2>&1 || true
        sleep 1
    elif [ "$st" -ne 0 ]; then
        log_warning "Forçage du registre terminé avec le code $st (non bloquant)."
    fi

    rm -f "$reg_file" 2>/dev/null || true

    # Laisse le wineserver écrire le registre (quelques secondes), sans jamais
    # bloquer : si des processus restent, l'étape suivante les réutilisera.
    settle_wine_session 15
    return 0
}

# Importe un fichier .reg dans le registre du préfixe courant (regedit
# silencieux et borné dans le temps). Renvoie 0 si l'import a réussi.
# Même mécanisme que force_windows_version_registry, réutilisé par les
# associations de pièces jointes et la mise à l'échelle des écrans QHD/4K.
import_reg_file() {
    local reg_file="$1"
    local reg_win_path="$2"
    local st=0

    set +e
    if [ "$SHOW_WINE_LOGS" = "1" ]; then
        run_with_timeout 180 "$WINE_BIN" regedit /S "$reg_win_path"
        st=$?
    else
        (
            export WINEDEBUG="-all"
            run_with_timeout 180 "$WINE_BIN" regedit /S "$reg_win_path"
        ) >>"$LOG_FILE" 2>&1
        st=$?
    fi
    set -e

    if status_is_timeout "$st"; then
        run_wineserver_tool -k >/dev/null 2>&1 || true
    fi

    [ "$st" -eq 0 ]
}

winetricks_verb() {
    local st=0
    run_with_timeout 1200 winetricks -q "$@" || st=$?

    if status_is_timeout "$st"; then
        run_wineserver_tool -k >/dev/null 2>&1 || true
    fi

    return "$st"
}

install_windows_components() {
    log_info "Installation des composants Windows utiles à Pronote."

    if ! check_command winetricks; then
        log_warning "winetricks indisponible : composants optionnels (polices, windowscodecs) ignorés."
        return 0
    fi

    export WINETRICKS_LATEST_VERSION_CHECK=disabled

    RUN_QUIET_ACTIVITY_DIR="$WINEPREFIX/drive_c"
    RUN_QUIET_FINALIZE_MSG="Composants : finalisation en cours, veuillez patienter quelques secondes..."

    if ! run_quiet "Installation des polices Microsoft" winetricks_verb corefonts; then
        log_warning "Installation partielle ou impossible de corefonts. Ce n'est pas forcément bloquant."
    fi

    if ! run_quiet "Installation de windowscodecs" winetricks_verb windowscodecs; then
        log_warning "windowscodecs non installé ou partiellement installé. Ce n'est pas forcément bloquant."
    fi

    RUN_QUIET_ACTIVITY_DIR=""
    RUN_QUIET_FINALIZE_MSG=""

    # winetricks laisse parfois des processus : fermeture bornée (30 s max).
    finish_wine_session 30 "Fin des composants Windows (ne fermez pas cette fenêtre)"
    return 0
}

prefix_is_complete() {
    [ -f "$WINEPREFIX/system.reg" ] && [ -d "$WINEPREFIX/drive_c/windows/system32" ]
}

# Attend (au plus SECONDES) que system.reg et system32 existent : le registre
# peut être écrit quelques secondes après la fin de wineboot.
wait_prefix_complete() {
    local secs="${1:-30}"
    local i

    for ((i = 0; i < secs; i++)); do
        if prefix_is_complete; then
            return 0
        fi
        sleep 1
    done

    prefix_is_complete
}

check_prefix_sanity() {
    if ! prefix_is_complete; then
        log_info "Le registre du préfixe n'est pas encore écrit : patience (30 s max)..."
        wait_prefix_complete 30 || true
    fi

    if ! prefix_is_complete; then
        log_error "Le préfixe Wine est incomplet : $WINEPREFIX"
        echo "system.reg ou drive_c/windows/system32 manquant : l'initialisation a échoué."
        print_disk_hint_if_low
        print_wine_diagnostics
        exit 1
    fi

    local mb
    mb="$(free_mb_at "$WINEPREFIX")"
    if [ -n "$mb" ] && [ "$mb" -lt 300 ] && [ "${PRONOTE_IGNORE_DISK:-0}" != "1" ]; then
        log_error "Espace disque presque épuisé après la création du préfixe ($(fmt_mb "$mb") libres)."
        print_disk_hint_if_low
        exit 1
    fi

    return 0
}

configure_wine() {
    echo ""
    box_title "Configuration de Wine" 46
    echo ""

    discover_wine_paths
    if [ -z "$WINE_BIN" ]; then
        log_error "Wine introuvable au moment de la configuration."
        print_wine_diagnostics
        exit 1
    fi

    log_info "Wine utilisé : $WINE_BIN (version $(get_wine_version))"

    # Réglage v4.6 rétabli : pas de fenêtre « Wine Mono / Gecko » pendant la
    # préparation du préfixe (Pronote n'en a pas besoin : il utilise CEF).
    # Debian/Ubuntu ne fournissent pas ces paquets : sans ce réglage, une
    # boîte de dialogue bloquante apparaissait au premier wineboot.
    export WINEDLLOVERRIDES="${WINEDLLOVERRIDES:-mscoree,mshtml=}"

    # --- Préfixe : aucune question n'est posée ------------------------------
    # Un préfixe existant de même architecture est réutilisé ; s'il contient
    # l'autre architecture, un autre chemin est choisi automatiquement.
    # Aucun préfixe n'est déplacé ni supprimé.
    resolve_prefix_conflict

    # --- Mode du préfixe selon l'architecture demandée -----------------------
    PREFIX_MODE="win64"
    if [ "$PRONOTE_ARCH" = "32" ]; then
        if [ -f "$WINEPREFIX/system.reg" ] && [ "$(detect_prefix_arch)" = "32" ]; then
            # Préfixe 32 bits natif déjà créé : pas besoin de le re-sonder.
            PREFIX_MODE="win32"
        elif probe_win32_prefix_support; then
            PREFIX_MODE="win32"
        else
            PREFIX_MODE="wow64"
        fi
    fi

    case "$PREFIX_MODE" in
        win32) export WINEARCH="win32" ;;
        *)     export WINEARCH="win64" ;;
    esac

    # --- Création si nécessaire ----------------------------------------------
    if [ ! -f "$WINEPREFIX/system.reg" ]; then
        log_info "Création d'un nouveau préfixe Wine : $WINEPREFIX"
        case "$PREFIX_MODE" in
            win32) log_info "Architecture du nouveau préfixe : 32 bits (natif, WINEARCH=win32)" ;;
            wow64) log_info "Architecture du nouveau préfixe : 64 bits WoW64 (Pronote 32 bits y sera installé)" ;;
            *)     log_info "Architecture du nouveau préfixe : 64 bits" ;;
        esac
        mkdir -p "$WINEPREFIX"
        check_disk_space_before_pronote
    fi

    RUN_QUIET_ACTIVITY_DIR="$WINEPREFIX"
    RUN_QUIET_FINALIZE_MSG="Initialisation : finalisation en cours, veuillez patienter quelques secondes..."

    if ! run_quiet "Initialisation du préfixe Wine" wine_prefix_init; then
        log_error "Échec de l'initialisation du préfixe Wine."
        print_disk_hint_if_low
        print_wine_diagnostics
        exit 1
    fi

    RUN_QUIET_ACTIVITY_DIR=""
    RUN_QUIET_FINALIZE_MSG=""

    # Le registre (system.reg) n'est écrit qu'à l'arrêt du wineserver :
    # on attend cet arrêt (quelques secondes en général, 90 s au maximum).
    finish_wine_session 90 "Écriture du registre Wine (ne fermez pas cette fenêtre)"

    # Sécurité : si le registre manque encore, un second essai d'initialisation
    # (repli hérité de la v4.6) avant tout abandon.
    if ! wait_prefix_complete 20; then
        log_warning "Préfixe encore incomplet : nouvel essai d'initialisation..."
        discover_wine_paths
        run_quiet "Nouvel essai d'initialisation du préfixe Wine" wine_prefix_init || true
        finish_wine_session 90 "Écriture du registre Wine (ne fermez pas cette fenêtre)"
    fi

    check_prefix_sanity

    # --- Version Windows ------------------------------------------------------
    log_info "Configuration de la version Windows vue par Wine : $(win_version_label)"
    if ! run_quiet "Configuration Windows" winecfg_set_version; then
        log_warning "winecfg /v $WIN_VERSION a échoué (non bloquant, le registre sera forcé)."
    fi

    force_windows_version_registry

    # --- Composants Windows ---------------------------------------------------
    install_windows_components

    # winetricks peut modifier la version Windows : on la force à nouveau.
    force_windows_version_registry

    # --- Pièces jointes : ouverture via les applications Linux ---------------
    install_attachment_opener
    register_attachment_associations

    # --- Écran haute résolution (QHD/4K) : taille des caractères -------------
    configure_hidpi_scaling

    log_success "Préfixe Wine prêt : $WINEPREFIX"
    return 0
}

# ==============================================================================
# Ouverture des pièces jointes avec les applications Linux
# ==============================================================================
# Pronote télécharge une pièce jointe du cahier de textes (PDF, Word, Excel,
# images, vidéos...) puis demande à Windows de l'ouvrir. Sans configuration,
# Wine ne sait pas quoi en faire : le fichier restait dans un dossier
# temporaire (vu sous CachyOS : téléchargé sous /run mais jamais ouvert).
#
# Principe (indépendant de l'environnement de bureau) :
#   1. un petit script « pronote-ouvrir-piece-jointe.sh » est déposé À LA
#      RACINE du préfixe Wine (il y trouve les lecteurs dosdevices) ;
#   2. le registre du préfixe (HKCR) associe chaque extension utile à ce
#      script : HKCR\.extension -> ProgID -> shell\open\command ;
#   3. appelé par Wine (comme un programme Windows via CreateProcess, le
#      noyau Linux exécutant son shebang), le script convertit le chemin
#      Windows en chemin Linux puis :
#        - images, PDF, vidéos...   -> navigateur par défaut de la
#          distribution (xdg-settings, repli sur les navigateurs connus) ;
#        - documents bureautiques   -> suite bureautique par défaut
#          (Word, Excel, PowerPoint, OpenDocument, RTF, texte...) ;
#          si aucune suite n'est installée, rien n'est ouvert ;
#        - autres formats           -> xdg-open.
# Les associations vivent dans le registre du préfixe : elles disparaissent
# avec lui et ne touchent pas au reste du système. Un journal des ouvertures
# est conservé dans ~/.local/share/pronote-pieces-jointes/ouverture.log.
# ==============================================================================

# Écrit le script d'ouverture dans le préfixe courant et garantit la présence
# du lecteur Z: (racine Linux), utilisé par le registre pour désigner le
# script (Z:\home\...\pronote-ouvrir-piece-jointe.sh).
install_attachment_opener() {
    local opener="$WINEPREFIX/pronote-ouvrir-piece-jointe.sh"

    cat > "$opener" << 'OUVRIR_EOF'
#!/usr/bin/env bash
# =============================================================================
# Ouvreur de pièces jointes Pronote - généré automatiquement par
# l'installateur Pronote Linux (ne pas modifier à la main).
#
# Pronote (sous Wine) télécharge une pièce jointe puis demande au système
# Windows de l'ouvrir. Le registre Wine (HKCR) redirige cette demande vers CE
# script, qui ouvre le fichier avec les applications Linux :
#   - images, PDF, vidéos...          -> navigateur par défaut de la
#                                        distribution ;
#   - documents bureautiques          -> suite bureautique par défaut
#     (Word, Excel, PowerPoint,          (LibreOffice, OnlyOffice...) ;
#      OpenDocument, RTF, texte...)      si aucune suite n'est installée,
#                                        rien n'est ouvert ;
#   - autres formats                  -> xdg-open (application par défaut).
#
# Le script fonctionne quel que soit l'environnement de bureau (KDE, GNOME,
# XFCE, Cinnamon, MATE, LXQt, COSMIC, Hyprland...) grâce aux outils XDG.
# Il est placé À LA RACINE du préfixe Wine : son emplacement indique où se
# trouvent les lecteurs (dosdevices) pour convertir les chemins Windows.
# =============================================================================

# Wine transmet un PATH « à la Windows » (C:\windows\...) : on rétablit un
# PATH Linux standard pour retrouver firefox, xdg-settings, libreoffice...
PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/snap/bin:$HOME/.local/bin:$HOME/.local/sbin:$HOME/.cargo/bin:$PATH"
export PATH

PREFIXE_WINE="$(cd "$(dirname "$0")" 2>/dev/null && pwd)"
JOURNAL_DIR="$HOME/.local/share/pronote-pieces-jointes"
JOURNAL="$JOURNAL_DIR/ouverture.log"

journal() {
    mkdir -p "$JOURNAL_DIR" 2>/dev/null || return 0
    # Journal borné : au-delà de 64 Ko, l'ancien est conservé en .1
    if [ -f "$JOURNAL" ] && [ "$(wc -c < "$JOURNAL" 2>/dev/null || echo 0)" -gt 65536 ]; then
        mv -f "$JOURNAL" "$JOURNAL.1" 2>/dev/null || true
    fi
    echo "$(date '+%Y-%m-%d %H:%M:%S') $*" >> "$JOURNAL" 2>/dev/null || true
}

# Lancement détaché : l'application ne doit pas rester rattachée à Wine,
# sinon Pronote pourrait attendre sa fermeture avant de rendre la main.
lancer_detache() {
    if command -v setsid >/dev/null 2>&1; then
        ( setsid "$@" >/dev/null 2>&1 </dev/null & ) >/dev/null 2>&1
    else
        ( nohup "$@" >/dev/null 2>&1 </dev/null & ) >/dev/null 2>&1
    fi
    return 0
}

# Conversion d'un chemin Windows (C:\users\...\fichier.pdf) en chemin Linux,
# sans relancer Wine : les lecteurs du préfixe servent de correspondance
# (dosdevices/c: -> drive_c, z: -> racine Linux, autres lecteurs idem).
chemin_windows_vers_linux() {
    local wpath="$1" lecteur reste periph

    wpath="${wpath#\"}"          # guillemets éventuels retirés
    wpath="${wpath%\"}"

    case "$wpath" in
        [A-Za-z]:[/\\]*)
            lecteur="${wpath:0:1}"
            lecteur="${lecteur,,}"                 # C -> c
            reste="${wpath:2}"
            reste="${reste//\\//}"                 # antislashs -> slashs
            reste="${reste#/}"
            periph="$PREFIXE_WINE/dosdevices/$lecteur:"
            if [ -e "$periph/$reste" ]; then
                readlink -f "$periph/$reste" 2>/dev/null || echo "$periph/$reste"
                return 0
            fi
            if [ "$lecteur" = "z" ]; then
                echo "/$reste"
                return 0
            fi
            echo "$periph/$reste"                  # chemin introuvable : tel quel
            return 0
            ;;
        *)
            echo "$wpath"                          # déjà un chemin Linux ?
            ;;
    esac
}

# Retrouve la commande Exec= d'un fichier .desktop à partir de son identifiant
# (ex. « firefox.desktop », « org.mozilla.firefox.desktop », « libreoffice-writer.desktop »).
commande_desktop() {
    local id="$1" f="" d ligne
    [ -n "$id" ] || return 1

    for d in \
        "${XDG_DATA_HOME:-$HOME/.local/share}/applications" \
        /usr/local/share/applications \
        /usr/share/applications \
        /var/lib/flatpak/exports/share/applications \
        "${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/exports/share/applications"; do
        if [ -f "$d/$id" ]; then
            f="$d/$id"
            break
        fi
    done

    [ -n "$f" ] || return 1
    ligne="$(grep -m1 '^Exec=' "$f" 2>/dev/null | cut -d= -f2-)"
    [ -n "$ligne" ] || return 1

    # %f %F %u %U %i %c %k retirés, espaces multiples compactés
    ligne="$(printf '%s' "$ligne" | sed -e 's/%[fFuUick]//g' -e 's/  */ /g' -e 's/^ //' -e 's/ $//')"
    [ -n "$ligne" ] || return 1
    echo "$ligne"
}

# Navigateur par défaut : celui du bureau (xdg-settings), sinon une commande
# connue présente sur la machine.
navigateur_par_defaut() {
    local id ligne b

    if command -v xdg-settings >/dev/null 2>&1; then
        id="$(xdg-settings get default-web-browser 2>/dev/null || true)"
        ligne="$(commande_desktop "$id")"
        if [ -n "$ligne" ]; then
            echo "$ligne"
            return 0
        fi
    fi

    for b in firefox chromium chromium-browser google-chrome google-chrome-stable \
             brave-browser microsoft-edge-stable microsoft-edge vivaldi-stable \
             vivaldi opera epiphany falkon konqueror qutebrowser waterfox palemoon; do
        command -v "$b" >/dev/null 2>&1 && { echo "$b"; return 0; }
    done
    return 1
}

# Suite bureautique par défaut : celle associée aux documents OpenDocument
# (xdg-mime), sinon une suite installée (par ordre de préférence).
suite_bureautique_par_defaut() {
    local id ligne s

    if command -v xdg-mime >/dev/null 2>&1; then
        id="$(xdg-mime query default application/vnd.oasis.opendocument.text 2>/dev/null || true)"
        ligne="$(commande_desktop "$id")"
        if [ -n "$ligne" ]; then
            echo "$ligne"
            return 0
        fi
    fi

    for s in soffice libreoffice onlyoffice-desktopeditors onlyoffice \
             wpsoffice wps calligrawords; do
        command -v "$s" >/dev/null 2>&1 && { echo "$s"; return 0; }
    done
    return 1
}

ouvrir_dans_le_navigateur() {
    local fichier="$1" nav

    if nav="$(navigateur_par_defaut)"; then
        # La commande peut contenir des arguments (« flatpak run org.x.y »)
        # shellcheck disable=SC2086
        lancer_detache $nav "$fichier"
        journal "Ouvert dans le navigateur ($nav) : $fichier"
    elif command -v xdg-open >/dev/null 2>&1; then
        lancer_detache xdg-open "$fichier"
        journal "Navigateur introuvable, ouvert avec xdg-open : $fichier"
    else
        journal "Aucun navigateur disponible pour : $fichier"
    fi
}

ouvrir_dans_la_suite_bureautique() {
    local fichier="$1" suite

    if suite="$(suite_bureautique_par_defaut)"; then
        # shellcheck disable=SC2086
        lancer_detache $suite "$fichier"
        journal "Ouvert dans la suite bureautique ($suite) : $fichier"
    else
        # Aucune suite bureautique installée : on n'ouvre rien (choix voulu)
        journal "Aucune suite bureautique installée : rien n'est ouvert : $fichier"
    fi
}

# ------------------------------- programme -----------------------------------

if [ "$#" -lt 1 ]; then
    journal "Appelé sans fichier à ouvrir."
    exit 0
fi

FICHIER="$(chemin_windows_vers_linux "$1")"

if [ ! -f "$FICHIER" ]; then
    journal "Fichier introuvable (reçu : [$1] -> [$FICHIER])."
    exit 0
fi

EXTENSION="${FICHIER##*.}"
EXTENSION="${EXTENSION,,}"

case "$EXTENSION" in
    png|jpg|jpeg|jfif|gif|webp|bmp|svg|pdf|mp4|m4v|webm|avi|mpg|mpeg|mov|mkv)
        ouvrir_dans_le_navigateur "$FICHIER"
        ;;
    doc|docx|docm|dot|dotx|xls|xlsx|xlsm|xltx|csv|ppt|pptx|pps|ppsx|rtf|txt|odt|ods|odp|odg|odb|odf|odc|odi|odm)
        ouvrir_dans_la_suite_bureautique "$FICHIER"
        ;;
    *)
        # Format non répertorié : l'application par défaut du système décide
        if command -v xdg-open >/dev/null 2>&1; then
            lancer_detache xdg-open "$FICHIER"
            journal "Ouvert avec xdg-open (format non répertorié .${EXTENSION}) : $FICHIER"
        else
            journal "Format non répertorié (.${EXTENSION}) et xdg-open absent : $FICHIER"
        fi
        ;;
esac

exit 0
OUVRIR_EOF

    if [ ! -s "$opener" ] || ! head -n1 "$opener" | grep -qE '^#!'; then
        log_warning "Impossible d'écrire l'ouvreur de pièces jointes : ouverture non configurée."
        return 0
    fi
    chmod 755 "$opener"

    # Répertoire du journal partagé (les deux architectures y écrivent)
    mkdir -p "$HOME/.local/share/pronote-pieces-jointes" 2>/dev/null || true

    # Lecteur Z: = racine Linux : sans lui, Windows ne verrait pas le script
    if [ -d "$WINEPREFIX/dosdevices" ] && [ ! -e "$WINEPREFIX/dosdevices/z:" ]; then
        ln -s / "$WINEPREFIX/dosdevices/z:" 2>/dev/null || true
    fi

    log_success "Ouvreur de pièces jointes installé : $opener"
    return 0
}

# Chemin Windows (lecteur Z:) de l'ouvreur, échappé pour un fichier .reg.
attachment_opener_reg_path() {
    local unix_path="$WINEPREFIX/pronote-ouvrir-piece-jointe.sh"
    local win_path="Z:/${unix_path#/}"    # /home/u/... devient Z:/home/u/...
    win_path="${win_path//\//\\}"          # slashs -> antislashs
    printf '%s' "${win_path//\\/\\\\}"  # échappement .reg : un \ devient \\
}

# Extensions ouvertes dans le navigateur par défaut (images, PDF, vidéos...)
ATTACHMENT_BROWSER_EXTS="png jpg jpeg jfif gif webp bmp svg pdf mp4 m4v webm avi mpg mpeg mov mkv"

# Extensions ouvertes dans la suite bureautique par défaut (si installée)
ATTACHMENT_OFFICE_EXTS="doc docx docm dot dotx xls xlsx xlsm xltx csv ppt pptx pps ppsx rtf txt odt ods odp odg odb odf odc odi odm"

# Enregistre dans le registre du préfixe (HKCR) l'association de toutes les
# extensions utiles vers l'ouvreur. Pronote utilise ShellExecute pour ouvrir
# une pièce jointe : Wine suit alors ces clés comme le ferait Windows.
register_attachment_associations() {
    local opener_reg
    opener_reg="$(attachment_opener_reg_path)"

    local reg_dir="$WINEPREFIX/drive_c/windows/temp"
    mkdir -p "$reg_dir" 2>/dev/null || true
    local reg_file="$reg_dir/pronote-associations.reg"
    local reg_win_path='C:\windows\temp\pronote-associations.reg'

    local ext
    if ! {
        printf 'Windows Registry Editor Version 5.00\r\n\r\n' > "$reg_file"
        printf '[HKEY_CLASSES_ROOT\\Pronote.PieceJointe]\r\n' >> "$reg_file"
        printf '@="Piece jointe Pronote"\r\n\r\n' >> "$reg_file"
        printf '[HKEY_CLASSES_ROOT\\Pronote.PieceJointe\\shell\\open\\command]\r\n' >> "$reg_file"
        printf '@="\\"%s\\" \\"%%1\\""\r\n\r\n' "$opener_reg" >> "$reg_file"
        for ext in $ATTACHMENT_BROWSER_EXTS $ATTACHMENT_OFFICE_EXTS; do
            printf '[HKEY_CLASSES_ROOT\\.%s]\r\n' "$ext" >> "$reg_file"
            printf '@="Pronote.PieceJointe"\r\n\r\n' >> "$reg_file"
        done
    } >>"$LOG_FILE" 2>&1; then
        log_warning "Impossible d'écrire le fichier d'associations : ouverture des pièces jointes non configurée."
        return 0
    fi

    if import_reg_file "$reg_file" "$reg_win_path"; then
        log_success "Ouverture des pièces jointes configurée :"
        log_success "  - images, PDF, vidéos... : navigateur par défaut de la distribution ;"
        log_success "  - documents bureautiques (Word, Excel, PowerPoint, OpenDocument,"
        log_success "    RTF, texte...) : suite bureautique par défaut (si aucune suite"
        log_success "    n'est installée, rien n'est ouvert)."
    else
        log_warning "L'enregistrement des associations de pièces jointes a échoué (non bloquant)."
    fi

    rm -f "$reg_file" 2>/dev/null || true

    # Laisse le wineserver écrire le registre (quelques secondes, borné)
    settle_wine_session 10
    return 0
}

# ==============================================================================
# Écrans haute résolution (QHD / 4K) : taille des caractères de Pronote
# ==============================================================================
# Wine n'applique aucune mise à l'échelle par défaut : sur un écran QHD
# (2560x1440) ou 4K (3840x2160), les caractères de Pronote (Arial 11 par
# défaut sous Windows) deviennent minuscules. L'équivalent Wine du réglage
# Windows est la valeur « LogPixels » du registre (96 = 100 %, 120 = 125 %,
# 144 = 150 %, 168 = 175 %, 192 = 200 %) : c'est ce que règle le curseur
# « Résolution de l'écran » de winecfg.
# La question n'est posée QUE si un écran haute résolution est détecté ;
# sur un écran 1080p ou plus petit, rien ne s'affiche (demande des
# utilisateurs : ne pas questionner pour rien).
# ==============================================================================

# Renvoie « LARGEUR HAUTEUR DPI » de l'écran le plus grand, ou rien si la
# résolution est inconnue. Ordre de détection : xrandr (X11 et XWayland :
# c'est la résolution que Wine voit), puis xdpyinfo, puis /sys/class/drm
# (résolution native du noyau, sans session graphique requise).
detect_screen_resolution() {
    local line mode w h mm mmw mmh dpi area
    local best_w=0 best_h=0 best_dpi=0 best_area=0

    # 1) xrandr : « SORTIE connected 3840x2160+0+0 ... 600mm x 340mm »
    if check_command xrandr; then
        while IFS= read -r line; do
            mode="$(grep -oE '[0-9]{3,5}x[0-9]{3,5}\+[0-9]+\+[0-9]+' <<< "$line" | head -n1)"
            [ -n "$mode" ] || continue

            w="${mode%%x*}"
            h="${mode#*x}"; h="${h%%+*}"
            case "$w$h" in ''|*[!0-9]*) continue ;; esac

            dpi=0
            mm="$(grep -oE '[0-9]{2,4}mm x [0-9]{2,4}mm' <<< "$line" | head -n1)"
            if [ -n "$mm" ]; then
                mmw="${mm%%mm*}"
                mmh="${mm##* x }"; mmh="${mmh%%mm*}"
                case "$mmw" in ''|*[!0-9]*) mmw=0 ;; esac
                case "$mmh" in ''|*[!0-9]*) mmh=0 ;; esac
                if [ "$mmw" -gt 10 ] && [ "$mmh" -gt 10 ]; then
                    dpi="$(awk -v w="$w" -v mmw="$mmw" 'BEGIN{printf "%.0f", w/(mmw/25.4)}')"
                fi
            fi

            area=$((w * h))
            if [ "$area" -gt "$best_area" ]; then
                best_area="$area"; best_w="$w"; best_h="$h"; best_dpi="$dpi"
            fi
        done < <(xrandr --current 2>/dev/null | grep ' connected')
    fi

    # 2) xdpyinfo : « dimensions: 3840x2160 pixels (600mm x 340mm) »
    if [ "$best_w" -eq 0 ] && check_command xdpyinfo; then
        line="$(xdpyinfo 2>/dev/null | grep -m1 'dimensions:')"
        mode="$(grep -oE '[0-9]{3,5}x[0-9]{3,5}' <<< "$line" | head -n1)"
        if [ -n "$mode" ]; then
            best_w="${mode%%x*}"
            best_h="${mode#*x}"
            case "$best_w$best_h" in ''|*[!0-9]*) best_w=0; best_h=0 ;; esac
        fi
        if [ "$best_w" -gt 0 ]; then
            mm="$(grep -oE '\([0-9]{2,4}mm x [0-9]{2,4}mm\)' <<< "$line" | head -n1)"
            if [ -n "$mm" ]; then
                mmw="${mm%%mm*}"; mmw="${mmw#(}"
                case "$mmw" in ''|*[!0-9]*) mmw=0 ;; esac
                if [ "$mmw" -gt 10 ]; then
                    best_dpi="$(awk -v w="$best_w" -v mmw="$mmw" 'BEGIN{printf "%.0f", w/(mmw/25.4)}')"
                fi
            fi
        fi
    fi

    # 3) Noyau : /sys/class/drm/cardX-CONNECTEUR/modes (première ligne = mode
    #    préféré). Résolution native, même sans session graphique.
    if [ "$best_w" -eq 0 ]; then
        local f
        for f in /sys/class/drm/card*-*/modes; do
            [ -r "$f" ] || continue
            IFS= read -r mode < "$f" || continue
            case "$mode" in
                [0-9]*x[0-9]*) ;;
                *) continue ;;
            esac
            w="${mode%%x*}"
            h="${mode#*x}"
            case "$w$h" in ''|*[!0-9]*) continue ;; esac
            area=$((w * h))
            if [ "$area" -gt "$best_area" ]; then
                best_area="$area"; best_w="$w"; best_h="$h"; best_dpi=0
            fi
        done
    fi

    [ "$best_w" -gt 0 ] || return 1
    echo "$best_w $best_h $best_dpi"
    return 0
}

# Facteur d'agrandissement conseillé (1.25, 1.5, 1.75 ou 2), ou 1.0 si
# inutile. Base : dimensions relatives à un écran 1080p ; affinée par la
# densité de pixels quand elle est connue (portables haute résolution).
# Borné à 200 %.
hidpi_recommended_factor() {
    local w="$1" h="$2" dpi="${3:-0}"

    awk -v w="$w" -v h="$h" -v dpi="$dpi" 'BEGIN {
        r = (w / 1920 > h / 1080) ? w / 1920 : h / 1080
        if (dpi >= 150 && dpi / 96 > r) r = dpi / 96
        if (r < 1.25) { print "1.0"; exit }
        if (r > 2) r = 2
        printf "%.2f", int(r * 4 + 0.5) / 4
    }'
}

# Écrit le DPI dans le registre Wine (équivalent du curseur « Résolution de
# l'écran » de winecfg) et mémorise le choix dans le préfixe pour ne plus
# poser la question aux exécutions suivantes.
apply_hidpi_scaling() {
    local dpi="$1" pourcent="$2"
    local marker="$WINEPREFIX/.pronote-echelle"

    local reg_dir="$WINEPREFIX/drive_c/windows/temp"
    mkdir -p "$reg_dir" 2>/dev/null || true
    local reg_file="$reg_dir/pronote-hidpi.reg"
    local reg_win_path='C:\windows\temp\pronote-hidpi.reg'

    if ! {
        printf 'Windows Registry Editor Version 5.00\r\n\r\n' > "$reg_file"
        printf '[HKEY_CURRENT_USER\\Control Panel\\Desktop]\r\n' >> "$reg_file"
        printf '"LogPixels"=dword:%08x\r\n\r\n' "$dpi" >> "$reg_file"
    } >>"$LOG_FILE" 2>&1; then
        log_warning "Impossible d'écrire le fichier de registre (disque plein ?) : mise à l'échelle ignorée."
        print_disk_hint_if_low
        return 0
    fi

    if import_reg_file "$reg_file" "$reg_win_path"; then
        echo "$pourcent" > "$marker" 2>/dev/null || true
        HIDPI_APPLIED="$pourcent"
        log_success "Mise à l'échelle appliquée : caractères agrandis à ${pourcent} % (${dpi} DPI)."
        log_info "Pour la modifier plus tard : WINEPREFIX=\"$WINEPREFIX\" winecfg,"
        log_info "onglet « Affichage », curseur « Résolution de l'écran »."
    else
        log_warning "L'écriture du DPI dans le registre Wine a échoué : taille d'origine conservée."
    fi

    rm -f "$reg_file" 2>/dev/null || true

    # Laisse le wineserver écrire le registre (quelques secondes, borné)
    settle_wine_session 10
    return 0
}

# Détecte un écran haute résolution et propose, le cas échéant, d'agrandir
# les caractères de Pronote (simple question O/N, compréhensible par tous).
configure_hidpi_scaling() {
    local marker="$WINEPREFIX/.pronote-echelle"

    # Choix déjà exprimé pour ce préfixe : on ne redemande pas.
    if [ -f "$marker" ]; then
        local precedent
        precedent="$(cat "$marker" 2>/dev/null || true)"
        if [ "$precedent" = "refuse" ]; then
            log_info "Mise à l'échelle des caractères : choix précédent « non » conservé pour ce préfixe."
        else
            log_info "Mise à l'échelle des caractères déjà configurée pour ce préfixe (${precedent:-?} %)."
        fi
        return 0
    fi

    local res w h dpi facteur
    res="$(detect_screen_resolution)" || res=""

    if [ -z "$res" ]; then
        log_info "Résolution de l'écran inconnue : la mise à l'échelle des caractères n'est pas proposée."
        return 0
    fi

    w="${res%% *}"
    h="${res#* }"; h="${h%% *}"
    dpi="${res##* }"

    facteur="$(hidpi_recommended_factor "$w" "$h" "$dpi")"

    # Écran 1080p ou plus petit : aucune question, la taille par défaut
    # (Arial 11, équivalent Windows) convient.
    if [ "$facteur" = "1.0" ] || [ -z "$facteur" ]; then
        return 0
    fi

    local pourcent dpi_cible reponse
    pourcent="$(awk -v f="$facteur" 'BEGIN{printf "%.0f", f*100}')"
    dpi_cible="$(awk -v f="$facteur" 'BEGIN{printf "%.0f", f*96}')"

    echo ""
    log_info "Écran haute résolution détecté : ${w}x${h} pixels."
    echo "Sur ce type d'écran (QHD ou 4K), les caractères de Pronote peuvent paraître"
    echo "très petits : Wine n'applique pas de mise à l'échelle par défaut."
    echo "Il est possible d'agrandir automatiquement la taille des caractères"
    echo "(équivalent d'un réglage Windows à ${pourcent} %)."
    echo ""

    reponse="$(ask_user "Agrandir les caractères de Pronote pour une meilleure lisibilité ? [O/n] : " "O")"

    case "$reponse" in
        n|N|non|Non|NON)
            echo "refuse" > "$marker" 2>/dev/null || true
            log_info "Taille d'origine conservée (modifiable plus tard via winecfg, onglet « Affichage »)."
            ;;
        *)
            apply_hidpi_scaling "$dpi_cible" "$pourcent"
            ;;
    esac

    return 0
}

# ==============================================================================
# Installation de Pronote
# ==============================================================================

download_pronote_installer() {
    local dest="$1"
    local -a candidates=("$PRONOTE_URL")

    local default_url="https://tele3.index-education.com/telechargement/pn/v${DEFAULT_YEAR}.0/exe/Install_PRNclient_FR_${DEFAULT_VERSION}_win${PRONOTE_ARCH}.exe"
    if [ "$default_url" != "$PRONOTE_URL" ]; then
        candidates+=("$default_url")
    fi

    local url size
    for url in "${candidates[@]}"; do
        log_info "Tentative de téléchargement : $url"
        rm -f "$dest" 2>/dev/null || true

        if download_file "$url" "$dest" 1; then
            size="$(wc -c < "$dest" 2>/dev/null || echo 0)"
            if [ "$size" -gt 20000000 ] && head -c 2 "$dest" 2>/dev/null | grep -q 'MZ'; then
                log_success "Téléchargement réussi depuis : $url"
                PRONOTE_URL="$url"
                return 0
            fi
            log_warning "Fichier téléchargé invalide ou incomplet ($size octets)."
            print_disk_hint_if_low
        else
            log_warning "Échec du téléchargement depuis cette adresse."
        fi
    done

    return 1
}

# winemenubuilder est désactivé pendant l'installateur : Wine ne crée ainsi
# pas d'entrées de menu « Client PRONOTE » en doublon (dossier Wine des menus
# MATE/XFCE), le script fournit ses propres raccourcis.
run_pronote_installer() {
    local installer="$1"
    local st=0

    (
        export WINEDLLOVERRIDES="winemenubuilder.exe=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
        "$WINE_BIN" "$installer"
    ) || st=$?

    return "$st"
}

# Après l'installateur : fermeture rapide de la session Wine.
# La v4.7 attendait jusqu'à 5 minutes (wineserver -w) alors que le service de
# mise à jour Index Éducation, démarré par l'installateur, ne s'arrête jamais
# seul : c'était la longue attente silencieuse vue sur CachyOS et Fedora.
# Ici : Pronote est fermé proprement s'il a été lancé par l'installateur, puis
# la session est close en 10 s au plus (wineserver -k : registre sauvegardé).
finish_pronote_wine_session() {
    echo ""
    log_info "Dernière étape Wine : arrêt des processus lancés par l'installateur (service de mise à jour)."
    log_info "Cela prend quelques secondes : merci de ne pas fermer cette fenêtre."

    if [ -n "$(prefix_wine_pids "PRONOTE")" ]; then
        log_warning "Pronote a été lancé par l'installateur : il va être fermé (relancez-le ensuite depuis le menu)."
        run_with_timeout 20 "$WINE_BIN" taskkill /IM "Client PRONOTE.exe" >/dev/null 2>&1 || true

        local i
        for ((i = 0; i < 10; i++)); do
            [ -z "$(prefix_wine_pids "PRONOTE")" ] && break
            sleep 1
        done
    fi

    finish_wine_session 10 "Fermeture des processus Wine (ne fermez pas cette fenêtre)"
}

cleanup_wine_menu_entries() {
    # Entrées créées par winemenubuilder (Wine) : doublons du lanceur Pronote
    if [ -d "$APP_DIR/wine" ]; then
        find "$APP_DIR/wine" \( -iname "*PRONOTE*" -o -iname "*Index Education*" \) -prune -exec rm -rf {} + 2>/dev/null || true
    fi

    if [ -d "$MENU_MERGE_DIR" ]; then
        find "$MENU_MERGE_DIR" -maxdepth 1 -type f -name "wine-Programs-*" \
            \( -iname "*PRONOTE*" -o -iname "*Index Education*" \) -delete 2>/dev/null || true
    fi

    if [ -d "$HOME/.local/share/desktop-directories" ]; then
        find "$HOME/.local/share/desktop-directories" -maxdepth 1 -type f -name "wine-Programs-*" \
            \( -iname "*PRONOTE*" -o -iname "*Index Education*" \) -delete 2>/dev/null || true
    fi

    return 0
}

install_pronote() {
    echo ""
    box_title "Installation de Pronote Client" 46
    echo ""

    check_disk_space_before_pronote

    local installer="$TMP_DIR/InstallPronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}.exe"

    log_info "Téléchargement de l'installateur Pronote ${PRONOTE_ARCH} bits..."
    echo "URL cible : $PRONOTE_URL"
    echo ""

    if ! download_pronote_installer "$installer"; then
        log_error "Téléchargement de l'installateur Pronote impossible."
        echo "Vérifiez la connexion Internet, ou fournissez une URL avec --url."
        exit 1
    fi

    echo ""
    log_warning "Une fenêtre d'installation Windows va s'ouvrir."
    echo ""
    echo "Conseils pendant l'installation :"
    echo "  • Décochez si possible : « Créer un raccourci sur le bureau »."
    echo "  • Décochez si possible : « Lancer Pronote après installation »."
    echo "  • Laissez l'installation se terminer normalement."
    echo ""
    echo "Remarque : une fois l'assistant fermé, l'installation peut encore"
    echo "travailler quelques secondes en arrière-plan (enregistrement de"
    echo "services). Le message affiché vous préviendra automatiquement."
    echo "Le script ferme ensuite lui-même les processus Wine restants"
    echo "(quelques secondes, barre animée) : ne fermez pas cette fenêtre."
    echo ""

    if [ "$ASSUME_YES" != "1" ]; then
        read -rp "Appuyez sur Entrée pour lancer l'installateur..." _
    fi

    RUN_QUIET_ACTIVITY_DIR="$WINEPREFIX/drive_c"
    RUN_QUIET_FINALIZE_MSG="Installation Pronote : finalisation en arrière-plan (services), veuillez patienter..."

    if ! run_quiet "Installation Pronote en cours" run_pronote_installer "$installer"; then
        log_warning "L'installateur Pronote s'est terminé avec un code d'erreur : vérification de l'installation..."
    fi

    RUN_QUIET_ACTIVITY_DIR=""
    RUN_QUIET_FINALIZE_MSG=""

    finish_pronote_wine_session

    local exe
    exe="$(find_pronote_exe)"

    if [ -z "$exe" ]; then
        log_error "Pronote ne semble pas installé : exécutable introuvable dans $WINEPREFIX/drive_c."
        echo "Causes fréquentes : assistant annulé, espace disque insuffisant, préfixe Wine incomplet."
        print_disk_hint_if_low
        print_wine_diagnostics
        exit 1
    fi

    log_success "Pronote installé : $exe"
    cleanup_wine_menu_entries
    return 0
}

# ==============================================================================
# Icônes : téléchargement, conversion, extraction, secours
# ==============================================================================

is_image_file() {
    local f="$1"
    [ -s "$f" ] || return 1

    local mime=""
    if check_command file; then
        mime="$(file -b --mime-type "$f" 2>/dev/null || true)"
        case "$mime" in
            image/*)
                return 0
                ;;
            text/html|application/json)
                return 1
                ;;
        esac
    fi

    # Repli : signatures binaires
    local magic=""
    magic="$(head -c 8 "$f" 2>/dev/null | od -An -tx1 2>/dev/null | tr -d ' \n' || true)"
    case "$magic" in
        ffd8ff*)               return 0 ;;   # JPEG
        89504e47*)             return 0 ;;   # PNG
        47494638*)             return 0 ;;   # GIF
        52494646*)             return 0 ;;   # WEBP (RIFF)
        00000100*)             return 0 ;;   # ICO
        3c3f786d6c*|3c737667*) return 0 ;;   # <?xml / <svg
    esac

    if grep -qi "<svg" "$f" 2>/dev/null; then
        return 0
    fi

    return 1
}

# ImageMagick 7 (magick) ou 6 (convert)
image_magick_cmd() {
    if check_command magick; then
        echo "magick"
    elif check_command convert; then
        echo "convert"
    else
        echo ""
    fi
}

im_identify_largest_index() {
    local src="$1"
    local im
    im="$(image_magick_cmd)"
    [ -n "$im" ] || { echo "0"; return 0; }

    local idx=""
    if [ "$im" = "magick" ]; then
        idx="$(magick identify -format "%p %w\n" "$src" 2>/dev/null | sort -k2 -n | tail -n1 | awk '{print $1}' || true)"
    else
        idx="$(identify -format "%p %w\n" "$src" 2>/dev/null | sort -k2 -n | tail -n1 | awk '{print $1}' || true)"
    fi

    case "$idx" in
        ''|*[!0-9]*) echo "0" ;;
        *)           echo "$idx" ;;
    esac
}

rasterize_svg_to_pngs() {
    local svg="$1"
    local name="$2"
    local size ok=0 out_dir im
    im="$(image_magick_cmd)"

    for size in "${ICON_SIZES[@]}"; do
        out_dir="$ICON_BASE_DIR/${size}x${size}/apps"
        mkdir -p "$out_dir"

        if check_command rsvg-convert; then
            if rsvg-convert -w "$size" -h "$size" "$svg" -o "$out_dir/${name}.png" >/dev/null 2>&1 && [ -s "$out_dir/${name}.png" ]; then
                ok=1
                continue
            fi
        fi

        if [ -n "$im" ]; then
            if "$im" -background none -density 384 "$svg" -resize "${size}x${size}" -gravity center -extent "${size}x${size}" "PNG32:$out_dir/${name}.png" >/dev/null 2>&1 && [ -s "$out_dir/${name}.png" ]; then
                ok=1
            else
                rm -f "$out_dir/${name}.png" 2>/dev/null || true
            fi
        fi
    done

    [ "$ok" -eq 1 ]
}

convert_image_to_pngs() {
    local src="$1"
    local name="$2"
    local size ok=0 out_dir

    if grep -qi "<svg" "$src" 2>/dev/null; then
        rasterize_svg_to_pngs "$src" "$name" && return 0
        return 1
    fi

    local im
    im="$(image_magick_cmd)"
    [ -n "$im" ] || return 1

    for size in "${ICON_SIZES[@]}"; do
        out_dir="$ICON_BASE_DIR/${size}x${size}/apps"
        mkdir -p "$out_dir"

        if "$im" "${src}[0]" -alpha set -background none -resize "${size}x${size}" -gravity center -extent "${size}x${size}" "PNG32:$out_dir/${name}.png" >/dev/null 2>&1 && [ -s "$out_dir/${name}.png" ]; then
            ok=1
        else
            rm -f "$out_dir/${name}.png" 2>/dev/null || true
        fi
    done

    [ "$ok" -eq 1 ] && [ -s "$ICON_BASE_DIR/256x256/apps/${name}.png" ]
}

extract_icon_from_exe() {
    local exe="$1"
    local name="$2"

    [ -f "$exe" ] || return 1
    check_command wrestool || return 1

    local work="$TMP_DIR/exe-icon"
    rm -rf "$work"
    mkdir -p "$work/png"

    wrestool -x -t 14 -o "$work" "$exe" >/dev/null 2>&1 || true

    local best_ico=""
    best_ico="$(ls -S "$work"/*.ico 2>/dev/null | head -n1 || true)"
    if [ -z "$best_ico" ] || [ ! -s "$best_ico" ]; then
        return 1
    fi

    local best_png=""
    if check_command icotool; then
        icotool -x -o "$work/png" "$best_ico" >/dev/null 2>&1 || true
        best_png="$(ls -S "$work"/png/*.png 2>/dev/null | head -n1 || true)"
    fi

    if [ -n "$best_png" ] && [ -s "$best_png" ]; then
        if convert_image_to_pngs "$best_png" "$name"; then
            return 0
        fi
        # Sans ImageMagick : copie brute du plus grand PNG extrait
        local out_dir="$ICON_BASE_DIR/256x256/apps"
        mkdir -p "$out_dir"
        cp -f "$best_png" "$out_dir/${name}.png" 2>/dev/null && return 0
    fi

    local im
    im="$(image_magick_cmd)"
    if [ -n "$im" ]; then
        local idx tmp_png="$work/best.png"
        idx="$(im_identify_largest_index "$best_ico")"
        if "$im" "${best_ico}[${idx}]" "PNG32:$tmp_png" >/dev/null 2>&1 && [ -s "$tmp_png" ]; then
            convert_image_to_pngs "$tmp_png" "$name" && return 0
        fi
    fi

    return 1
}

# Icône de secours : papillon stylisé, sans texte (rendu identique partout,
# plus de lettre aléatoire due à une police absente).
write_fallback_svg() {
    local target="$1"
    mkdir -p "$(dirname "$target")"

    cat > "$target" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1e8f5a"/>
      <stop offset="1" stop-color="#0d5f3b"/>
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="240" height="240" rx="52" fill="url(#bg)"/>
  <g fill="#ffffff" fill-opacity="0.96">
    <path d="M128 78 C 96 40, 40 44, 42 92 C 44 128, 92 136, 122 122 Z"/>
    <path d="M128 78 C 160 40, 216 44, 214 92 C 212 128, 164 136, 134 122 Z"/>
    <path d="M124 132 C 92 128, 52 150, 62 188 C 70 216, 110 208, 124 176 Z"/>
    <path d="M132 132 C 164 128, 204 150, 194 188 C 186 216, 146 208, 132 176 Z"/>
  </g>
  <rect x="122" y="70" width="12" height="118" rx="6" fill="#0a3f28"/>
  <circle cx="128" cy="66" r="10" fill="#0a3f28"/>
  <path d="M120 60 C 108 48, 104 38, 106 30" stroke="#0a3f28" stroke-width="5" fill="none" stroke-linecap="round"/>
  <path d="M136 60 C 148 48, 152 38, 150 30" stroke="#0a3f28" stroke-width="5" fill="none" stroke-linecap="round"/>
</svg>
EOF
}

install_pixmap_and_xdg_icon() {
    local name="$1"
    mkdir -p "$PIXMAP_DIR"

    local src="" size f
    for size in 256 128 64 48; do
        if [ -s "$ICON_BASE_DIR/${size}x${size}/apps/${name}.png" ]; then
            src="$ICON_BASE_DIR/${size}x${size}/apps/${name}.png"
            break
        fi
    done

    if [ -n "$src" ]; then
        cp -f "$src" "$PIXMAP_DIR/${name}.png" 2>/dev/null || true
    fi

    if check_command xdg-icon-resource; then
        for size in "${ICON_SIZES[@]}"; do
            f="$ICON_BASE_DIR/${size}x${size}/apps/${name}.png"
            [ -s "$f" ] || continue
            xdg-icon-resource install --noupdate --novendor --size "$size" "$f" "$name" >/dev/null 2>&1 || true
        done
    fi

    return 0
}

refresh_icon_caches() {
    if check_command gtk-update-icon-cache; then
        gtk-update-icon-cache -f -t "$ICON_BASE_DIR" >/dev/null 2>&1 || true
    fi
    if check_command xdg-icon-resource; then
        xdg-icon-resource forceupdate >/dev/null 2>&1 || true
    fi
    if check_command update-desktop-database; then
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    fi

    rm -f "$HOME/.cache/icon-cache.kcache" 2>/dev/null || true
    if check_command kbuildsycoca6; then
        kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
    fi
    if check_command kbuildsycoca5; then
        kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
    fi

    touch "$ICON_BASE_DIR" "$APP_DIR" 2>/dev/null || true
    return 0
}

resolve_icon_path() {
    local name="$1"
    local size f

    for size in 256 128 64 48 32; do
        f="$ICON_BASE_DIR/${size}x${size}/apps/${name}.png"
        if [ -s "$f" ]; then
            echo "$f"
            return 0
        fi
    done

    if [ -s "$PIXMAP_DIR/${name}.png" ]; then
        echo "$PIXMAP_DIR/${name}.png"
        return 0
    fi

    if [ -s "$ICON_DIR/${name}.svg" ]; then
        echo "$ICON_DIR/${name}.svg"
        return 0
    fi

    echo "$name"
}

fetch_and_install_icon() {
    local exe_path="$1"
    local icon_name="pronote-${PRONOTE_YEAR}"
    local svg_target="$ICON_DIR/${icon_name}.svg"
    local img_tmp="$TMP_DIR/pronote-icon-source"
    local got_raster=0

    mkdir -p "$ICON_DIR" "$PIXMAP_DIR"

    log_info "Téléchargement de l'icône Pronote par défaut..."

    local url
    for url in "${PRONOTE_ICON_URLS[@]}"; do
        rm -f "$img_tmp" 2>/dev/null || true

        if download_file "$url" "$img_tmp" && is_image_file "$img_tmp"; then
            if convert_image_to_pngs "$img_tmp" "$icon_name"; then
                got_raster=1
                log_success "Icône PNG installée depuis l'image par défaut."
                break
            fi
            log_warning "Image téléchargée mais conversion en PNG impossible."
            if [ -z "$(image_magick_cmd)" ] && ! check_command rsvg-convert; then
                break
            fi
        fi
    done

    if [ "$got_raster" -eq 0 ]; then
        log_warning "Icône par défaut indisponible, invalide ou non convertible."
        log_info "Tentative d'extraction de l'icône embarquée dans le programme..."
        if extract_icon_from_exe "$exe_path" "$icon_name"; then
            got_raster=1
            log_success "Icône extraite directement du programme Pronote."
        else
            log_warning "Extraction depuis l'exécutable impossible (icoutils ou ImageMagick absents ?)."
        fi
    fi

    if [ "$got_raster" -eq 0 ]; then
        log_warning "Aucune icône récupérée : création d'une icône de secours (SVG sans texte)."
        write_fallback_svg "$svg_target"
        rasterize_svg_to_pngs "$svg_target" "$icon_name" || true
    else
        # Ne pas laisser une ancienne icône de secours vectorielle prendre le
        # dessus sur les PNG dans certains thèmes.
        rm -f "$svg_target" 2>/dev/null || true
    fi

    install_pixmap_and_xdg_icon "$icon_name"
    refresh_icon_caches

    echo "$icon_name"
}

find_pronote_exe() {
    local -a files=()
    local f

    while IFS= read -r -d '' f; do
        if [[ "$f" == *"$PRONOTE_YEAR"* ]]; then
            files+=("$f")
        fi
    done < <(find "$WINEPREFIX/drive_c" -type f -iname "Client PRONOTE*.exe" -print0 2>/dev/null || true)

    if [ "${#files[@]}" -eq 0 ]; then
        while IFS= read -r -d '' f; do
            files+=("$f")
        done < <(find "$WINEPREFIX/drive_c" -type f -iname "Client PRONOTE*.exe" -print0 2>/dev/null || true)
    fi

    if [ "${#files[@]}" -eq 0 ]; then
        while IFS= read -r -d '' f; do
            case "$f" in
                *[Uu]nins*|*[Ii]nstall*) ;;
                *) files+=("$f") ;;
            esac
        done < <(find "$WINEPREFIX/drive_c" -type f -ipath "*Index Education*" -iname "*PRONOTE*.exe" -print0 2>/dev/null || true)
    fi

    echo "${files[0]:-}"
}

# ==============================================================================
# Ajout de ~/.local/bin au PATH
# ==============================================================================

ensure_path_in_shell_rc() {
    local shell_name
    shell_name="$(basename "${SHELL:-/bin/bash}")"

    local rc_file=""
    local path_line=""

    case "$shell_name" in
        zsh)
            rc_file="$HOME/.zshrc"
            path_line="export PATH=\"$BIN_DIR:\$PATH\""
            ;;
        fish)
            rc_file="$HOME/.config/fish/config.fish"
            path_line="set -gx PATH \$PATH $BIN_DIR"
            ;;
        *)
            rc_file="$HOME/.bashrc"
            path_line="export PATH=\"$BIN_DIR:\$PATH\""
            ;;
    esac

    local files_to_patch=("$rc_file")
    if [ "$shell_name" != "fish" ]; then
        files_to_patch+=("$HOME/.profile")
    fi

    local f
    for f in "${files_to_patch[@]}"; do
        if [ -f "$f" ] && grep -qF "$BIN_DIR" "$f" 2>/dev/null; then
            log_info "Le dossier '$BIN_DIR' est déjà configuré dans $f."
            continue
        fi

        mkdir -p "$(dirname "$f")"
        touch "$f"
        {
            echo ""
            echo "# >>> Ajouté par le script d'installation Pronote >>>"
            echo "# Commandes : pronote-${PRONOTE_YEAR}, pronote-desinstaller-32, pronote-desinstaller-64"
            echo "$path_line"
            echo "# <<< Fin ajout Pronote <<<"
        } >> "$f"

        log_success "Le dossier '$BIN_DIR' a été ajouté au PATH dans $f."
    done

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) export PATH="$BIN_DIR:$PATH" ;;
    esac

    hash -r 2>/dev/null || true
}

# ==============================================================================
# Raccourci bureau universel
# ==============================================================================

create_desktop_shortcut() {
    local desktop_file_source="$1"
    local -a desktop_dirs=()
    local d

    if check_command xdg-user-dir; then
        d="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
        # xdg-user-dir renvoie $HOME quand aucun dossier Bureau n'est configuré
        if [ -n "$d" ] && [ -d "$d" ] && [ "$d" != "$HOME" ]; then
            desktop_dirs+=("$d")
        fi
    fi

    for d in "$HOME/Bureau" "$HOME/Desktop"; do
        [ -d "$d" ] || continue
        local already=0
        local existing
        for existing in ${desktop_dirs[@]+"${desktop_dirs[@]}"}; do
            [ "$existing" = "$d" ] && already=1
        done
        [ "$already" -eq 0 ] && desktop_dirs+=("$d")
    done

    if [ "${#desktop_dirs[@]}" -eq 0 ]; then
        log_info "Aucun dossier Bureau/Desktop détecté : raccourci bureau non créé."
        return 0
    fi

    local target
    for d in "${desktop_dirs[@]}"; do
        target="$d/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}.desktop"
        cp -f "$desktop_file_source" "$target"
        chmod +x "$target"
        if check_command gio; then
            gio set "$target" metadata::trusted true 2>/dev/null || true
        fi
        log_success "Raccourci bureau créé : $target"
    done
}

# ==============================================================================
# Menu XDG fusionné : force la présence dans « Éducation »
# ==============================================================================
# Certains menus (MATE notamment) n'affichaient pas Pronote dans Éducation
# malgré Categories=Education. Le fichier fusionné inclut explicitement les
# lanceurs 32 et 64 bits dans le sous-menu Education (nom standard commun à
# MATE, XFCE, Cinnamon, Budgie, KDE, LXQt). GNOME Shell l'ignore sans effet.
# ------------------------------------------------------------------------------

install_xdg_menu_entry() {
    mkdir -p "$MENU_MERGE_DIR"
    local menu_file="$MENU_MERGE_DIR/pronote-${PRONOTE_YEAR}.menu"

    cat > "$menu_file" << EOF
<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
 "http://www.freedesktop.org/standards/menu-spec/1.0/menu.dtd">
<!-- Généré par le script d'installation Pronote : place Pronote dans le
     menu Éducation (MATE, XFCE, Cinnamon, Budgie, KDE, LXQt). -->
<Menu>
  <Name>Applications</Name>
  <Menu>
    <Name>Education</Name>
    <Include>
      <Filename>pronote-${PRONOTE_YEAR}-64.desktop</Filename>
      <Filename>pronote-${PRONOTE_YEAR}-32.desktop</Filename>
    </Include>
  </Menu>
</Menu>
EOF

    log_success "Menu XDG (Éducation) créé : $menu_file"
}

# ==============================================================================
# Création des lanceurs et des raccourcis
# ==============================================================================

create_launchers() {
    echo ""
    box_title "Creation des raccourcis Linux" 46
    echo ""

    mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$PIXMAP_DIR"

    discover_wine_paths

    local real_exe
    real_exe="$(find_pronote_exe)"

    if [ -z "$real_exe" ]; then
        log_warning "Exécutable Pronote non trouvé automatiquement."
        log_warning "Un chemin générique sera utilisé."
        real_exe="$WINEPREFIX/drive_c/Program Files/Index Education/PRONOTE ${PRONOTE_YEAR}/Client PRONOTE.exe"
    fi

    log_info "Exécutable utilisé : $real_exe"

    local icon_name
    icon_name="$(fetch_and_install_icon "$real_exe")"

    local icon_file
    icon_file="$(resolve_icon_path "$icon_name")"
    log_info "Icône utilisée : $icon_file"

    local launcher="$BIN_DIR/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}"
    local launcher_log_dir="$HOME/.local/share/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}"
    local wine_for_launcher="${WINE_BIN:-wine}"
    local wineserver_for_launcher="${WINESERVER_BIN:-}"
    local wine_dir_for_launcher=""
    wine_dir_for_launcher="$(dirname "$wine_for_launcher")"

    mkdir -p "$launcher_log_dir"

    cat > "$launcher" << EOF
#!/usr/bin/env bash
# Lanceur Pronote ${PRONOTE_YEAR} (${PRONOTE_ARCH} bits) - généré automatiquement
export WINEPREFIX="$WINEPREFIX"
export WINEDEBUG="-nls"
export PATH="$wine_dir_for_launcher:\$PATH"

# Binaire Wine mémorisé à l'installation ; repli sur le PATH si le chemin a
# disparu (ex. NixOS après nix-collect-garbage, mise à jour de Wine).
WINE_CMD="$wine_for_launcher"
if [ ! -x "\$WINE_CMD" ]; then
    WINE_CMD="\$(command -v wine 2>/dev/null || echo wine)"
fi
export WINE="\$WINE_CMD"
$(if [ -n "$wineserver_for_launcher" ]; then echo "[ -x \"$wineserver_for_launcher\" ] && export WINESERVER=\"$wineserver_for_launcher\""; fi)

LOG_DIR="$launcher_log_dir"
LOG_FILE="\$LOG_DIR/pronote.log"
mkdir -p "\$LOG_DIR"

if [ -f "\$LOG_FILE" ] && [ "\$(wc -c < "\$LOG_FILE" 2>/dev/null || echo 0)" -gt 10485760 ]; then
    mv "\$LOG_FILE" "\$LOG_FILE.1" 2>/dev/null || true
fi

CACHE_DIRS=(
    "\$WINEPREFIX/drive_c/ProgramData/IndexEducation/PRONOTE/CLIENT"
    "\$WINEPREFIX/drive_c/users/\$USER/AppData/Local/IndexEducation/PRONOTE/CLIENT"
)

for CACHE_DIR in "\${CACHE_DIRS[@]}"; do
    if [ -d "\$CACHE_DIR" ]; then
        find "\$CACHE_DIR" -type d -name "Cache" -exec rm -rf {} + 2>/dev/null || true
    fi
done

if [ "\${PRONOTE_DEBUG:-0}" = "1" ]; then
    exec "\$WINE_CMD" "$real_exe" "\$@"
else
    exec "\$WINE_CMD" "$real_exe" "\$@" >> "\$LOG_FILE" 2>&1
fi
EOF

    chmod +x "$launcher"
    ln -sf "$launcher" "$BIN_DIR/pronote-${PRONOTE_YEAR}"
    log_success "Lanceur créé : $launcher"
    log_info "Alias générique : $BIN_DIR/pronote-${PRONOTE_YEAR}"

    local desktop_file="$APP_DIR/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}.desktop"

    cat > "$desktop_file" << EOF
[Desktop Entry]
Version=1.0
Name=Pronote Client ${PRONOTE_YEAR} (${PRONOTE_ARCH} bits)
GenericName=Client Pronote
Comment=Client Pronote ${PRONOTE_YEAR} ${PRONOTE_ARCH} bits via Wine
Exec=$launcher
Terminal=false
Type=Application
Icon=$icon_file
Categories=Education;Office;
StartupNotify=true
Keywords=pronote;education;ecole;college;lycee;
EOF

    chmod +x "$desktop_file"
    log_success "Raccourci menu créé : $desktop_file"

    install_xdg_menu_entry

    update-desktop-database "$APP_DIR" 2>/dev/null || true
    refresh_icon_caches

    cleanup_wine_menu_entries

    ensure_path_in_shell_rc

    create_desktop_shortcut "$desktop_file"

    create_uninstaller "$desktop_file" "$launcher" "$icon_file" "$icon_name"
    create_uninstaller_dispatcher
}

# ==============================================================================
# Désinstallateur (un par architecture)
# ==============================================================================

create_uninstaller() {
    local desktop_file="$1"
    local launcher="$2"
    local icon_file="$3"
    local icon_name="$4"
    local uninstaller="$BIN_DIR/pronote-desinstaller-${PRONOTE_ARCH}"
    local other_arch="64"
    local wine_for_uninstaller="${WINE_BIN:-wine}"

    if [ "$PRONOTE_ARCH" = "64" ]; then
        other_arch="32"
    fi
    local wineserver_for_uninstaller="${WINESERVER_BIN:-wineserver}"

    local q_wineprefix q_desktop_file q_launcher q_icon_file q_app_dir q_icon_base_dir q_icon_name q_pixmap_dir q_bin_dir q_wine q_wineserver
    printf -v q_wineprefix    '%q' "$WINEPREFIX"
    printf -v q_desktop_file  '%q' "$desktop_file"
    printf -v q_launcher      '%q' "$launcher"
    printf -v q_icon_file     '%q' "$icon_file"
    printf -v q_app_dir       '%q' "$APP_DIR"
    printf -v q_icon_base_dir '%q' "$ICON_BASE_DIR"
    printf -v q_icon_name     '%q' "$icon_name"
    printf -v q_pixmap_dir    '%q' "$PIXMAP_DIR"
    printf -v q_bin_dir       '%q' "$BIN_DIR"
    printf -v q_wine          '%q' "$wine_for_uninstaller"
    printf -v q_wineserver    '%q' "$wineserver_for_uninstaller"

    cat > "$uninstaller" << EOF
#!/usr/bin/env bash
# Désinstallateur Pronote ${PRONOTE_YEAR} (${PRONOTE_ARCH} bits) - généré automatiquement

set -u

WINEPREFIX=$q_wineprefix
DESKTOP_FILE=$q_desktop_file
LAUNCHER=$q_launcher
ICON_FILE=$q_icon_file
ICON_NAME=$q_icon_name
APP_DIR=$q_app_dir
ICON_BASE_DIR=$q_icon_base_dir
PIXMAP_DIR=$q_pixmap_dir
BIN_DIR=$q_bin_dir
WINE_BIN=$q_wine
WINESERVER_BIN=$q_wineserver
PRONOTE_ARCH="${PRONOTE_ARCH}"
LOG_DIR="\$HOME/.local/share/pronote-installer"
LOG_FILE="\$LOG_DIR/desinstallation-${PRONOTE_YEAR}-${PRONOTE_ARCH}-\$(date +%Y%m%d-%H%M%S).log"
WINEDEBUG_QUIET="-all"
SHOW_WINE_LOGS="0"
RUN_QUIET_ACTIVITY_DIR=""
RUN_QUIET_FINALIZE_MSG=""

export WINEPREFIX
export WINE="\$WINE_BIN"
export WINESERVER="\$WINESERVER_BIN"
export PATH="\$(dirname "\$WINE_BIN"):\$PATH"

GREEN='\\033[0;32m'
YELLOW='\\033[1;33m'
RED='\\033[0;31m'
BLUE='\\033[0;34m'
NC='\\033[0m'

log_info()    { echo -e "\${BLUE}ℹ️  \$*\${NC}" >&2; }
log_success() { echo -e "\${GREEN}✅ \$*\${NC}" >&2; }
log_warning() { echo -e "\${YELLOW}⚠️  \$*\${NC}" >&2; }
log_error()   { echo -e "\${RED}❌ \$*\${NC}" >&2; }

repeat_char() {
    local char="\$1"
    local count="\$2"
    local out=""
    local i

    for ((i = 0; i < count; i++)); do
        out+="\$char"
    done

    printf "%s" "\$out"
}

box_title() {
    local title="\$1"
    local width="\${2:-46}"
    local inner=\$((width - 2))

    printf "╔"
    repeat_char "═" "\$width"
    printf "╗\\n"

    printf "║ %-*s ║\\n" "\$inner" "\$title"

    printf "╚"
    repeat_char "═" "\$width"
    printf "╝\\n"
}

progress_pulse() {
    local label="\$1"
    local pid="\$2"
    local activity_dir="\${3:-}"
    local finalize_msg="\${4:-Finalisation en cours, veuillez patienter quelques secondes...}"

    local width=24
    local block=7
    local pos=0
    local direction=1
    local start_time=\$SECONDS

    local check_activity=0
    [ -n "\$activity_dir" ] && check_activity=1

    local last_size=-1
    local stable_count=0
    local finalized=0
    local sample_tick=0
    local current_label="\$label"

    if [ ! -t 1 ]; then
        while kill -0 "\$pid" 2>/dev/null; do
            sleep 1
        done
        return
    fi

    while kill -0 "\$pid" 2>/dev/null; do
        local bar=""
        local i
        local elapsed=\$((SECONDS - start_time))
        local min=\$((elapsed / 60))
        local sec=\$((elapsed % 60))

        if [ "\$check_activity" -eq 1 ] && [ "\$finalized" -eq 0 ]; then
            sample_tick=\$((sample_tick + 1))

            if (( sample_tick >= 8 )); then
                sample_tick=0
                local cur_size
                cur_size="\$(du -sk "\$activity_dir" 2>/dev/null | cut -f1)"
                cur_size="\${cur_size:-0}"

                if [ "\$cur_size" = "\$last_size" ]; then
                    stable_count=\$((stable_count + 1))
                else
                    stable_count=0
                fi
                last_size="\$cur_size"

                if (( stable_count >= 3 && elapsed >= 5 )); then
                    finalized=1
                    current_label="\$finalize_msg"
                fi
            fi
        fi

        for ((i = 0; i < width; i++)); do
            if (( i >= pos && i < pos + block )); then
                bar+="█"
            else
                bar+=" "
            fi
        done

        printf "\\r\\033[K%s [%s] %02d:%02d" "\$current_label" "\$bar" "\$min" "\$sec"

        pos=\$((pos + direction))

        if (( pos <= 0 )); then
            pos=0
            direction=1
        elif (( pos + block >= width )); then
            pos=\$((width - block))
            direction=-1
        fi

        sleep 0.12
    done

    printf "\\r\\033[K"
}

run_quiet() {
    local label="\$1"
    shift

    if [ "\$SHOW_WINE_LOGS" = "1" ]; then
        log_info "\$label"
        "\$@"
        return \$?
    fi

    mkdir -p "\$LOG_DIR"

    (
        export WINEDEBUG="\$WINEDEBUG_QUIET"
        "\$@"
    ) >>"\$LOG_FILE" 2>&1 &

    local pid=\$!
    progress_pulse "\$label" "\$pid" "\$RUN_QUIET_ACTIVITY_DIR" "\$RUN_QUIET_FINALIZE_MSG"

    wait "\$pid"
    local status=\$?

    if [ "\$status" -eq 0 ]; then
        log_success "\$label : terminé"
    else
        log_warning "\$label : terminé avec un code retour \$status"
        echo "Journal technique : \$LOG_FILE"
    fi

    return "\$status"
}

clear
box_title "Desinstallation de Pronote ${PRONOTE_YEAR} (${PRONOTE_ARCH} bits)" 46
echo ""

# ----------------------------------------------------------------------------
# Garde-fou : deux versions installées dans le MÊME préfixe Wine.
# C'était le cas sous NixOS avec les versions <= 4.8 (WINEPREFIX héritée par
# le re-exec nix-shell). La désinstallation ci-dessous retirerait alors
# Pronote pour les DEUX versions : on prévient avant d'aller plus loin.
# ----------------------------------------------------------------------------
OTHER_ARCH="${other_arch}"
OTHER_LAUNCHER="\$BIN_DIR/pronote-${PRONOTE_YEAR}-\$OTHER_ARCH"

if [ -f "\$OTHER_LAUNCHER" ]; then
    OTHER_PREFIX="\$(sed -n 's|^export WINEPREFIX=||p' "\$OTHER_LAUNCHER" 2>/dev/null | head -n1 | tr -d '"')"
    if [ -n "\$OTHER_PREFIX" ] && [ "\$OTHER_PREFIX" = "\$WINEPREFIX" ]; then
        echo ""
        log_error "Les versions ${PRONOTE_ARCH} et \$OTHER_ARCH bits partagent le même préfixe Wine :"
        echo "  \$WINEPREFIX"
        echo ""
        echo "Désinstaller depuis Wine retirerait Pronote pour les DEUX versions."
        echo "Solution : relancez le script d'installation pour l'une des deux versions"
        echo "(chacune obtiendra alors son propre préfixe), puis revenez ici."
        echo ""
        read -rp "Continuer quand même (Pronote sera retiré des deux versions) ? [o/N] : " FORCE_CHOICE
        case "\$FORCE_CHOICE" in
            o|O|oui|Oui|OUI|y|Y|yes|YES) ;;
            *)
                echo "Désinstallation annulée."
                exit 0
                ;;
        esac
    fi
fi

echo "Ce script va ouvrir l'outil Wine « Ajout/Suppression de programmes »."
echo ""
echo "Avant de commencer, assurez-vous que Pronote est fermé."
echo "Le script va aussi tenter de fermer automatiquement les processus Pronote/Wine"
echo "du préfixe suivant :"
echo "  \$WINEPREFIX"
echo ""
log_warning "Si d'autres applications Wine utilisent ce même préfixe, elles peuvent être fermées."
echo ""

read -rp "Continuer ? [O/n] : " CONTINUE
case "\$CONTINUE" in
    n|N|non|Non|NON)
        echo "Désinstallation annulée."
        exit 0
        ;;
esac

echo ""
echo "Par défaut, les messages techniques de Wine seront masqués."
read -rp "Afficher les messages techniques de Wine ? [o/N] : " DEBUG_CHOICE

case "\$DEBUG_CHOICE" in
    o|O|oui|Oui|OUI|y|Y|yes|YES)
        SHOW_WINE_LOGS="1"
        log_warning "Mode expert activé."
        ;;
    *)
        SHOW_WINE_LOGS="0"
        log_info "Mode silencieux activé."
        echo "Journal technique : \$LOG_FILE"
        ;;
esac

echo ""
log_info "Fermeture préventive des processus Pronote/Wine du préfixe..."

"\$WINE_BIN" taskkill /F /IM "Client PRONOTE.exe" >/dev/null 2>&1 || true
"\$WINE_BIN" taskkill /F /IM "PRONOTE.exe" >/dev/null 2>&1 || true
"\$WINE_BIN" taskkill /F /IM "IECefSubProcess.exe" >/dev/null 2>&1 || true

"\$WINESERVER_BIN" -k >/dev/null 2>&1 || true
sleep 1

echo ""
echo "Dans la fenêtre qui va s'ouvrir :"
echo ""
echo "  1. Sélectionnez « INDEX EDUCATION - Client PRONOTE ${PRONOTE_YEAR} » ou l'entrée Pronote équivalente."
echo "  2. Cliquez sur « Modifier/Supprimer »."
echo "  3. Suivez l'assistant de désinstallation jusqu'à son terme."
echo "  4. Remarque : l'assistant peut sembler avoir disparu prématurément ; le bouton « Modifier/Supprimer »"
echo "     apparait alors en grisé (non sélectionnable), même si Pronote est toujours visible dans la liste."
echo "     C'est normal : il suffit de cliquer sur « OK » pour fermer/terminer proprement."
echo ""
echo "Ensuite, relancez la commande pronote-desinstaller-${PRONOTE_ARCH} pour supprimer le service de mise à jour associé, présent"
echo "dans la même liste et nommé exactement :"
echo ""
echo "     « Mise à jour automatique - Index Education - »"
echo ""
echo "Sélectionnez-le puis cliquez à nouveau sur « Modifier/Supprimer »"
echo "(la remarque 4 ci-dessus s'applique aussi à cette étape)."
echo ""
echo "Quand tout est terminé, fermez la fenêtre « Ajout/Suppression de programmes »."
echo ""

read -rp "Appuyez sur Entrée pour ouvrir le désinstallateur Wine..."

RUN_QUIET_ACTIVITY_DIR="\$WINEPREFIX/drive_c"
RUN_QUIET_FINALIZE_MSG="Désinstallation : finalisation en cours, veuillez patienter quelques secondes..."

run_quiet "Desinstallation via Wine" "\$WINE_BIN" uninstaller || true

RUN_QUIET_ACTIVITY_DIR=""
RUN_QUIET_FINALIZE_MSG=""

echo ""
log_info "Nettoyage des raccourcis Linux..."

rm -f "\$DESKTOP_FILE" 2>/dev/null || true
rm -f "\$LAUNCHER" 2>/dev/null || true

GENERIC_LAUNCHER="\$BIN_DIR/pronote-${PRONOTE_YEAR}"
if [ -L "\$GENERIC_LAUNCHER" ] && [ "\$(readlink -f "\$GENERIC_LAUNCHER")" = "\$(readlink -f "\$LAUNCHER" 2>/dev/null)" ]; then
    rm -f "\$GENERIC_LAUNCHER" 2>/dev/null || true
fi

OTHER_DESKTOP_COUNT=\$(find "\$APP_DIR" -maxdepth 1 -name "pronote-${PRONOTE_YEAR}-*.desktop" 2>/dev/null | wc -l)

if [ "\$OTHER_DESKTOP_COUNT" -eq 0 ]; then
    rm -f "\$ICON_FILE" 2>/dev/null || true
    rm -f "\$PIXMAP_DIR/\${ICON_NAME}.png" 2>/dev/null || true
    rm -f "\$ICON_BASE_DIR/scalable/apps/\${ICON_NAME}.svg" 2>/dev/null || true

    for SIZE in 16 22 24 32 48 64 128 256; do
        rm -f "\$ICON_BASE_DIR/\${SIZE}x\${SIZE}/apps/\${ICON_NAME}.png" 2>/dev/null || true
    done

    # Fichier de menu XDG (Éducation) créé par l'installateur v4.7
    rm -f "\$HOME/.config/menus/applications-merged/pronote-${PRONOTE_YEAR}.menu" 2>/dev/null || true

    # Journal de l'ouvreur de pièces jointes (partagé entre les deux versions)
    rm -rf "\$HOME/.local/share/pronote-pieces-jointes" 2>/dev/null || true
else
    log_info "Une autre version de Pronote est encore installée : l'icône est conservée."
fi

DESKTOP_TARGETS=()
if command -v xdg-user-dir >/dev/null 2>&1; then
    XDG_DESK="\$(xdg-user-dir DESKTOP 2>/dev/null || true)"
    [ -n "\$XDG_DESK" ] && DESKTOP_TARGETS+=("\$XDG_DESK")
fi
DESKTOP_TARGETS+=("\$HOME/Bureau" "\$HOME/Desktop")

for DDIR in "\${DESKTOP_TARGETS[@]}"; do
    [ -d "\$DDIR" ] || continue
    rm -f "\$DDIR/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}.desktop" 2>/dev/null || true
    rm -f "\$DDIR/pronote-${PRONOTE_YEAR}.desktop" 2>/dev/null || true
done

if [ -d "\$APP_DIR/wine/Programs" ]; then
    find "\$APP_DIR/wine/Programs" -maxdepth 1 \\
        \\( -iname "PRONOTE*" -o -iname "Index Education*" \\) \\
        -exec rm -rf {} + 2>/dev/null || true
fi

# Menus Wine résiduels (winemenubuilder)
if [ -d "\$HOME/.config/menus/applications-merged" ]; then
    find "\$HOME/.config/menus/applications-merged" -maxdepth 1 -type f -name "wine-Programs-*" \\
        \\( -iname "*PRONOTE*" -o -iname "*Index Education*" \\) -delete 2>/dev/null || true
fi

update-desktop-database "\$APP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "\$ICON_BASE_DIR" 2>/dev/null || true
rm -f "\$HOME/.cache/icon-cache.kcache" 2>/dev/null || true
kbuildsycoca6 --noincremental 2>/dev/null || true
kbuildsycoca5 --noincremental 2>/dev/null || true

echo ""
log_success "Désinstallation terminée."
echo ""
echo "Vous pouvez maintenant fermer la fenêtre « Ajout/Suppression de programmes »"
echo "si elle est encore ouverte."
echo ""
echo "Remarque : le préfixe Wine n'a pas été supprimé :"
echo "  \$WINEPREFIX"
echo ""
echo "Pour supprimer entièrement ce préfixe Wine, uniquement si vous êtes sûr"
echo "qu'il ne contient pas d'autres logiciels utiles :"
echo "  rm -rf \\"\$WINEPREFIX\\""
echo ""
EOF

    chmod +x "$uninstaller"
    log_success "Désinstallateur créé : $uninstaller"

    ln -sf "$uninstaller" "$BIN_DIR/pronote-desinstaller-${PRONOTE_YEAR}-${PRONOTE_ARCH}"
}

create_uninstaller_dispatcher() {
    local dispatcher="$BIN_DIR/pronote-desinstaller"
    local q_bin_dir
    printf -v q_bin_dir '%q' "$BIN_DIR"

    cat > "$dispatcher" << EOF
#!/usr/bin/env bash
# Sélecteur de désinstallateur Pronote - généré automatiquement
set -u

BIN_DIR=$q_bin_dir

U32="\$BIN_DIR/pronote-desinstaller-32"
U64="\$BIN_DIR/pronote-desinstaller-64"

HAS32=0
HAS64=0
[ -x "\$U32" ] && HAS32=1
[ -x "\$U64" ] && HAS64=1

if [ "\$HAS32" -eq 0 ] && [ "\$HAS64" -eq 0 ]; then
    echo "Aucun désinstallateur Pronote trouvé dans \$BIN_DIR."
    exit 1
fi

if [ "\$#" -ge 1 ]; then
    case "\$1" in
        32) exec "\$U32" ;;
        64) exec "\$U64" ;;
    esac
fi

if [ "\$HAS32" -eq 1 ] && [ "\$HAS64" -eq 0 ]; then
    exec "\$U32"
fi

if [ "\$HAS64" -eq 1 ] && [ "\$HAS32" -eq 0 ]; then
    exec "\$U64"
fi

echo "Deux versions de Pronote sont installées."
echo ""
echo "  1) Désinstaller la version 64 bits  (pronote-desinstaller-64)"
echo "  2) Désinstaller la version 32 bits  (pronote-desinstaller-32)"
echo ""
read -rp "Votre choix [1/2] [Entrée=1] : " CHOICE

case "\${CHOICE:-1}" in
    2) exec "\$U32" ;;
    *) exec "\$U64" ;;
esac
EOF

    chmod +x "$dispatcher"
    log_success "Sélecteur de désinstallation créé : $dispatcher"
}

# ==============================================================================
# Aide / problèmes connus
# ==============================================================================

show_troubleshooting() {
    echo ""
    box_title "Problemes connus et solutions" 62
    echo ""
    echo "1. Erreur libcef.dll au démarrage, surtout avec deux écrans :"
    echo "   → Lancez : WINEPREFIX=\"$WINEPREFIX\" winecfg"
    echo "   → Onglet « Affichage » > cochez « Émuler un bureau virtuel » (ex. 1920x1080)."
    echo ""
    echo "2. Crash IECefSubProcess.exe :"
    echo "   → Essayez d'augmenter la résolution du bureau virtuel dans winecfg."
    echo ""
    echo "3. Pronote ne se lance qu'une seule fois :"
    echo "   → Le lanceur nettoie automatiquement certains caches au démarrage."
    echo "   → Si le problème persiste, relancez la session Linux."
    echo ""
    echo "4. Raccourci bureau non cliquable sous GNOME/Nautilus (icône avec cadenas) :"
    echo "   → Clic droit sur l'icône > « Autoriser le lancement »."
    echo ""
    echo "5. Diagnostic au lancement de Pronote :"
    echo "   → PRONOTE_DEBUG=1 pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}"
    echo ""
    echo "6. Une commande pronote-* semble « ne rien faire » (fréquent sur Mint XFCE) :"
    echo "   → Le PATH du terminal courant n'est pas encore à jour."
    echo "   → Faites simplement :  source ~/.bashrc && hash -r"
    echo "   → Ou ouvrez un nouveau terminal / reconnectez-vous."
    echo "   → En dernier recours, chemin complet :"
    echo "     $BIN_DIR/pronote-desinstaller-${PRONOTE_ARCH}"
    echo ""
    echo "7. « WINEARCH is set to win32 but this is not supported in wow64 mode » :"
    echo "   → Wine 10+/11 n'accepte plus les préfixes win32 purs."
    echo "   → Ce script installe alors Pronote 32 bits dans un préfixe WoW64 dédié."
    echo ""
    echo "8. Erreurs « No space left on device », « error=112 », « could not load kernel32.dll » :"
    echo "   → Disque (ou couche d'écriture du Live CD) plein pendant l'installation."
    echo "   → Libérez de l'espace, supprimez le préfixe incomplet puis relancez :"
    echo "     rm -rf \"$WINEPREFIX\""
    echo ""
    echo "9. L'entrée n'apparaît pas tout de suite dans le menu (MATE, XFCE, Budgie) :"
    echo "   → Déconnectez-vous / reconnectez-vous, ou relancez le tableau de bord"
    echo "     (MATE : mate-panel --replace &  |  XFCE : xfce4-panel -r)."
    echo ""
    echo "10. « Le préfixe Wine est incomplet » au premier lancement (Debian/Ubuntu/Mint) :"
    echo "    → Corrigé en v4.8 (wineserver est cherché dans /usr/lib/.../wine)."
    echo "    → Si cela se reproduit, relancez simplement le script : il réutilise le préfixe."
    echo ""
    echo "11. Des processus Wine (services Pronote) restent actifs après une session :"
    echo "    → WINEPREFIX=\"$WINEPREFIX\" ${WINESERVER_BIN:-wineserver} -k"
    echo ""
    echo "12. Les versions 32 et 64 bits se trouvent dans le MÊME préfixe Wine"
    echo "    (installation faite avec une version <= 4.8 du script) :"
    echo "    → Corrigé en v4.9 : chaque architecture a de nouveau son propre préfixe."
    echo "    → Relancez ce script pour chacune des deux versions : celle qui était"
    echo "      écrasée sera réinstallée proprement dans son préfixe séparé."
    echo ""
    echo "13. Une pièce jointe (PDF, Word, image...) ne s'ouvre pas :"
    echo "    → Journal des ouvertures : ~/.local/share/pronote-pieces-jointes/ouverture.log"
    echo "    → Images/PDF/vidéos s'ouvrent dans le navigateur ; documents bureautiques dans"
    echo "      LibreOffice/OnlyOffice ; sans suite bureautique installée, rien n'est ouvert."
    echo ""
    echo "14. Caractères trop petits ou trop grands (écran QHD / 4K) :"
    echo "    → WINEPREFIX=\"$WINEPREFIX\" winecfg"
    echo "    → Onglet « Affichage » > curseur « Résolution de l'écran » (96 = taille d'origine)."
    echo ""
    if [ "$IN_NIX_SHELL" = "1" ]; then
        echo "15. NixOS : Wine provient de nix-shell. Pour le pérenniser (nix-collect-garbage),"
        echo "    ajoutez wineWow64Packages.stable (ou wineWowPackages.stable) et winetricks"
        echo "    à environment.systemPackages, puis nixos-rebuild switch."
        echo ""
    fi
}

# ==============================================================================
# Programme principal
# ==============================================================================

main() {
    mkdir -p "$LOG_DIR"
    ensure_privilege_helper || true

    PRONOTE_URL_FROM_OPTION=""
    PRONOTE_ARCH_FROM_OPTION=""

    if [ ! -t 0 ]; then
        ASSUME_YES="1"
        SHOW_WINE_LOGS="0"
        log_info "Entrée non interactive détectée : mode silencieux activé."
    fi

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                usage
                exit 0
                ;;
            --yes)
                ASSUME_YES="1"
                shift
                ;;
            --arch)
                if [ -z "${2:-}" ]; then
                    log_error "--arch nécessite une valeur (32 ou 64)."
                    exit 1
                fi
                if [[ "$2" != "32" && "$2" != "64" ]]; then
                    log_error "Architecture invalide : $2 (attendu 32 ou 64)."
                    exit 1
                fi
                PRONOTE_ARCH_FROM_OPTION="$2"
                shift 2
                ;;
            --url)
                if [ -z "${2:-}" ]; then
                    log_error "--url nécessite une URL."
                    exit 1
                fi
                PRONOTE_URL_FROM_OPTION="$2"
                shift 2
                ;;
            --uninstall)
                UNINSTALL_ONLY="1"
                if [[ "${2:-}" == "32" || "${2:-}" == "64" ]]; then
                    UNINSTALL_ARCH="$2"
                    shift
                fi
                shift
                ;;
            --in-nix-shell)
                IN_NIX_SHELL="1"
                shift
                ;;
            *)
                log_error "Option inconnue : $1"
                usage
                exit 1
                ;;
        esac
    done

    if [ "$UNINSTALL_ONLY" = "1" ]; then
        local uninstaller=""

        if [ -n "$UNINSTALL_ARCH" ]; then
            uninstaller="$BIN_DIR/pronote-desinstaller-${UNINSTALL_ARCH}"
        else
            uninstaller="$BIN_DIR/pronote-desinstaller"
        fi

        if [ ! -x "$uninstaller" ]; then
            for candidate in \
                "$BIN_DIR/pronote-desinstaller-64" \
                "$BIN_DIR/pronote-desinstaller-32" \
                "$BIN_DIR/pronote-desinstaller-${DEFAULT_YEAR}" \
                "$BIN_DIR/pronote-desinstaller"; do
                if [ -x "$candidate" ]; then
                    uninstaller="$candidate"
                    break
                fi
            done
        fi

        if [ -x "$uninstaller" ]; then
            exec "$uninstaller"
        else
            log_error "Désinstallateur introuvable. Avez-vous déjà installé Pronote avec ce script ?"
            exit 1
        fi
    fi

    detect_distro

    if [ "$IN_NIX_SHELL" = "1" ]; then
        DISTRO_FAMILY="nixos-done"
        log_success "Environnement NixOS (ou dérivé) actif."
    fi

    log_info "Distribution détectée : $DISTRO_NAME ($DISTRO_FAMILY)"
    log_info "Installateur Pronote Linux v$SCRIPT_VERSION"

    set_target_windows_version

    ask_wine_output_mode
    ensure_dependencies

    check_latest_version

    if [ -n "$PRONOTE_URL_FROM_OPTION" ]; then
        process_custom_url
    else
        get_pronote_url
    fi

    set_wineprefix
    configure_wine
    install_pronote
    create_launchers

    log_info "Nettoyage des fichiers temporaires..."
    rm -rf "$TMP_DIR" 2>/dev/null || true

    echo ""
    echo -e "${GREEN}"
    box_title "Installation terminee avec succes" 62
    echo -e "${NC}"

    log_success "Version installée : Pronote $PRONOTE_VERSION - ${PRONOTE_ARCH} bits"
    echo ""
    echo "Pièces jointes (cahier de textes) :"
    echo "  • Images, PDF, vidéos... : ouvertes dans le navigateur par défaut"
    echo "  • Documents bureautiques (Word, Excel, PowerPoint, OpenDocument,"
    echo "    RTF, texte...) : ouverts dans la suite bureautique installée"
    echo "    (rien n'est ouvert si aucune suite n'est installée)"
    if [ -n "$HIDPI_APPLIED" ]; then
        echo ""
        echo "Écran haute résolution : caractères de Pronote agrandis à $HIDPI_APPLIED %"
        echo "  (modifiable : WINEPREFIX=\"$WINEPREFIX\" winecfg, onglet « Affichage »)"
    fi
    echo ""
    echo "Pour lancer Pronote :"
    echo "  • Menu Applications > Éducation > Pronote Client $PRONOTE_YEAR (${PRONOTE_ARCH} bits)"
    echo "  • Ou en terminal : pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}   (alias : pronote-$PRONOTE_YEAR)"
    echo "  • Ou double-clic sur le raccourci Bureau (si un dossier Bureau a été détecté)"
    echo ""
    echo "Pour désinstaller :"
    echo "  • Version 64 bits : ~/.local/bin/pronote-desinstaller-64"
    echo "  • Version 32 bits : ~/.local/bin/pronote-desinstaller-32"
    echo "  • Ou simplement   : ~/.local/bin/pronote-desinstaller (propose le choix si les deux sont installées)"
    echo ""
    echo "Astuce : les versions 32 et 64 bits peuvent être installées en même temps ;"
    echo "chacune a son propre préfixe Wine et son propre désinstallateur."
    echo ""
    echo "Emplacements :"
    echo "  • Préfixe Wine : $WINEPREFIX"
    echo "  • Lanceur : $BIN_DIR/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}"
    echo "  • Raccourci menu : $APP_DIR/pronote-${PRONOTE_YEAR}-${PRONOTE_ARCH}.desktop"
    echo "  • Menu XDG : $MENU_MERGE_DIR/pronote-${PRONOTE_YEAR}.menu"
    echo "  • Journal d'installation : $LOG_FILE"
    echo ""

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *)
            log_warning "$BIN_DIR n'est pas présent dans le PATH actuel."
            echo "Dans ce terminal, faites :"
            echo "  source ~/.bashrc && hash -r"
            ;;
    esac

    show_troubleshooting

    log_success "Bonne utilisation de Pronote !"
    echo ""
}

main "$@"
