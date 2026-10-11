# MozInRustBackup

**Sauvegarde et restauration des profils Firefox et Thunderbird sous Linux** —
l'équivalent Linux de MozBackup, écrit en Rust, sans aucune dépendance externe.

L'objectif est simple : **un néophyte doit pouvoir sauvegarder son profil sur une
clé USB et le remettre en place sur un autre ordinateur, sans rien comprendre à
l'informatique et sans jamais risquer de perdre ses données.**

---

## Sommaire

1. [Ce que fait le programme](#1-ce-que-fait-le-programme)
2. [Installation](#2-installation)
3. [Utilisation : l'assistant pas à pas](#3-utilisation--lassistant-pas-à-pas)
4. [Utilisation en ligne de commande](#4-utilisation-en-ligne-de-commande)
5. [Le format de sauvegarde](#5-le-format-de-sauvegarde)
6. [Où les profils sont-ils cherchés ?](#6-où-les-profils-sont-ils-cherchés-)
7. [Sécurité des données](#7-sécurité-des-données)
8. [Dépannage](#8-dépannage)
9. [Détails techniques](#9-détails-techniques)
10. [Désinstallation](#10-désinstallation)

---

## 1. Ce que fait le programme

Au lancement, le logiciel demande d'abord ce que vous voulez faire :
**sauvegarder** un profil ou en **restaurer** un.

### Sauvegarde

1. Il **scanne le disque dur** à la recherche des profils Firefox et Thunderbird,
   qu'ils viennent de l'installation classique de la distribution, d'un paquet
   **Flatpak** ou d'un paquet **Snap**.
2. Il affiche clairement la liste des profils trouvés avec, pour chacun :
   le **type** (Firefox ou Thunderbird), l'**origine** (Flatpak, Snap, natif
   Debian, natif Arch, natif Mint, natif Fedora…), la **taille**, le **nombre de
   fichiers**, la **date de dernière modification** et le **chemin exact**.
3. Vous choisissez le profil à sauvegarder (ou « tous les profils »).
4. Vous choisissez **où** enregistrer la sauvegarde : clé USB proposée en
   priorité, dossier conseillé, dernier dossier utilisé ou n'importe quel autre
   dossier.
5. Le profil est **compressé en un seul fichier**, sans jamais être modifié :
   c'est une **copie conforme** (contenu, droits, dates, liens symboliques).
6. Un **journal** `.txt` est écrit à côté de l'archive : il indique en évidence
   l'emplacement exact du profil d'origine. C'est lui qui permet de restaurer le
   profil **au bon endroit**, des années plus tard.

Exemple de fichiers produits sur la clé USB :

```
Firefox_11102026_Flatpak.mozinrustbackup     ← la sauvegarde compressée
Firefox_11102026_Flatpak.txt                 ← le journal (à conserver !)
```

Le nom est composé ainsi : **application + date (JJMMAAAA) + origine**, ce qui
évite toute confusion entre, par exemple, le Firefox Flatpak et le Firefox natif
Arch d'un même ordinateur.

### Restauration

1. Vous choisissez le fichier de sauvegarde (`.mozinrustbackup`).
2. Le logiciel **vérifie que c'est bien une sauvegarde légitime** : signature
   interne, manifeste, version de format, empreintes des fichiers.
3. Il **indique précisément où le profil sera restauré**, d'après le journal.
   Si la sauvegarde vient d'un autre ordinateur (autre nom d'utilisateur), le
   chemin est automatiquement adapté à votre dossier personnel.
4. Si un profil identique existe déjà, **il demande confirmation** avant
   d'écraser quoi que ce soit — et propose de **conserver l'ancien profil** en le
   renommant `…-remplace-20261011-143005`, plutôt que de l'effacer.
5. La restauration s'effectue avec contrôle d'intégrité fichier par fichier.

### En plus

* **Vérification** d'une sauvegarde à tout moment (`--verifier` ou menu 3) :
  chaque fichier est relu et comparé à son empreinte SHA-256.
* **Recherche sur un autre disque** : retrouve les profils d'un ancien
  ordinateur, d'un disque externe ou d'une clé USB, même imbriqués profondément.
* **Fermeture assistée** de Firefox/Thunderbird si l'application est ouverte
  (un profil utilisé pendant la sauvegarde pourrait être incohérent).
* **Contrôle d'espace disque** avant d'écrire, avec proposition de changer de
  support.
* **Barre de progression** avec le fichier en cours et la quantité traitée.
* Les sauvegardes créées par une version du logiciel restent lisibles par toutes
  les versions suivantes : **le format ne change pas**.

---

## 2. Installation

### Méthode recommandée : le script d'installation

```bash
cd MozInRustBackup
./install.sh
```

Le script détecte votre distribution (Debian, Ubuntu, Mint, Fedora, Arch,
CachyOS, Manjaro, openSUSE, NixOS, Alpine, Void, Gentoo, Solus, Slackware…),
vérifie la présence du compilateur Rust, propose de l'installer si nécessaire,
compile le programme et installe :

* le binaire `mozinrustbackup` (`~/.local/bin`) ;
* une entrée dans le **menu de votre bureau** ;
* l'icône associée ;
* les **boîtes de dialogue natives** (`zenity`) si elles manquent.

Options :

```bash
./install.sh --system        # pour tous les utilisateurs (mot de passe admin)
./install.sh --no-dialogs    # ne propose pas d'installer zenity
```

### Compiler soi-même

```bash
./build.sh                   # utilise cargo s'il est là, sinon rustc
./build.sh --tests           # compile et lance la suite de tests
```

Ou directement :

```bash
cargo build --release        # → target/release/mozinrustbackup
```

**Aucune dépendance n'est téléchargée** : le programme n'utilise que la
bibliothèque standard de Rust. Il se compile donc hors ligne, dans un
environnement d'entreprise ou sur un ordinateur sans Internet.
Rust 1.70 ou plus récent est requis (le script d'installation s'en charge).

---

## 3. Utilisation : l'assistant pas à pas

Lancez `mozinrustbackup` (menu du bureau, ou commande dans un terminal).

```
┌─ MozInRustBackup ──────────────────────────────────────────────┐
│ Que voulez-vous faire ?                                        │
└────────────────────────────────────────────────────────────────┘
   1. Sauvegarder un profil Firefox ou Thunderbird
   2. Restaurer une sauvegarde
   3. Vérifier une sauvegarde
   4. Afficher les profils détectés sur ce disque
   5. Quitter
```

### Écran de sélection des profils

```
┌─ Profils détectés ────────────────────────────────────────────────────────┐
│ Déf. │ Type et origine        │ Profil           │ Taille │ Modifié le     │
├──────┼────────────────────────┼──────────────────┼────────┼────────────────┤
│ ★    │ Firefox natif Debian   │ default-release  │ 1,4 Go │ 05/10/2026 14h │
│      │ Firefox natif Debian   │ perso            │ 240 Mo │ 12/03/2026 09h │
│ ★    │ Firefox Flatpak        │ default          │ 780 Mo │ 01/10/2026 20h │
│      │ Firefox Snap           │ default          │ 95 Mo  │ 22/09/2026 18h │
│ ★    │ Thunderbird Flatpak    │ default          │ 3,1 Go │ 30/09/2026 11h │
│      │ Thunderbird natif Arch │ default          │ 3,0 Go │ 02/10/2026 08h │
└──────────────────────────────────────────────────────────────────────────┘
```

★ = profil utilisé par défaut au lancement de l'application.

### Si aucune boîte de dialogue graphique n'est disponible

Sur un serveur, en session SSH ou si `zenity`/`kdialog` n'est pas installé,
l'assistant bascule automatiquement en **mode texte** : mêmes écrans, mêmes
questions, mêmes choix (numérotés), Entrée valide la valeur proposée.
`Ctrl+C` interrompt proprement une opération en cours.

---

## 4. Utilisation en ligne de commande

```bash
mozinrustbackup --liste
mozinrustbackup --sauvegarder --profil 2 --dossier /media/cle-usb
mozinrustbackup --restaurer /media/cle-usb/Firefox_11102026_Flatpak.mozinrustbackup
mozinrustbackup --verifier /media/cle-usb/Firefox_11102026_Flatpak.mozinrustbackup
```

Options principales :

| Option | Effet |
|---|---|
| `--liste` | affiche les profils trouvés (type, origine, taille, date, chemin) |
| `--sauvegarder` | crée une sauvegarde sans passer par l'assistant |
| `--profil <n°\|chemin>` | profil à sauvegarder (numéro issu de `--liste`) |
| `--dossier <dossier>` | destination de la sauvegarde |
| `--sans-caches` | n'inclut pas les caches (sauvegarde plus légère) |
| `--oui` | ne pose aucune question (écrasement compris) |
| `--restaurer <fichier>` | restaure une sauvegarde |
| `--destination <dossier>` | restaure ailleurs que l'emplacement d'origine |
| `--mode <renommer\|supprimer\|refuser>` | comportement si un profil existe déjà (défaut : `refuser`) |
| `--verifier <fichier>` | contrôle l'intégrité de la sauvegarde |
| `--chercher <dossier>` | cherche les profils dans un autre disque |
| `--ui <zenity\|kdialog\|yad\|tui>` | force le type d'interface |

Codes de retour : `0` succès, `1` erreur, `2` mauvaise utilisation, `3` annulation.
`mozinrustbackup --aide` affiche le récapitulatif complet.

---

## 5. Le format de sauvegarde

Le fichier `.mozinrustbackup` est une **archive ZIP standard** contenant :

```
Firefox_11102026_Flatpak.mozinrustbackup
├── profil/                                ← le profil, tel quel
│   ├── prefs.js
│   ├── places.sqlite
│   ├── key4.db
│   ├── storage/…
│   └── …
├── mozinrustbackup.manifeste.txt          ← qui, quoi, où, quand
└── mozinrustbackup.empreintes.txt         ← SHA-256 de chaque fichier
```

**Pourquoi ZIP ?** Parce que c'est un format universel : si un jour vous n'avez
plus le logiciel sous la main, vous pouvez ouvrir la sauvegarde avec
`unzip`, 7-Zip, Ark ou l'outil graphique de votre bureau, et récupérer vos
fichiers à la main. Aucune dépendance à MozInRustBackup n'est nécessaire pour
accéder à vos données.

Le ZIP est écrit avec les extensions suivantes, ce qui garantit une copie
fidèle : droits Unix, dates précises au secondes (champ `0x5455`), utilisateur et
groupe (champ `0x7875`), liens symboliques, dossiers vides, et **ZIP64** pour les
profils dépassant 4 Go.

### Le journal `.txt`

Écrit à côté de l'archive (même nom de base), il est **lisible avec n'importe
quel éditeur de texte** :

```
# ─────────────────────────────────────────────────────────────────────
#  MozInRustBackup — journal de sauvegarde de profil Mozilla
#
#  Ce fichier est INDISPENSABLE pour restaurer la sauvegarde au bon
#  endroit. Conservez-le à côté de l'archive, sans le renommer.
# ─────────────────────────────────────────────────────────────────────
magic = MOZINRUSTBACKUP-SAUVERGARDE
format_version = 1
application = Firefox
origine = Flatpak
origine_detail = org.mozilla.firefox
profil_nom = default-release
profil_chemin = /home/alice/.var/app/org.mozilla.firefox/.mozilla/firefox/ab12cd34.default-release
profil_racine = /home/alice/.var/app/org.mozilla.firefox/.mozilla/firefox
utilisateur = alice
dossier_utilisateur = /home/alice
machine = portable-alice
date_sauvegarde = 2026-10-11 14:32:07
fichiers = 1284
taille_totale = 1523456789
archive_sha256 = e69ee4c956ddbec4…
```

Le même manifeste est aussi **embarqué dans l'archive** : la sauvegarde reste donc
exploitable même si le journal a été perdu, renommé ou copié ailleurs.

---

## 6. Où les profils sont-ils cherchés ?

| Type d'installation | Firefox | Thunderbird |
|---|---|---|
| native (paquet de la distribution) | `~/.mozilla/firefox` | `~/.thunderbird` |
| Flatpak | `~/.var/app/org.mozilla.firefox/.mozilla/firefox` | `~/.var/app/org.mozilla.Thunderbird/.thunderbird` |
| Snap | `~/snap/firefox/common/.mozilla/firefox` | `~/snap/thunderbird/common/.thunderbird` |

Les profils sont listés à partir de `profiles.ini` (ou de `profiles.json` pour
les anciennes versions de Snap/Flatpak). Les dossiers qui ressemblent à un profil
sans être déclarés (copie manuelle, profil restauré, ancien profil conservé)
sont également proposés, avec la mention correspondante.

La commande `--chercher /media/cle-usb` (ou l'option « Chercher sur un autre
disque » de l'assistant) analyse en profondeur un disque externe pour retrouver
les profils d'un ancien ordinateur, par exemple
`/media/cle-usb/home/bob/.mozilla/firefox/xyz.default`.

---

## 7. Sécurité des données

* **Le profil d'origine n'est jamais modifié.** Il est ouvert en lecture seule
  pendant toute l'opération. Aucun fichier du profil n'est renommé, déplacé ni
  supprimé, même en cas d'erreur.
* **Aucune archive incomplète** : la sauvegarde est écrite dans un fichier
  temporaire `…mozinrustbackup.part`, renommé seulement une fois terminée et
  vérifiée. Si l'opération est interrompue (annulation, coupure de courant,
  `Ctrl+C`), le fichier temporaire est supprimé.
* **Rien n'est effacé sans autorisation** : si un profil existe déjà à
  l'emplacement de restauration, le logiciel demande explicitement quoi faire, et
  l'option par défaut **conserve** l'ancien profil (renommé avec un suffixe
  `-remplace-<date>`).
* **Contrôle d'intégrité** systématique : CRC-32 sur chaque élément de l'archive
  et empreinte SHA-256 sur chaque fichier.
* **Fichiers de verrou exclus** : `parent.lock`, `.parentlock` et `lock` ne sont
  pas sauvegardés — restaurer un verrou empêcherait Firefox/Thunderbird de
  démarrer (« profil déjà utilisé »). Ce détail est signalé dans le journal.
* **Sauvegarde pendant que l'application est ouverte** : le programme prévient et
  propose de fermer proprement l'application (elle enregistre alors ses fichiers
  avant de quitter) ; il attend la fermeture effective.
* **Fichiers temporaires** : aucune copie intermédiaire n'est laissée dans le
  profil ni dans `/tmp` (la compression est faite en flux).

---

## 8. Dépannage

**« Aucun profil Firefox ou Thunderbird n'a été trouvé »**
Lancez au moins une fois l'application : un profil n'est créé qu'au premier
démarrage. Si le profil se trouve sur un autre disque, utilisez
« Chercher sur un autre disque » ou `--chercher /media/…`.

**Les fenêtres graphiques ne s'affichent pas, le mode texte apparaît**
Installez `zenity` (GNOME, Cinnamon, XFCE, MATE) ou `kdialog` (KDE) :
le script d'installation le propose, ou `sudo apt install zenity`.
Le mode texte reste parfaitement fonctionnel.

**« Le dossier n'est pas accessible en écriture »**
La clé USB est peut-être montée en lecture seule (protection physique, système
de fichiers abîmé) : essayez un autre port, un autre dossier, ou vérifiez avec
`mount | grep media`.

**« Espace insuffisant »**
La sauvegarde occupe en moyenne 30 % à 60 % de la taille du profil. La
sauvegarde « allégée » (sans caches) est souvent 3 à 5 fois plus petite.

**« Sauvegarde endommagée »**
L'archive a été altérée (mauvais support, copie incomplète, secteur défectueux).
Essayez une autre copie de sauvegarde ; l'originale reste intacte.

**« Ce fichier n'est pas une sauvegarde MozInRustBackup »**
Le fichier choisi n'est pas une archive du logiciel. Sélectionnez le fichier
`.mozinrustbackup` créé lors de la sauvegarde (et non le journal `.txt`).

**Firefox refuse de démarrer après restauration**
Fermez Firefox, vérifiez qu'aucun fichier de verrou ne traîne
(`ls ~/.mozilla/firefox/<profil>/parent.lock`) et supprimez-le le cas échéant.
Le journal indique le chemin exact du profil restauré.

---

## 9. Détails techniques

* **Langage** : Rust (édition 2021), **zéro dépendance externe** — le programme
  n'utilise que la bibliothèque standard.
* **Taille du binaire** : environ 1 Mo, sans éditeur de liens dynamique
  supplémentaire.
* **Implémentations écrites à la main** (donc entièrement auditables, testées et
  sans surprise de licence) : compresseur et décompresseur DEFLATE (RFC 1951),
  conteneur ZIP avec ZIP64, CRC-32 (IEEE) et SHA-256 (FIPS 180-4).
* **Interopérabilité vérifiée** par des tests automatisés qui comparent le
  résultat avec `zlib` et `zipfile` de Python, dans les deux sens (nos archives
  sont relues par Python ; nous relisons des archives créées par Python,
  y compris en DEFLATE dynamique et en ZIP64 avec 66 000 entrées).
* **Tests** : 48 tests couvrant l'archive, le format, la détection des profils,
  l'assistant complet (parcours scripté) et un cycle sauvegarde → restauration
  avec comparaison octet à octet (contenu, droits, dates, liens symboliques).

```bash
cargo test            # suite complète
./build.sh --tests    # compilation + tests
```

Structure du projet :

```
MozInRustBackup/
├── Cargo.toml
├── build.sh                  compilation (cargo ou rustc seul)
├── install.sh                installation multi-distributions
├── uninstall.sh              désinstallation
├── packaging/                raccourci de bureau + icône
├── src/
│   ├── main.rs               point d'entrée
│   ├── cli.rs                ligne de commande
│   ├── archive/              ZIP, DEFLATE, CRC-32, SHA-256
│   ├── core/                 détection des profils, sauvegarde, restauration,
│   │                         manifeste, lecture de profiles.ini / profiles.json
│   ├── sys/                  distribution, processus, disque, fichiers
│   └── ui/                   assistant (zenity, kdialog, mode texte)
└── tests/                    tests d'intégration et d'interopérabilité
```

---

## 10. Désinstallation

```bash
./uninstall.sh              # installation personnelle
./uninstall.sh --system     # installation système
```

Le programme, le raccourci et l'icône sont retirés. **Vos sauvegardes
(`.mozinrustbackup`), vos journaux (`.txt`) et vos profils ne sont jamais
touchés.**

---

## Licence

MIT. Ce logiciel n'est ni édité ni supporté par la Mozilla Foundation ; les noms
Firefox et Thunderbird sont utilisés uniquement pour désigner les logiciels dont
il manipule les profils.
