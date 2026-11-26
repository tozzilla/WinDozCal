//! Icona del tray con il numero del giorno: un foglio di calendario 32x32 disegnato in memoria
//! (RGBA), senza font di sistema né dipendenze. Le cifre sono un font bitmap 5x7 a scala 2, che a
//! 16 px (tray al 100%) resta leggibile come 5x7.

pub const SIZE: u32 = 32;

const HEADER: [u8; 4] = [0xE4, 0x26, 0x2B, 0xFF]; // --onair
const PAGE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const INK: [u8; 4] = [0x10, 0x14, 0x2A, 0xFF]; // --rail
const BORDER: [u8; 4] = [0x5A, 0x60, 0x7A, 0xFF];

/// Cifre 0-9, 7 righe da 5 bit (bit 4 = colonna di sinistra).
const DIGITS: [[u8; 7]; 10] = [
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    [
        0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
    ],
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ],
];
const SCALE: u32 = 2;
const DIGIT_W: u32 = 5 * SCALE;
const DIGIT_GAP: u32 = 2;

struct Canvas(Vec<u8>);

impl Canvas {
    fn set(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x < SIZE && y < SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            self.0[i..i + 4].copy_from_slice(&c);
        }
    }

    fn rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, c: [u8; 4]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, c);
            }
        }
    }
}

/// Pixel RGBA (`SIZE` x `SIZE`, righe dall'alto) dell'icona per il giorno del mese `day` (1-31).
pub fn day_icon_rgba(day: u32) -> Vec<u8> {
    let mut c = Canvas(vec![0; (SIZE * SIZE * 4) as usize]);
    let (left, right, top, bottom) = (2, SIZE - 3, 5, SIZE - 2);

    // Foglio con bordo e angoli smussati, intestazione rossa, due anelli.
    c.rect(left, top, right, bottom, BORDER);
    c.rect(left + 1, top + 1, right - 1, bottom - 1, PAGE);
    c.rect(left + 1, top + 1, right - 1, top + 6, HEADER);
    for (x, y) in [(left, top), (right, top), (left, bottom), (right, bottom)] {
        c.set(x, y, [0; 4]);
    }
    // Coordinate pari: a 16 px (riduzione 2:1) ogni blocco 2x2 resta un pixel pieno.
    for x in [9, SIZE - 9] {
        c.rect(x - 1, top - 3, x, top + 2, INK);
    }

    // Numero centrato nella parte bianca.
    let text: Vec<usize> = day
        .clamp(1, 31)
        .to_string()
        .bytes()
        .map(|b| (b - b'0') as usize)
        .collect();
    let width = text.len() as u32 * DIGIT_W + (text.len() as u32 - 1) * DIGIT_GAP;
    let body_top = top + 7;
    let x0 = ((SIZE - width) / 2) & !1;
    let y0 = (body_top + (bottom - body_top - 7 * SCALE) / 2) & !1;
    for (i, d) in text.iter().enumerate() {
        let dx = x0 + i as u32 * (DIGIT_W + DIGIT_GAP);
        for (row, bits) in DIGITS[*d].iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    let (px, py) = (dx + col * SCALE, y0 + row as u32 * SCALE);
                    c.rect(px, py, px + SCALE - 1, py + SCALE - 1, INK);
                }
            }
        }
    }
    c.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(img: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * SIZE + x) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn draws_header_page_and_distinct_days() {
        let img = day_icon_rgba(26);
        assert_eq!(img.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(px(&img, 16, 8), HEADER);
        assert_eq!(px(&img, 0, 0)[3], 0, "angolo trasparente");
        assert!(img.chunks(4).any(|p| p == INK), "cifre presenti");
        let days: Vec<Vec<u8>> = (1..=31).map(day_icon_rgba).collect();
        for (i, a) in days.iter().enumerate() {
            for b in &days[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    /// `WINDOZCAL_ICON_DUMP=<dir> cargo test tray_icon` salva le icone grezze per un controllo visivo.
    #[test]
    fn dump_for_visual_check() {
        if let Ok(dir) = std::env::var("WINDOZCAL_ICON_DUMP") {
            for day in [1, 8, 18, 26, 31] {
                std::fs::write(format!("{dir}/day{day}.rgba"), day_icon_rgba(day)).unwrap();
            }
        }
    }
}
