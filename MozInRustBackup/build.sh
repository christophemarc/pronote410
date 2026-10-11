#!/bin/sh
# ==============================================================================
#  MozInRustBackup — compilation du programme
#
#  Ce script construit le binaire `mozinrustbackup`.
#  Il utilise `cargo` s'il est présent, sinon `rustc` seul (aucune dépendance
#  externe n'est nécessaire : le programme se compile hors ligne).
#
#  Usage :
#     ./build.sh              (binaire optimisé)
#     ./build.sh --debug      (binaire de mise au point)
#     ./build.sh --tests      (compile et lance la suite de tests)
# ==============================================================================
set -eu

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$DIR"

MODE=release
TESTS=0
for arg in "$@"; do
    case "$arg" in
        --debug) MODE=debug ;;
        --tests|--test) TESTS=1 ;;
        -h|--help)
            sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "Option inconnue : $arg" >&2; exit 2 ;;
    esac
done

if command -v cargo >/dev/null 2>&1; then
    echo "→ Compilation avec cargo ($(cargo --version))"
    if [ "$MODE" = "release" ]; then
        cargo build --release --locked 2>/dev/null || cargo build --release
    else
        cargo build
    fi
    if [ "$TESTS" = "1" ]; then
        echo "→ Lancement des tests"
        cargo test
    fi
    BIN="$DIR/target/$MODE/mozinrustbackup"
elif command -v rustc >/dev/null 2>&1; then
    echo "→ Compilation avec rustc ($(rustc --version))"
    VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
    echo "  (version $VERSION)"
    mkdir -p build
    rustc --edition 2021 -C opt-level=2 --crate-type lib --crate-name mozinrustbackup \
        src/lib.rs --out-dir build
    rustc --edition 2021 -C opt-level=2 --crate-name mozinrustbackup \
        --extern mozinrustbackup=build/libmozinrustbackup.rlib \
        -L build src/main.rs -o build/mozinrustbackup
    BIN="$DIR/build/mozinrustbackup"
    if [ "$TESTS" = "1" ]; then
        echo "  (les tests nécessitent cargo : installation recommandée)"
    fi
else
    echo "Erreur : ni cargo ni rustc n'est installé." >&2
    echo "Installez Rust : https://rustup.rs  (ou le paquet « rustc » de votre distribution)" >&2
    exit 1
fi

echo
echo "✓ Programme compilé : $BIN"
"$BIN" --version
echo
echo "Pour installer (menu, raccourci de bureau) : ./install.sh"
