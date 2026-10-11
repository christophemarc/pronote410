//! Tests d'interopérabilité avec Python (`zlib` / `zipfile`).
//!
//! Ils vérifient que :
//!  * nos archives `.mozinrustbackup` sont relues par un outil tiers (le
//!    gestionnaire d'archives de l'utilisateur, Python, `unzip`…) ;
//!  * nous savons relire, restaurer, une archive produite par un autre outil
//!    (blocs DEFLATE dynamiques, ZIP64, descripteurs de données).
//!
//! Les tests sont ignorés silencieusement si `python3` n'est pas installé.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use mozinrustbackup::archive::deflate::Level;
use mozinrustbackup::archive::zip::{WriteEntry, ZipReader, ZipWriter};

fn python_available() -> bool {
    Command::new("python3")
        .arg("-c")
        .arg("import zlib, zipfile")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("mrb-interop-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_python(script: &str, cwd: &Path) -> String {
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .current_dir(cwd)
        .output()
        .expect("python3");
    assert!(
        out.status.success(),
        "échec du script Python :\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn python_reads_our_archive() {
    if !python_available() {
        eprintln!("python3 absent : test ignoré");
        return;
    }
    let dir = temp_dir("lire-nous");
    let archive = dir.join("test.mozinrustbackup");

    // Charge utile : texte répétitif (compressible) et données pseudo-aléatoires.
    let text: Vec<u8> = b"Profil Firefox de test -- MozInRustBackup -- "
        .iter()
        .cloned()
        .cycle()
        .take(300_000)
        .collect();
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let random: Vec<u8> = (0..100_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state & 0xFF) as u8
        })
        .collect();

    let file = std::fs::File::create(&archive).unwrap();
    {
        let mut w = ZipWriter::new(file, 0, Level::Normal);
        let d = WriteEntry::dir("profil", 0o040755, 1_700_000_000, 1000, 1000);
        w.add_dir(&d).unwrap();
        let m = WriteEntry::file("profil/prefs.js", 0o100600, 1_700_000_100, 1000, 1000);
        w.add_file(&m, &mut &text[..], text.len() as u64, true, &mut |_| {}).unwrap();
        let m = WriteEntry::file("profil/places.sqlite", 0o100600, 1_700_000_200, 1000, 1000);
        w.add_file(&m, &mut &random[..], random.len() as u64, true, &mut |_| {}).unwrap();
        let l = WriteEntry::symlink("profil/current", 1_700_000_300, 1000, 1000);
        w.add_bytes(&l, b"prefs.js", false).unwrap();
        let e = WriteEntry::dir("profil/vide", 0o040700, 1_700_000_400, 1000, 1000);
        w.add_dir(&e).unwrap();
        w.finish().unwrap();
    }

    std::fs::write(dir.join("attendu-texte.bin"), &text).unwrap();
    std::fs::write(dir.join("attendu-aleatoire.bin"), &random).unwrap();

    let result = run_python(
        r#"
import zipfile, sys, hashlib, os
z = zipfile.ZipFile("test.mozinrustbackup")
bad = z.testzip()
assert bad is None, bad
names = sorted(z.namelist())
print("NOMS:", " ".join(names))
z.extractall("extrait")
t = open("attendu-texte.bin","rb").read()
r = open("attendu-aleatoire.bin","rb").read()
assert open("extrait/profil/prefs.js","rb").read() == t
assert open("extrait/profil/places.sqlite","rb").read() == r
info = z.getinfo("profil/prefs.js")
print("COMPRESSION:", info.compress_type, info.file_size, info.compress_size)
print("MODE:", oct(info.external_attr >> 16))
print("LIEN:", open("extrait/profil/current","rb").read().decode())
print("OK")
"#,
        &dir,
    );
    assert!(result.contains("OK"), "sortie Python inattendue : {}", result);
    assert!(result.contains("NOMS: profil/ profil/current profil/places.sqlite profil/prefs.js profil/vide/"), "{}", result);
    assert!(result.contains("COMPRESSION: 8 300000"), "{}", result);
    assert!(result.contains("LIEN: prefs.js"), "{}", result);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn we_read_python_archives() {
    if !python_available() {
        eprintln!("python3 absent : test ignoré");
        return;
    }
    let dir = temp_dir("lire-python");
    run_python(
        r#"
import zipfile, os
os.makedirs("src/sous-dossier", exist_ok=True)
texte = ("Thunderbird MozInRustBackup " * 20000).encode()
open("src/messages.txt","wb").write(texte)
open("src/sous-dossier/petit.txt","wb").write(b"bonjour")
# Niveau 9 : zlib utilise des blocs DEFLATE dynamiques.
with zipfile.ZipFile("python9.zip","w",zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    z.write("src/messages.txt","src/messages.txt")
    z.write("src/sous-dossier/petit.txt","src/sous-dossier/petit.txt")
# Sans compression (blocs stored).
with zipfile.ZipFile("pythonstore.zip","w",zipfile.ZIP_STORED) as z:
    z.write("src/messages.txt","src/messages.txt")
# ZIP64 forcé : beaucoup d'entrées.
with zipfile.ZipFile("python64.zip","w",zipfile.ZIP_DEFLATED,allowZip64=True) as z:
    for i in range(66000):
        z.writestr("f%05d.txt" % i, b"x")
print("ok")
"#,
        &dir,
    );

    for name in ["python9.zip", "pythonstore.zip", "python64.zip"] {
        let file = std::fs::File::open(dir.join(name)).unwrap();
        let mut rd = ZipReader::open(file).unwrap();
        let entries: Vec<String> = rd.entries().iter().map(|e| e.name.clone()).collect();
        if name == "python64.zip" {
            assert_eq!(entries.len(), 66_000, "nombre d'entrées ZIP64");
            let data = rd.read_entry_to_vec(66_000 - 1).unwrap();
            assert_eq!(data, b"x");
            continue;
        }
        let idx = entries.iter().position(|n| n == "src/messages.txt").unwrap();
        let data = rd.read_entry_to_vec(idx).unwrap();
        assert_eq!(data.len(), "Thunderbird MozInRustBackup ".len() * 20_000);
        assert!(data.starts_with(b"Thunderbird MozInRustBackup "));
        if name == "python9.zip" {
            let idx = entries.iter().position(|n| n == "src/sous-dossier/petit.txt").unwrap();
            assert_eq!(rd.read_entry_to_vec(idx).unwrap(), b"bonjour");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn our_deflate_stream_is_readable_by_zlib() {
    if !python_available() {
        eprintln!("python3 absent : test ignoré");
        return;
    }
    use mozinrustbackup::archive::deflate::{compress_stream, Deflater};
    let dir = temp_dir("zlib");
    let text: Vec<u8> = b"Le contenu d'un profil est surtout du texte compresse facilement. "
        .iter()
        .cloned()
        .cycle()
        .take(200_000)
        .collect();
    let mut compressed = Vec::new();
    let mut src: &[u8] = &text;
    compress_stream(&mut src, &mut compressed, Level::Normal).unwrap();
    std::fs::write(dir.join("compresse.bin"), &compressed).unwrap();
    std::fs::write(dir.join("attendu.bin"), &text).unwrap();

    let out = run_python(
        r#"
import zlib
d = open("compresse.bin","rb").read()
txt = zlib.decompress(d, -15)
ref = open("attendu.bin","rb").read()
assert txt == ref, (len(txt), len(ref))
print("OK", len(d), len(ref))
"#,
        &dir,
    );
    assert!(out.starts_with("OK"), "{}", out);
    // Le flux doit aussi être relisible par notre propre décompresseur.
    let mut back = Vec::new();
    mozinrustbackup::archive::inflate::inflate(&compressed[..], &mut back, None).unwrap();
    assert_eq!(back, text);

    // Et avec un flux vide (0 octet) comme avec un flux minuscule.
    for payload in [b"".to_vec(), b"a".to_vec(), b"ab".to_vec(), b"abc".to_vec()] {
        let mut c = Vec::new();
        let mut d = Deflater::new(Level::Max);
        d.write(&payload, &mut c).unwrap();
        d.finish(&mut c).unwrap();
        std::fs::write(dir.join("mini.bin"), &c).unwrap();
        std::fs::write(dir.join("mini-attendu.bin"), &payload).unwrap();
        let out = run_python(
            r#"
import zlib
d = open("mini.bin","rb").read()
ref = open("mini-attendu.bin","rb").read()
assert zlib.decompress(d, -15) == ref
print("OK")
"#,
            &dir,
        );
        assert!(out.contains("OK"));
    }

    // Cohérence de nos lecteurs avec un Cursor en mémoire.
    let mut sink = Cursor::new(Vec::new());
    mozinrustbackup::archive::inflate::inflate(&compressed[..], &mut sink, Some(text.len() as u64))
        .unwrap();
    assert_eq!(sink.into_inner(), text);
    let _ = std::fs::remove_dir_all(&dir);
}
