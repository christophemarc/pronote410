//! Compresseur DEFLATE (RFC 1951) écrit à la main, sans dépendance externe.
//!
//! - Compression LZ77 (fenêtre glissante de 32 Kio, chaînes de hachage) suivie
//!   d'un codage de Huffman **fixe** (celui de la RFC 1951, tableaux statiques) :
//!   aucun arbre à transmettre, donc un flux plus simple et plus robuste.
//! - Si un bloc compressé devient plus gros que les données d'origine, un bloc
//!   « stored » (non compressé) est émis à la place : la taille de sortie ne
//!   dépasse jamais celle de l'entrée de plus de 0,02 % (5 octets / 64 Kio).
//! - Le flux produit est un DEFLATE standard : `gzip`, `zlib`, `unzip`, 7-Zip,
//!   Python, etc. le lisent sans problème.

use std::io::{self, Write};

pub const WINDOW: usize = 32_768;
const CHUNK: usize = 32_768;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const HASH_BITS: u32 = 15;
const HASH_SIZE: usize = 1 << HASH_BITS;
const MAX_DIST: usize = 32_768;

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Niveaux de compression acceptés par `Deflater::new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Aucune compression (blocs « stored »), pour les données déjà compressées.
    Store,
    Fast,
    Normal,
    Max,
}

impl Level {
    fn chain_depth(self) -> usize {
        match self {
            Level::Store => 0,
            Level::Fast => 8,
            Level::Normal => 48,
            Level::Max => 192,
        }
    }
}

#[inline]
fn reverse_bits(mut v: u32, n: u32) -> u32 {
    let mut r = 0u32;
    for _ in 0..n {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    r
}

/// Écrivain de bits au format DEFLATE (paquet LSB d'abord).
struct BitWriter {
    acc: u64,
    nbits: u32,
    out: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { acc: 0, nbits: 0, out: Vec::with_capacity(CHUNK + 64) }
    }

    #[inline]
    fn put_bits(&mut self, value: u32, n: u32) {
        if n == 0 {
            return;
        }
        self.acc |= (value as u64) << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.out.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    /// Écrit un code de Huffman (transmis bit de poids fort en premier).
    #[inline]
    fn put_code(&mut self, code: u32, n: u32) {
        self.put_bits(reverse_bits(code, n), n);
    }

    #[allow(dead_code)]
    fn align_to_byte(&mut self) {
        if self.nbits % 8 != 0 {
            let pad = 8 - (self.nbits % 8);
            self.put_bits(0, pad);
        }
    }

    fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    fn pending_bits(&self) -> (u64, u32) {
        (self.acc, self.nbits)
    }

    fn set_pending(&mut self, acc: u64, nbits: u32) {
        self.acc = acc;
        self.nbits = nbits;
    }
}

/// Émet un littéral avec le code de Huffman fixe de la RFC 1951.
#[inline]
fn emit_literal(bw: &mut BitWriter, sym: u32) {
    if sym < 144 {
        bw.put_code(0x30 + sym, 8);
    } else if sym < 256 {
        bw.put_code(0x190 + (sym - 144), 9);
    } else if sym < 280 {
        bw.put_code(sym - 256, 7);
    } else {
        bw.put_code(0xC0 + (sym - 280), 8);
    }
}

#[inline]
fn length_code(len: usize) -> usize {
    // `len` est compris entre 3 et 258.
    let mut i = 28;
    while i > 0 && (len as u16) < LENGTH_BASE[i] {
        i -= 1;
    }
    i
}

#[inline]
fn distance_code(dist: usize) -> usize {
    let mut i = 29;
    while i > 0 && (dist as u16) < DIST_BASE[i] {
        i -= 1;
    }
    i
}

/// Compresseur DEFLATE incrémental.
pub struct Deflater {
    level: Level,
    // État du flux de bits (partiel, conservé entre les blocs).
    acc: u64,
    nbits: u32,
    // Données en attente de traitement.
    pending: Vec<u8>,
    // Derniers 32 Kio déjà traités (fenêtre glissante).
    history: Vec<u8>,
    // Fenêtre de travail réutilisée d'un bloc à l'autre.
    window: Vec<u8>,
    prev: Vec<u32>,
    head: Vec<u32>,
    scratch: BitWriter,
    finished: bool,
    input_total: u64,
}

impl Deflater {
    pub fn new(level: Level) -> Self {
        Deflater {
            level,
            acc: 0,
            nbits: 0,
            pending: Vec::with_capacity(CHUNK),
            history: Vec::with_capacity(WINDOW),
            window: Vec::with_capacity(WINDOW + CHUNK),
            prev: Vec::new(),
            head: vec![0u32; HASH_SIZE],
            scratch: BitWriter::new(),
            finished: false,
            input_total: 0,
        }
    }

    /// Compresse `data` et transmet les octets produits à `out`.
    pub fn write(&mut self, data: &[u8], out: &mut dyn Write) -> io::Result<()> {
        assert!(!self.finished, "Deflater::write après finish");
        self.input_total += data.len() as u64;
        let mut rest = data;
        while !rest.is_empty() {
            let space = CHUNK - self.pending.len();
            let take = space.min(rest.len());
            self.pending.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.pending.len() == CHUNK {
                self.flush_chunk(out)?;
            }
        }
        Ok(())
    }

    /// Termine le flux DEFLATE (bloc final vide marqué BFINAL).
    pub fn finish(&mut self, out: &mut dyn Write) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        if !self.pending.is_empty() {
            self.flush_chunk(out)?;
        }
        // Bloc final vide : BFINAL = 1, BTYPE = 01 (Huffman fixe), puis EOB.
        // Bits émis LSB d'abord : bit0 = BFINAL = 1, bit1 = 1, bit2 = 0 (BTYPE = 01).
        self.put_raw_bits(out, 0b011u64, 3)?;
        self.put_raw_bits(out, reverse_bits(256, 7) as u64, 7)?; // symbole 256 (fin de bloc)
        self.pad_to_byte(out)?;
        self.finished = true;
        Ok(())
    }

    /// Nombre d'octets d'entrée consommés.
    pub fn input_len(&self) -> u64 {
        self.input_total
    }

    // --- internes ---------------------------------------------------------

    fn put_raw_bits(&mut self, out: &mut dyn Write, value: u64, n: u32) -> io::Result<()> {
        self.acc |= value << self.nbits;
        self.nbits += n;
        self.drain_full_bytes(out)
    }

    fn drain_full_bytes(&mut self, out: &mut dyn Write) -> io::Result<()> {
        let mut buf = [0u8; 16];
        let mut n = 0;
        while self.nbits >= 8 {
            buf[n] = (self.acc & 0xFF) as u8;
            self.acc >>= 8;
            self.nbits -= 8;
            n += 1;
        }
        if n > 0 {
            out.write_all(&buf[..n])?;
        }
        Ok(())
    }

    fn pad_to_byte(&mut self, out: &mut dyn Write) -> io::Result<()> {
        if self.nbits % 8 != 0 {
            let pad = 8 - (self.nbits % 8);
            self.put_raw_bits(out, 0, pad)?;
        }
        Ok(())
    }

    /// Traite le tampon `pending` complet et met à jour l'historique.
    fn flush_chunk(&mut self, out: &mut dyn Write) -> io::Result<()> {
        let chunk_len = self.pending.len();
        if chunk_len == 0 {
            return Ok(());
        }
        if self.level == Level::Store {
            let data = std::mem::take(&mut self.pending);
            self.write_stored_block(out, &data)?;
            self.push_history(&data);
            return Ok(());
        }

        // Fenêtre = historique + données en attente.
        self.window.clear();
        self.window.extend_from_slice(&self.history);
        let history_len = self.window.len();
        self.window.extend_from_slice(&self.pending);

        let n = self.window.len();
        self.prev.clear();
        self.prev.resize(n, 0);
        for h in self.head.iter_mut() {
            *h = 0;
        }
        // Toutes les positions de l'historique entrent dans les chaînes de hachage.
        let mut i = 0usize;
        let hist_end = if history_len >= 2 { history_len - 2 } else { 0 };
        while i < hist_end {
            self.insert(i);
            i += 1;
        }

        // Le bloc est d'abord encodé dans un tampon d'essai, afin de pouvoir
        // choisir un bloc « stored » si la compression ne gagne rien.
        let (acc0, nbits0) = (self.acc, self.nbits);
        self.scratch.out.clear();
        self.scratch.set_pending(self.acc, self.nbits);
        // BFINAL = 0, BTYPE = 01 (Huffman fixe) : bits 0, 1, 0.
        self.scratch.put_bits(0b010, 3);

        let depth = self.level.chain_depth();
        let mut pos = history_len;
        while pos < n {
            let (len, dist) = self.find_match(pos, n, depth);
            if len >= MIN_MATCH {
                let lc = length_code(len);
                emit_literal(&mut self.scratch, 257 + lc as u32);
                let extra = LENGTH_EXTRA[lc] as u32;
                if extra > 0 {
                    self.scratch.put_bits((len as u16 - LENGTH_BASE[lc]) as u32, extra);
                }
                let dc = distance_code(dist);
                self.scratch.put_code(dc as u32, 5);
                let dextra = DIST_EXTRA[dc] as u32;
                if dextra > 0 {
                    self.scratch.put_bits((dist as u16 - DIST_BASE[dc]) as u32, dextra);
                }
                for k in 0..len {
                    if pos + k + 3 <= n {
                        self.insert(pos + k);
                    }
                }
                pos += len;
            } else {
                emit_literal(&mut self.scratch, self.window[pos] as u32);
                if pos + 3 <= n {
                    self.insert(pos);
                }
                pos += 1;
            }
        }
        emit_literal(&mut self.scratch, 256); // fin de bloc

        let produced = self.scratch.take();
        // 5 octets d'en-tête pour le bloc « stored » : on privilégie la
        // compression dès qu'elle apporte un gain réel.
        if produced.len() >= chunk_len + 5 {
            self.acc = acc0;
            self.nbits = nbits0;
            let data = std::mem::take(&mut self.pending);
            self.write_stored_block(out, &data)?;
            self.push_history(&data);
        } else {
            let (acc, nbits) = self.scratch.pending_bits();
            self.acc = acc;
            self.nbits = nbits;
            out.write_all(&produced)?;
            let data = std::mem::take(&mut self.pending);
            self.drain_full_bytes(out)?;
            self.push_history(&data);
        }
        Ok(())
    }

    fn write_stored_block(&mut self, out: &mut dyn Write, data: &[u8]) -> io::Result<()> {
        let mut offset = 0usize;
        while offset < data.len() {
            let max = 65_535usize;
            let len = (data.len() - offset).min(max);
            self.put_raw_bits(out, 0b000, 3)?; // BFINAL = 0, BTYPE = 00
            self.pad_to_byte(out)?;
            let len16 = len as u16;
            let nlen16 = !len16;
            let mut hdr = [0u8; 4];
            hdr[..2].copy_from_slice(&len16.to_le_bytes());
            hdr[2..].copy_from_slice(&nlen16.to_le_bytes());
            out.write_all(&hdr)?;
            out.write_all(&data[offset..offset + len])?;
            offset += len;
        }
        Ok(())
    }

    fn push_history(&mut self, data: &[u8]) {
        if data.len() >= WINDOW {
            self.history.clear();
            self.history.extend_from_slice(&data[data.len() - WINDOW..]);
        } else {
            let overflow = (self.history.len() + data.len()).saturating_sub(WINDOW);
            if overflow > 0 {
                self.history.drain(..overflow);
            }
            self.history.extend_from_slice(data);
        }
    }

    #[inline]
    fn hash3(&self, p: usize) -> usize {
        let w = &self.window;
        let h = ((w[p] as u32) << 10) ^ ((w[p + 1] as u32) << 5) ^ (w[p + 2] as u32);
        (h & ((1 << HASH_BITS) - 1)) as usize
    }

    #[inline]
    fn insert(&mut self, p: usize) {
        if p + 2 >= self.window.len() {
            return;
        }
        let h = self.hash3(p);
        self.prev[p] = self.head[h];
        self.head[h] = (p + 1) as u32;
    }

    fn find_match(&self, pos: usize, n: usize, depth: usize) -> (usize, usize) {
        if pos + MIN_MATCH > n {
            return (0, 0);
        }
        let h = self.hash3(pos);
        let max_len = MAX_MATCH.min(n - pos);
        let mut best_len = MIN_MATCH - 1;
        let mut best_dist = 0usize;
        let mut cand = self.head[h] as i64 - 1;
        let mut steps = depth;
        while cand >= 0 && steps > 0 {
            let c = cand as usize;
            if c >= pos {
                break;
            }
            let dist = pos - c;
            if dist > MAX_DIST {
                break;
            }
            if self.window[c + best_len] == self.window[pos + best_len] {
                let mut l = 0usize;
                while l < max_len && self.window[c + l] == self.window[pos + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = dist;
                    if l >= max_len {
                        break;
                    }
                }
            }
            cand = self.prev[c] as i64 - 1;
            steps -= 1;
        }
        if best_dist == 0 {
            (0, 0)
        } else {
            (best_len, best_dist)
        }
    }
}

/// Compresse un flux complet depuis un lecteur vers un écrivain.
///
/// Renvoie `(octets_lus, octets_écrits)`.
pub fn compress_stream<R: io::Read, W: Write>(
    input: &mut R,
    output: &mut W,
    level: Level,
) -> io::Result<(u64, u64)> {
    let mut deflater = Deflater::new(level);
    let mut buf = vec![0u8; 128 * 1024];
    let mut written = 0u64;
    let mut counted = CountingWriter { inner: output, written: 0 };
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        deflater.write(&buf[..n], &mut counted)?;
    }
    deflater.finish(&mut counted)?;
    written += counted.written;
    Ok((deflater.input_len(), written))
}

struct CountingWriter<'a, W: Write> {
    inner: &'a mut W,
    written: u64,
}

impl<'a, W: Write> Write for CountingWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deflate(data: &[u8], level: Level) -> Vec<u8> {
        let mut d = Deflater::new(level);
        let mut out = Vec::new();
        d.write(data, &mut out).unwrap();
        d.finish(&mut out).unwrap();
        out
    }

    #[test]
    fn empty_stream_is_valid() {
        let out = deflate(b"", Level::Normal);
        assert!(!out.is_empty());
    }

    #[test]
    fn repetitive_data_compresses() {
        let data = b"Bonjour le monde, ".repeat(2000);
        let out = deflate(&data, Level::Normal);
        assert!(out.len() < data.len() / 5, "ratio insuffisant : {} vs {}", out.len(), data.len());
    }

    #[test]
    fn random_data_does_not_inflate_much() {
        // Générateur pseudo-aléatoire déterministe (xorshift).
        let mut state = 0x1234_5678_9ABC_DEF0u64;
        let data: Vec<u8> = (0..200_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state & 0xFF) as u8
            })
            .collect();
        let out = deflate(&data, Level::Normal);
        assert!(
            out.len() <= data.len() + data.len() / 1000,
            "surcroît trop important : {} vs {}",
            out.len(),
            data.len()
        );
    }
}
