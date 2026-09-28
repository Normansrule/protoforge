//! IMA ADPCM (4 bits per sample) — how Iris squeezes 16 kHz microphone audio
//! into a Bluetooth Low Energy (BLE) link: 256 kbit/s of 16-bit PCM becomes
//! 64 kbit/s, which fits comfortably in a typical phone connection.
//!
//! Algorithm: Interactive Multimedia Association, "Recommended Practices for
//! Enhancing Digital Audio Compatibility in Multimedia Systems", rev. 3.00,
//! 1992 (the IMA/DVI ADPCM step and index tables). The encoder tracks the
//! decoder's predictor exactly, so errors never accumulate. Nibbles are
//! packed low nibble first (the WAV `0x0011` convention).

pub const STEP: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190,
    209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500,
    20350, 22385, 24623, 27086, 29794, 32767,
];

pub const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub pred: i32,
    pub index: i32,
}

fn update(st: &mut State, code: u8) {
    let step = STEP[st.index as usize];
    let mut vpdiff = step >> 3;
    if code & 4 != 0 {
        vpdiff += step;
    }
    if code & 2 != 0 {
        vpdiff += step >> 1;
    }
    if code & 1 != 0 {
        vpdiff += step >> 2;
    }
    if code & 8 != 0 {
        st.pred -= vpdiff;
    } else {
        st.pred += vpdiff;
    }
    st.pred = st.pred.clamp(-32768, 32767);
    st.index = (st.index + INDEX[(code & 15) as usize]).clamp(0, 88);
}

pub fn encode_sample(st: &mut State, x: i16) -> u8 {
    let step = STEP[st.index as usize];
    let mut diff = x as i32 - st.pred;
    let mut code = 0u8;
    if diff < 0 {
        code = 8;
        diff = -diff;
    }
    if diff >= step {
        code |= 4;
        diff -= step;
    }
    if diff >= step >> 1 {
        code |= 2;
        diff -= step >> 1;
    }
    if diff >= step >> 2 {
        code |= 1;
    }
    update(st, code);
    code
}

pub fn decode_sample(st: &mut State, code: u8) -> i16 {
    update(st, code);
    st.pred as i16
}

/// Encode (two samples per byte, low nibble first).
pub fn encode(x: &[i16], st: &mut State) -> Vec<u8> {
    let mut out = vec![0u8; x.len().div_ceil(2)];
    for (i, &s) in x.iter().enumerate() {
        let c = encode_sample(st, s);
        out[i / 2] |= if i % 2 == 0 { c } else { c << 4 };
    }
    out
}

pub fn decode(b: &[u8], n: usize, st: &mut State) -> Vec<i16> {
    (0..n.min(b.len() * 2))
        .map(|i| {
            let c = if i % 2 == 0 { b[i / 2] & 15 } else { b[i / 2] >> 4 };
            decode_sample(st, c)
        })
        .collect()
}

/// C implementation (identical arithmetic).
pub fn c_source() -> String {
    let step = STEP.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    let idx = INDEX.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    format!(
        r#"/* IMA ADPCM encoder (IMA "Recommended Practices ...", rev. 3.00, 1992). */
static const int16_t IRIS_ADPCM_STEP[89] = {{{step}}};
static const int8_t IRIS_ADPCM_INDEX[16] = {{{idx}}};

static void iris_adpcm_update(iris_adpcm_t *st, uint8_t code) {{
    int32_t step = IRIS_ADPCM_STEP[st->index];
    int32_t vpdiff = step >> 3;
    if (code & 4) vpdiff += step;
    if (code & 2) vpdiff += step >> 1;
    if (code & 1) vpdiff += step >> 2;
    int32_t p = st->pred + ((code & 8) ? -vpdiff : vpdiff);
    st->pred = (int16_t)(p > 32767 ? 32767 : (p < -32768 ? -32768 : p));
    int32_t ix = st->index + IRIS_ADPCM_INDEX[code & 15];
    st->index = (uint8_t)(ix < 0 ? 0 : (ix > 88 ? 88 : ix));
}}

/* Encode n samples into (n + 1) / 2 bytes, low nibble first. */
void iris_adpcm_encode(iris_adpcm_t *st, const int16_t *x, int32_t n, uint8_t *out) {{
    for (int32_t i = 0; i < n; i++) {{
        int32_t step = IRIS_ADPCM_STEP[st->index];
        int32_t diff = (int32_t)x[i] - st->pred;
        uint8_t code = 0;
        if (diff < 0) {{ code = 8; diff = -diff; }}
        if (diff >= step) {{ code |= 4; diff -= step; }}
        if (diff >= (step >> 1)) {{ code |= 2; diff -= step >> 1; }}
        if (diff >= (step >> 2)) {{ code |= 1; }}
        iris_adpcm_update(st, code);
        if ((i & 1) == 0) out[i >> 1] = code;
        else out[i >> 1] |= (uint8_t)(code << 4);
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_snr_of_a_tone() {
        let x: Vec<i16> = (0..1600)
            .map(|n| (8000.0 * (2.0 * std::f64::consts::PI * 440.0 * n as f64 / 16000.0).sin()) as i16)
            .collect();
        let enc = encode(&x, &mut State::default());
        assert_eq!(enc.len(), 800);
        let y = decode(&enc, x.len(), &mut State::default());
        // skip the first 20 ms while the step size adapts
        let (s, e) = x.iter().zip(&y).skip(320).fold((0.0, 0.0), |(s, e), (a, b)| {
            let (a, b) = (*a as f64, *b as f64);
            (s + a * a, e + (a - b) * (a - b))
        });
        let snr = 10.0 * (s / e).log10();
        assert!(snr > 20.0, "ADPCM SNR {snr:.1} dB");
    }

    #[test]
    fn encoder_tracks_decoder_state() {
        let x: Vec<i16> = (0..257).map(|n| ((n * 97) % 2000 - 1000) as i16).collect();
        let mut se = State::default();
        let enc = encode(&x, &mut se);
        let mut sd = State::default();
        let _ = decode(&enc, x.len(), &mut sd);
        assert_eq!(se, sd);
        assert_eq!(STEP.len(), 89);
        assert!(c_source().contains("IRIS_ADPCM_STEP[89]"));
    }
}
