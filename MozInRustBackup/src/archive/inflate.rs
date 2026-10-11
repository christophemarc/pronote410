//! Décompresseur DEFLATE (RFC 1951) complet : blocs *stored*, *fixed* et
//! *dynamic*. Écrit à la main (algorithme de type « puff »), sans dépendance.
//!
//! Il lit aussi les archives produites par d'autres outils (7-Zip, Info-ZIP,
//! Python, etc.), ce qui garantit qu'une sauvegarde MozInRustBackup reste
//! exploitable même si l'utilisateur l'a reconditionnée avec un autre logiciel.

use std::io::{self, Read, Write};

use super::crc32::Crc32;

#[derive(Debug)]
pub enum InflateError {
    Io(io::Error),
    /// Données compressées invalides.
    Corrupt(&'static str),
    /// Fin de flux prématurée.
    Truncated,
    /// La taille annoncée ne correspond pas aux données décompressées.
    SizeMismatch { expected: u64, got: u64 },
}

impl std::fmt::Display for InflateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InflateError::Io(e) => write!(f, "erreur d'entrée/sortie : {}", e),
            InflateError::Corrupt(m) => write!(f, "flux DEFLATE corrompu ({})", m),
            InflateError::Truncated => write!(f, "flux DEFLATE tronqué"),
            InflateError::SizeMismatch { expected, got } => write!(
                f,
                "taille incohérente : {} octets attendus, {} obtenus",
                expected, got
            ),
        }
    }
}

impl std::error::Error for InflateError {}

impl From<io::Error> for InflateError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            InflateError::Truncated
        } else {
            InflateError::Io(e)
        }
    }
}

pub type Result<T> = std::result::Result<T, InflateError>;

// --- lecteur de bits ------------------------------------------------------

struct BitReader<R: Read> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    len: usize,
    bitbuf: u64,
    bitcnt: u32,
    eof: bool,
}

impl<R: Read> BitReader<R> {
    fn new(inner: R) -> Self {
        BitReader {
            inner,
            buf: vec![0u8; 64 * 1024].into_boxed_slice(),
            pos: 0,
            len: 0,
            bitbuf: 0,
            bitcnt: 0,
            eof: false,
        }
    }

    fn fill(&mut self) -> Result<()> {
        while self.bitcnt <= 56 {
            if self.pos == self.len {
                if self.eof {
                    break;
                }
                self.len = self.inner.read(&mut self.buf)?;
                self.pos = 0;
                if self.len == 0 {
                    self.eof = true;
                    break;
                }
            }
            self.bitbuf |= (self.buf[self.pos] as u64) << self.bitcnt;
            self.bitcnt += 8;
            self.pos += 1;
        }
        Ok(())
    }

    fn get_bit(&mut self) -> Result<u32> {
        if self.bitcnt == 0 {
            self.fill()?;
            if self.bitcnt == 0 {
                return Err(InflateError::Truncated);
            }
        }
        let b = (self.bitbuf & 1) as u32;
        self.bitbuf >>= 1;
        self.bitcnt -= 1;
        Ok(b)
    }

    fn get_bits(&mut self, n: u32) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        self.fill()?;
        if self.bitcnt < n {
            return Err(InflateError::Truncated);
        }
        let v = (self.bitbuf & ((1u64 << n) - 1)) as u32;
        self.bitbuf >>= n;
        self.bitcnt -= n;
        Ok(v)
    }

    /// Aligne sur l'octet suivant et renvoie les bits ignorés.
    fn align_to_byte(&mut self) {
        let drop = self.bitcnt % 8;
        self.bitbuf >>= drop;
        self.bitcnt -= drop;
    }

    fn read_bytes(&mut self, out: &mut [u8]) -> Result<()> {
        // Les bits non consommés (alignés) doivent être relus en premier.
        self.align_to_byte();
        let mut written = 0usize;
        while written < out.len() && self.bitcnt >= 8 {
            out[written] = (self.bitbuf & 0xFF) as u8;
            self.bitbuf >>= 8;
            self.bitcnt -= 8;
            written += 1;
        }
        while written < out.len() {
            if self.pos == self.len {
                if self.eof {
                    return Err(InflateError::Truncated);
                }
                self.len = self.inner.read(&mut self.buf)?;
                self.pos = 0;
                if self.len == 0 {
                    self.eof = true;
                    return Err(InflateError::Truncated);
                }
            }
            out[written] = self.buf[self.pos];
            self.pos += 1;
            written += 1;
        }
        Ok(())
    }
}

// --- arbres de Huffman ----------------------------------------------------

struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huffman {
    /// Construit un arbre canonique à partir des longueurs de code.
    fn new(lengths: &[u8]) -> Result<Huffman> {
        let mut count = [0u16; 16];
        for &l in lengths {
            if l as usize > 15 {
                return Err(InflateError::Corrupt("longueur de code > 15"));
            }
            count[l as usize] += 1;
        }
        if count[0] as usize == lengths.len() {
            // Aucun code : arbre vide, jamais utilisé (cas légal pour les distances).
            return Ok(Huffman { count, symbol: Vec::new() });
        }
        count[0] = 0;
        // Vérifie la validité (codes incomplets tolérés, sur/sous-subscription refusée).
        let mut left: i32 = 1;
        for len in 1..16 {
            left <<= 1;
            left -= count[len] as i32;
            if left < 0 {
                return Err(InflateError::Corrupt("arbre de Huffman sur-souscrit"));
            }
        }
        let mut offs = [0u16; 16];
        for len in 1..15 {
            offs[len + 1] = offs[len] + count[len];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huffman { count, symbol })
    }

    fn decode<R: Read>(&self, br: &mut BitReader<R>) -> Result<u16> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: usize = 0;
        for len in 1..=15usize {
            code |= br.get_bit()? as i32;
            let count = self.count[len] as i32;
            if code - first < count {
                return Ok(self.symbol[index + (code - first) as usize]);
            }
            index += count as usize;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(InflateError::Corrupt("code de Huffman invalide"))
    }
}

// --- tables de longueurs / distances --------------------------------------

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

const CL_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn fixed_trees() -> (Huffman, Huffman) {
    let mut lit_lengths = [0u8; 288];
    for (i, l) in lit_lengths.iter_mut().enumerate() {
        *l = if i < 144 {
            8
        } else if i < 256 {
            9
        } else if i < 280 {
            7
        } else {
            8
        };
    }
    let dist_lengths = [5u8; 30];
    (
        Huffman::new(&lit_lengths).expect("arbre fixe littéral"),
        Huffman::new(&dist_lengths).expect("arbre fixe distance"),
    )
}

/// Résultat d'une décompression : nombre d'octets produits et CRC-32 des données.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InflateOutcome {
    pub bytes: u64,
    pub crc32: u32,
}

/// Décompresse un flux DEFLATE complet de `input` vers `output`.
///
/// Si `expected_size` est fourni, une taille différente provoque une erreur.
pub fn inflate<R: Read, W: Write>(
    input: R,
    output: &mut W,
    expected_size: Option<u64>,
) -> Result<InflateOutcome> {
    let mut br = BitReader::new(input);
    let mut window = vec![0u8; 32 * 1024];
    let mut wpos: usize = 0;
    let mut total: u64 = 0;
    let mut crc = Crc32::new();
    let mut outbuf: Vec<u8> = Vec::with_capacity(64 * 1024);

    macro_rules! push {
        ($byte:expr) => {{
            let b: u8 = $byte;
            window[wpos & 0x7FFF] = b;
            wpos = wpos.wrapping_add(1);
            outbuf.push(b);
            if outbuf.len() >= 64 * 1024 {
                crc.update(&outbuf);
                output.write_all(&outbuf)?;
                total += outbuf.len() as u64;
                outbuf.clear();
            }
        }};
    }

    let (fixed_lit, fixed_dist) = fixed_trees();

    loop {
        let bfinal = br.get_bits(1)?;
        let btype = br.get_bits(2)?;
        match btype {
            0 => {
                // Bloc non compressé.
                br.align_to_byte();
                let mut hdr = [0u8; 4];
                br.read_bytes(&mut hdr)?;
                let len = u16::from_le_bytes([hdr[0], hdr[1]]);
                let nlen = u16::from_le_bytes([hdr[2], hdr[3]]);
                if len != !nlen {
                    return Err(InflateError::Corrupt("en-tête de bloc stored incohérent"));
                }
                let mut remaining = len as usize;
                let mut chunk = [0u8; 4096];
                while remaining > 0 {
                    let take = remaining.min(chunk.len());
                    br.read_bytes(&mut chunk[..take])?;
                    for &byte in &chunk[..take] {
                        push!(byte);
                    }
                    remaining -= take;
                }
            }
            1 | 2 => {
                let (lit, dist) = if btype == 1 {
                    (None, None)
                } else {
                    let (l, d) = read_dynamic_tables(&mut br)?;
                    (Some(l), Some(d))
                };
                let lit_tree = lit.as_ref().unwrap_or(&fixed_lit);
                let dist_tree = dist.as_ref().unwrap_or(&fixed_dist);
                loop {
                    let sym = lit_tree.decode(&mut br)?;
                    match sym {
                        0..=255 => push!(sym as u8),
                        256 => break,
                        257..=285 => {
                            let idx = sym as usize - 257;
                            let len = LENGTH_BASE[idx] as usize
                                + br.get_bits(LENGTH_EXTRA[idx] as u32)? as usize;
                            let dsym = dist_tree.decode(&mut br)? as usize;
                            if dsym >= 30 {
                                return Err(InflateError::Corrupt("code de distance invalide"));
                            }
                            let d = DIST_BASE[dsym] as usize
                                + br.get_bits(DIST_EXTRA[dsym] as u32)? as usize;
                            if d > 32_768 || d == 0 {
                                return Err(InflateError::Corrupt("distance hors fenêtre"));
                            }
                            if d > total as usize + outbuf.len() {
                                return Err(InflateError::Corrupt("référence avant le début"));
                            }
                            for _ in 0..len {
                                let b = window[(wpos.wrapping_sub(d)) & 0x7FFF];
                                push!(b);
                            }
                        }
                        _ => return Err(InflateError::Corrupt("symbole littéral invalide")),
                    }
                }
            }
            _ => return Err(InflateError::Corrupt("type de bloc inconnu (3)")),
        }
        if bfinal == 1 {
            break;
        }
    }

    if !outbuf.is_empty() {
        crc.update(&outbuf);
        output.write_all(&outbuf)?;
        total += outbuf.len() as u64;
    }
    output.flush()?;

    if let Some(exp) = expected_size {
        if exp != total {
            return Err(InflateError::SizeMismatch { expected: exp, got: total });
        }
    }

    Ok(InflateOutcome { bytes: total, crc32: crc.value() })
}

