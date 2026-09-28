//! lz-string's `compressToEncodedURIComponent` / `decompressFromEncodedURIComponent`
//! (pieroxy, <https://github.com/pieroxy/lz-string>, MIT licence), ported
//! to Rust bit for bit so Protoforge can build Falstad CircuitJS links
//! (`circuitjs.html?ctz=…` is decompressed with this exact function).
//!
//! The algorithm is LZW over UTF-16 code units: the dictionary starts with
//! three control codes (0 = next literal is 8-bit, 1 = next literal is
//! 16-bit, 2 = end of stream); every new phrase `w + c` gets the next code,
//! and the code width grows by one bit each time the dictionary size passes
//! a power of two. Bits are packed LSB-first into 6-bit groups that map to
//! a URL-safe alphabet (`A–Z a–z 0–9 + - $`).

use std::collections::{HashMap, HashSet};

const KEY_URI_SAFE: &[u8; 65] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-$";

struct BitWriter<'a> {
    bits_per_char: u32,
    val: u32,
    pos: u32,
    out: Vec<u32>,
    map: &'a dyn Fn(u32) -> u32,
}

impl BitWriter<'_> {
    /// Push `n` bits of `value`, least significant bit first.
    fn write_lsb(&mut self, mut value: u32, n: u32) {
        for _ in 0..n {
            self.push_bit(value & 1);
            value >>= 1;
        }
    }
    fn push_bit(&mut self, b: u32) {
        self.val = (self.val << 1) | b;
        if self.pos == self.bits_per_char - 1 {
            self.pos = 0;
            self.out.push((self.map)(self.val));
            self.val = 0;
        } else {
            self.pos += 1;
        }
    }
}

/// Generic `_compress` over UTF-16 code units.
fn compress_units(input: &[u16], bits_per_char: u32, map: &dyn Fn(u32) -> u32) -> Vec<u32> {
    let mut dict: HashMap<Vec<u16>, u32> = HashMap::new();
    let mut to_create: HashSet<Vec<u16>> = HashSet::new();
    let mut w: Vec<u16> = Vec::new();
    let mut enlarge_in: u32 = 2;
    let mut dict_size: u32 = 3;
    let mut num_bits: u32 = 2;
    let mut bw = BitWriter {
        bits_per_char,
        val: 0,
        pos: 0,
        out: Vec::new(),
        map,
    };

    let emit_w = |w: &Vec<u16>,
                  bw: &mut BitWriter,
                  dict: &HashMap<Vec<u16>, u32>,
                  to_create: &mut HashSet<Vec<u16>>,
                  enlarge_in: &mut u32,
                  num_bits: &mut u32| {
        if to_create.contains(w) {
            let ch = w[0] as u32;
            if ch < 256 {
                bw.write_lsb(0, *num_bits);
                bw.write_lsb(ch, 8);
            } else {
                bw.write_lsb(1, *num_bits);
                bw.write_lsb(ch, 16);
            }
            *enlarge_in -= 1;
            if *enlarge_in == 0 {
                *enlarge_in = 1 << *num_bits;
                *num_bits += 1;
            }
            to_create.remove(w);
        } else {
            bw.write_lsb(dict[w], *num_bits);
        }
        *enlarge_in -= 1;
        if *enlarge_in == 0 {
            *enlarge_in = 1 << *num_bits;
            *num_bits += 1;
        }
    };

    for &c in input {
        let cv = vec![c];
        if !dict.contains_key(&cv) {
            dict.insert(cv.clone(), dict_size);
            dict_size += 1;
            to_create.insert(cv.clone());
        }
        let mut wc = w.clone();
        wc.push(c);
        if dict.contains_key(&wc) {
            w = wc;
        } else {
            emit_w(&w, &mut bw, &dict, &mut to_create, &mut enlarge_in, &mut num_bits);
            dict.insert(wc, dict_size);
            dict_size += 1;
            w = cv;
        }
    }
    if !w.is_empty() {
        emit_w(&w, &mut bw, &dict, &mut to_create, &mut enlarge_in, &mut num_bits);
    }
    // end of stream
    bw.write_lsb(2, num_bits);
    // flush the last char
    loop {
        bw.val <<= 1;
        if bw.pos == bits_per_char - 1 {
            bw.out.push((bw.map)(bw.val));
            break;
        }
        bw.pos += 1;
    }
    bw.out
}

/// `LZString.compressToEncodedURIComponent(input)`.
pub fn compress_to_encoded_uri_component(input: &str) -> String {
    let units: Vec<u16> = input.encode_utf16().collect();
    let map = |v: u32| KEY_URI_SAFE[v as usize] as u32;
    compress_units(&units, 6, &map).into_iter().map(|c| c as u8 as char).collect()
}

