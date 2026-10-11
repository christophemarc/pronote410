//! CRC-32 (IEEE 802.3), tel qu'utilisé par le conteneur ZIP.

const fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static TABLE: [u32; 256] = make_table();

/// Implémentation incrémentale (streaming) du CRC-32.
#[derive(Debug, Clone)]
pub struct Crc32 {
    value: u32,
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc32 {
    pub const fn new() -> Self {
        Crc32 { value: 0 }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut crc = self.value ^ 0xFFFF_FFFF;
        for &b in data {
            crc = TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
        }
        self.value = crc ^ 0xFFFF_FFFF;
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }
}

/// CRC-32 d'un tampon complet.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = Crc32::new();
    c.update(data);
    c.value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    }

    #[test]
    fn streaming_equals_oneshot() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let mut c = Crc32::new();
        for chunk in data.chunks(7) {
            c.update(chunk);
        }
        assert_eq!(c.value(), crc32(&data));
    }
}