fn read_dynamic_tables<R: Read>(br: &mut BitReader<R>) -> Result<(Huffman, Huffman)> {
    let hlit = br.get_bits(5)? as usize + 257;
    let hdist = br.get_bits(5)? as usize + 1;
    let hclen = br.get_bits(4)? as usize + 4;
    if hlit > 286 || hdist > 30 {
        return Err(InflateError::Corrupt("HLIT/HDIST hors limites"));
    }
    let mut cl_lengths = [0u8; 19];
    for i in 0..hclen {
        cl_lengths[CL_ORDER[i]] = br.get_bits(3)? as u8;
    }
    let cl_tree = Huffman::new(&cl_lengths)?;

    let mut lengths = vec![0u8; hlit + hdist];
    let mut i = 0usize;
    while i < lengths.len() {
        let sym = cl_tree.decode(br)?;
        match sym {
            0..=15 => {
                lengths[i] = sym as u8;
                i += 1;
            }
            16 => {
                if i == 0 {
                    return Err(InflateError::Corrupt("répétition sans valeur précédente"));
                }
                let prev = lengths[i - 1];
                let rep = 3 + br.get_bits(2)? as usize;
                for _ in 0..rep {
                    if i >= lengths.len() {
                        return Err(InflateError::Corrupt("répétition trop longue"));
                    }
                    lengths[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let rep = 3 + br.get_bits(3)? as usize;
                i += rep;
                if i > lengths.len() {
                    return Err(InflateError::Corrupt("répétition de zéros trop longue"));
                }
            }
            18 => {
                let rep = 11 + br.get_bits(7)? as usize;
                i += rep;
                if i > lengths.len() {
                    return Err(InflateError::Corrupt("répétition de zéros trop longue"));
                }
            }
            _ => return Err(InflateError::Corrupt("symbole de longueur invalide")),
        }
    }
    if lengths[256] == 0 {
        return Err(InflateError::Corrupt("code de fin de bloc manquant"));
    }
    let lit = Huffman::new(&lengths[..hlit])?;
    let dist = Huffman::new(&lengths[hlit..])?;
    Ok((lit, dist))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::deflate::{Deflater, Level};

    fn roundtrip(data: &[u8], level: Level) {
        let mut compressed = Vec::new();
        let mut d = Deflater::new(level);
        d.write(data, &mut compressed).unwrap();
        d.finish(&mut compressed).unwrap();
        let mut out = Vec::new();
        let res = inflate(&compressed[..], &mut out, Some(data.len() as u64)).unwrap();
        assert_eq!(out, data, "niveau {:?}", level);
        assert_eq!(res.bytes, data.len() as u64);
        assert_eq!(res.crc32, crate::archive::crc32::crc32(data));
    }

    #[test]
    fn roundtrip_various() {
        roundtrip(b"", Level::Normal);
        roundtrip(b"a", Level::Normal);
        roundtrip(b"aaaa", Level::Normal);
        roundtrip(&vec![0u8; 300_000], Level::Max);
        let text = b"Le renard brun rapide saute par-dessus le chien paresseux. ".repeat(5000);
        roundtrip(&text, Level::Normal);
        let mut state = 42u64;
        let random: Vec<u8> = (0..500_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state & 0xFF) as u8
            })
            .collect();
        roundtrip(&random, Level::Normal);
        roundtrip(&random, Level::Store);
    }
}