/// `LZString.decompressFromEncodedURIComponent(input)`; `None` on corrupt data.
pub fn decompress_from_encoded_uri_component(input: &str) -> Option<String> {
    if input.is_empty() {
        return None;
    }
    let vals: Vec<u32> = input
        .replace(' ', "+")
        .bytes()
        .map(|b| KEY_URI_SAFE.iter().position(|k| *k == b).map(|p| p as u32).unwrap_or(0))
        .collect();
    let units = decompress_units(vals.len(), 32, |i| vals.get(i).copied().unwrap_or(0))?;
    String::from_utf16(&units).ok()
}

fn decompress_units(length: usize, reset: u32, next: impl Fn(usize) -> u32) -> Option<Vec<u16>> {
    let mut dict: Vec<Vec<u16>> = vec![vec![], vec![], vec![]];
    let mut enlarge_in: u32 = 4;
    let mut dict_size: u32 = 4;
    let mut num_bits: u32 = 3;
    let mut result: Vec<u16> = Vec::new();
    let (mut val, mut position, mut index) = (next(0), reset, 1usize);
    let read = |n: u32, val: &mut u32, position: &mut u32, index: &mut usize| -> u32 {
        let mut bits = 0u32;
        for k in 0..n {
            let resb = *val & *position;
            *position >>= 1;
            if *position == 0 {
                *position = reset;
                *val = next(*index);
                *index += 1;
            }
            if resb > 0 {
                bits |= 1 << k;
            }
        }
        bits
    };
    let first = read(2, &mut val, &mut position, &mut index);
    let c: Vec<u16> = match first {
        0 => vec![read(8, &mut val, &mut position, &mut index) as u16],
        1 => vec![read(16, &mut val, &mut position, &mut index) as u16],
        _ => return Some(vec![]),
    };
    dict.push(c.clone());
    let mut w = c.clone();
    result.extend_from_slice(&c);
    loop {
        if index > length {
            return Some(vec![]);
        }
        let mut code = read(num_bits, &mut val, &mut position, &mut index);
        match code {
            0 | 1 => {
                let n = if code == 0 { 8 } else { 16 };
                let lit = read(n, &mut val, &mut position, &mut index) as u16;
                dict.push(vec![lit]);
                dict_size += 1;
                code = dict_size - 1;
                enlarge_in -= 1;
            }
            2 => return Some(result),
            _ => {}
        }
        if enlarge_in == 0 {
            enlarge_in = 1 << num_bits;
            num_bits += 1;
        }
        let entry: Vec<u16> = if (code as usize) < dict.len() && !dict[code as usize].is_empty() {
            dict[code as usize].clone()
        } else if code == dict_size {
            let mut e = w.clone();
            e.push(w[0]);
            e
        } else {
            return None;
        };
        result.extend_from_slice(&entry);
        let mut add = w.clone();
        add.push(entry[0]);
        dict.push(add);
        dict_size += 1;
        enlarge_in -= 1;
        w = entry;
        if enlarge_in == 0 {
            enlarge_in = 1 << num_bits;
            num_bits += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors produced by the reference JavaScript lz-string 1.5.0 (npm).
    const VECTORS: &[(&str, &str)] = &[
        ("", "Q"),
        ("a", "IZA"),
        ("hello world", "BYUwNmD2AEDukCcwBMg"),
        ("hello hello hello hello", "BYUwNmD2AEoTcq3FIA"),
        (
            "$ 1 0.000005 10.2 50 5 50\nr 176 80 384 80 0 1000\nc 384 80 384 352 0 0.000015 0\n",
            "CQAgjCAMB0l3BWcMBMIGXeyAoATuAOwBsIAHJgMxkAs5mmY8OAxiNXRe7ewmg7HhgkuIA",
        ),
        ("Ωµ€ — unicode ✓ 😀", "pXAFbDUEAEgoBBCuB2BLAxgewCYFMKGRyCheDcAA9oA"),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "IY18ZXTt2Q",
        ),
    ];

    #[test]
    fn matches_reference_javascript() {
        for (plain, packed) in VECTORS {
            assert_eq!(compress_to_encoded_uri_component(plain), *packed, "{plain:?}");
        }
    }

    #[test]
    fn round_trips() {
        for (plain, packed) in VECTORS.iter().skip(1) {
            assert_eq!(decompress_from_encoded_uri_component(packed).as_deref(), Some(*plain));
        }
        let mut rng = crate::math::Rng::new(7);
        for len in [1usize, 5, 64, 500, 3000] {
            let s: String = (0..len)
                .map(|_| {
                    let r = rng.uniform();
                    if r < 0.7 {
                        (b'a' + (rng.uniform() * 6.0) as u8) as char
                    } else if r < 0.9 {
                        ' '
                    } else {
                        'Ω'
                    }
                })
                .collect();
            let c = compress_to_encoded_uri_component(&s);
            assert!(c.bytes().all(|b| KEY_URI_SAFE.contains(&b)));
            assert_eq!(decompress_from_encoded_uri_component(&c).as_deref(), Some(s.as_str()), "len {len}");
        }
    }
}
