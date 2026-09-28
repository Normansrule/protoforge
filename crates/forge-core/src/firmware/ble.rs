//! Iris Bluetooth Low Energy (BLE) protocol: a Generic Attribute Profile
//! (GATT) design, packed payload formats, and link-layer throughput and
//! latency arithmetic.
//!
//! **Air time.** An uncoded LE packet is preamble (1 byte on the 1M PHY, 2 on
//! 2M) + access address (4) + header (2) + payload + message integrity check
//! (4, only when encrypted) + CRC (3); at 1 µs/bit (1M) or 0.5 µs/bit (2M).
//! Consecutive packets in a connection event are separated by the
//! inter-frame space T_IFS = 150 µs, and every peripheral packet answers a
//! central packet (empty = no payload), so one data packet costs
//! `T_data + T_IFS + T_empty + T_IFS`. Without the Data Length Extension
//! (DLE, Bluetooth 4.2+) the payload is at most 27 bytes; with it, 251.
//! A notification carries MTU − 3 bytes of data in an attribute protocol
//! (ATT) packet (3-byte header) inside a logical link control and adaptation
//! protocol (L2CAP) frame (4-byte header). Bluetooth Core Specification
//! v5.x, Vol 6 Part B (link layer) and Vol 3 Parts A/F/G (L2CAP, ATT, GATT).
//!
//! **What limits real throughput** is usually the phone: how many packets it
//! allows per connection event and which interval it grants. Those are
//! inputs here (`max_packets_per_event`), stated as typical assumptions.
//!
//! Standard services use Bluetooth SIG assigned numbers: Battery Service
//! 0x180F (Battery Level 0x2A19), Device Information 0x180A (Manufacturer
//! Name 0x2A29, Model Number 0x2A24, Firmware Revision 0x2A26, Hardware
//! Revision 0x2A27), Client Characteristic Configuration descriptor 0x2902.

use serde::{Deserialize, Serialize};

// ------------------------------------------------------------------ UUIDs

/// SHA-1 (FIPS 180-4) — only used to derive name-based UUIDs, not for security.
pub fn sha1(msg: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let mut m = msg.to_vec();
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in m.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn uuid_string(b: &[u8]) -> String {
    let hx: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hx[0..8], &hx[8..12], &hx[12..16], &hx[16..20], &hx[20..32])
}

/// RFC 4122 name space for URLs (6ba7b811-9dad-11d1-80b4-00c04fd430c8).
pub const NAMESPACE_URL: &str = "6ba7b811-9dad-11d1-80b4-00c04fd430c8";

/// Name-based UUID, version 5 (RFC 4122 §4.3): SHA-1 of name space + name,
/// first 16 bytes, version nibble 5, variant bits 10.
pub fn uuid_v5(namespace: &str, name: &str) -> String {
    let mut msg = uuid_le_bytes(namespace);
    msg.reverse(); // back to network (big-endian) order
    msg.extend_from_slice(name.as_bytes());
    let d = sha1(&msg);
    let mut b = [0u8; 16];
    b.copy_from_slice(&d[..16]);
    b[6] = (b[6] & 0x0f) | 0x50;
    b[8] = (b[8] & 0x3f) | 0x80;
    uuid_string(&b)
}

/// The project's custom 128-bit service base: a version-5 UUID of
/// `protoforge:<project name>:hud-service`, with bytes 2–3 (hex digits 5–8)
/// cleared so characteristics can be numbered 0001, 0002, … — the same
/// "base + 16-bit alias" layout vendor examples use. It is deterministic
/// (same project → same UUIDs, so firmware and phone app agree) and
/// **project-specific**: not registered with the Bluetooth SIG.
pub fn service_base(project_name: &str) -> String {
    let u = uuid_v5(
        NAMESPACE_URL,
        &format!("protoforge:{}:hud-service", project_name.trim().to_lowercase()),
    );
    format!("{}0000{}", &u[0..4], &u[8..])
}

/// Characteristic `short` of a service base (replaces hex digits 5–8).
pub fn char_uuid(base: &str, short: u16) -> String {
    format!("{}{short:04x}{}", &base[0..4], &base[8..])
}

/// The five `BT_UUID_128_ENCODE(w32, w1, w2, w3, w48)` arguments of a UUID string (Zephyr).
pub fn zephyr_encode_args(u: &str) -> String {
    let g: Vec<&str> = u.split('-').collect();
    format!("0x{}, 0x{}, 0x{}, 0x{}, 0x{}", g[0], g[1], g[2], g[3], g[4])
}

/// Little-endian byte array of a 128-bit UUID string (the order BLE stacks store it).
pub fn uuid_le_bytes(u: &str) -> Vec<u8> {
    let hex: String = u.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let mut b: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0))
        .collect();
    b.reverse();
    b
}

#[derive(Clone, Debug, Serialize)]
pub struct Field {
    pub name: String,
    pub ctype: String,
    pub bytes: u32,
    pub desc: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Characteristic {
    pub name: String,
    pub c_name: String,
    pub uuid: String,
    pub properties: Vec<String>,
    pub direction: String,
    pub description: String,
    /// Packed payload header (variable UTF-8 text / audio follows when `trailing` is set).
    pub fields: Vec<Field>,
    pub header_bytes: u32,
    pub trailing: String,
    pub max_len: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Service {
    pub name: String,
    pub uuid: String,
    pub standard: bool,
    pub characteristics: Vec<Characteristic>,
}

fn f(name: &str, ctype: &str, desc: &str) -> Field {
    let bytes = match ctype {
        "uint8_t" | "int8_t" => 1,
        "uint16_t" | "int16_t" => 2,
        "uint32_t" | "int32_t" => 4,
        _ => 0,
    };
    Field {
        name: name.into(),
        ctype: ctype.into(),
        bytes,
        desc: desc.into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn chr(
    name: &str,
    c_name: &str,
    uuid: String,
    props: &[&str],
    dir: &str,
    desc: &str,
    fields: Vec<Field>,
    trailing: &str,
    max_len: u32,
) -> Characteristic {
    let header_bytes = fields.iter().map(|x| x.bytes).sum();
    Characteristic {
        name: name.into(),
        c_name: c_name.into(),
        uuid,
        properties: props.iter().map(|s| s.to_string()).collect(),
        direction: dir.into(),
        description: desc.into(),
        fields,
        header_bytes,
        trailing: trailing.into(),
        max_len,
    }
}

/// The Iris GATT table. `mtu` bounds the variable-length characteristics.
pub fn gatt(mtu: u32, base: &str) -> Vec<Service> {
    let data = mtu.clamp(23, 517) - 3;
    let p2g = "phone → glasses";
    let g2p = "glasses → phone";
    vec![
        Service {
            name: "Iris HUD (custom, project-specific)".into(),
            uuid: base.into(),
            standard: false,
            characteristics: vec![
                chr(
                    "Notification",
                    "notif",
                    char_uuid(base, 1),
                    &["write", "write without response"],
                    p2g,
                    "A phone notification to show on the Notification screen.",
                    vec![
                        f(
                            "app_id",
                            "uint8_t",
                            "source app category (0 other, 1 messages, 2 calendar, 3 call …)",
                        ),
                        f("flags", "uint8_t", "bit0 = high priority (wake display), bit1 = replace previous"),
                        f("id", "uint16_t", "notification id (for dismiss)"),
                        f("title_len", "uint8_t", "bytes of UTF-8 title that follow"),
                        f("body_len", "uint8_t", "bytes of UTF-8 body after the title"),
                    ],
                    "UTF-8 title then body (non-ASCII shows as '?' in the 5×7 font)",
                    data,
                ),
                chr(
                    "Navigation",
                    "nav",
                    char_uuid(base, 2),
                    &["write without response"],
                    p2g,
                    "Turn-by-turn guidance from the phone's maps app.",
                    vec![
                        f(
                            "maneuver",
                            "uint8_t",
                            "0 straight, 1 slight right, 2 right, 3 sharp right, 4 U-turn, 5–7 left mirror, 8 arrive",
                        ),
                        f("reserved", "uint8_t", "0"),
                        f("arrow_deg", "int16_t", "arrow direction, degrees clockwise from straight ahead"),
                        f("distance_m", "uint32_t", "metres to the manoeuvre"),
                        f("eta_min", "uint16_t", "minutes to destination"),
                        f("street_len", "uint8_t", "bytes of UTF-8 street name that follow"),
                    ],
                    "UTF-8 street name",
                    (11 + 31).min(data),
                ),
                chr(
                    "Captions",
                    "captions",
                    char_uuid(base, 3),
                    &["write without response"],
                    p2g,
                    "Live transcription text streamed back from the phone's speech recogniser.",
                    vec![
                        f("seq", "uint16_t", "increments per message; gaps reveal drops"),
                        f("flags", "uint8_t", "bit0 = final (else partial hypothesis), bit1 = clear screen"),
                        f("len", "uint8_t", "bytes of UTF-8 text that follow"),
                    ],
                    "UTF-8 caption text",
                    data,
                ),
                chr(
                    "Microphone audio",
                    "audio",
                    char_uuid(base, 4),
                    &["notify"],
                    g2p,
                    "IMA ADPCM, 16 kHz mono, 4 bits/sample, for the phone's speech recogniser.",
                    vec![
                        f("seq", "uint16_t", "block counter"),
                        f("predictor", "int16_t", "ADPCM predictor at block start (resync after loss)"),
                        f("step_index", "uint8_t", "ADPCM step index at block start"),
                        f("n_samples", "uint8_t", "samples in this block ÷ 2 (bytes of ADPCM that follow)"),
                    ],
                    "ADPCM nibbles, low nibble first",
                    data,
                ),
                chr(
                    "Display control",
                    "display",
                    char_uuid(base, 5),
                    &["read", "write"],
                    p2g,
                    "Brightness, active screen and power-mode override.",
                    vec![
                        f("brightness_pct", "uint8_t", "0–100"),
                        f(
                            "screen",
                            "uint8_t",
                            "0 clock, 1 navigation, 2 captions, 3 notification, 4 teleprompter",
                        ),
                        f("mode", "uint8_t", "0 auto (learned policy), 1 off, 2 low refresh, 3 full"),
                        f("reserved", "uint8_t", "0"),
                    ],
                    "",
                    4,
                ),
                chr(
                    "Gesture events",
                    "gesture",
                    char_uuid(base, 6),
                    &["notify"],
                    g2p,
                    "Touch-strip gestures recognised by the gesture state machine.",
                    vec![
                        f("gesture", "uint8_t", "1 tap, 2 double tap, 3 long press"),
                        f("screen", "uint8_t", "screen shown when it happened"),
                        f("reserved", "uint16_t", "0"),
                        f("uptime_ms", "uint32_t", "milliseconds since boot"),
                    ],
                    "",
                    8,
                ),
                chr(
                    "Status",
                    "status",
                    char_uuid(base, 7),
                    &["read", "notify"],
                    g2p,
                    "Power-policy decision and link state, for the phone app's debug view.",
                    vec![
                        f("context", "uint8_t", "0 idle, 1 glance, 2 navigate, 3 caption"),
                        f("display_mode", "uint8_t", "0 off, 1 low, 2 full"),
                        f("radio_mode", "uint8_t", "0 fast interval, 1 slow interval"),
                        f("soc_bucket", "uint8_t", "battery bucket used by the policy table"),
                        f("battery_mv", "uint16_t", "battery voltage"),
                        f("conn_interval_x1250us", "uint16_t", "current connection interval in 1.25 ms units"),
                    ],
                    "",
                    8,
                ),
            ],
        },
        Service {
            name: "Battery Service".into(),
            uuid: "0x180F".into(),
            standard: true,
            characteristics: vec![chr(
                "Battery Level",
                "battery_level",
                "0x2A19".into(),
                &["read", "notify"],
                g2p,
                "State of charge, percent (Bluetooth SIG Battery Service).",
                vec![f("level", "uint8_t", "0–100 %")],
                "",
                1,
            )],
        },
        Service {
            name: "Device Information".into(),
            uuid: "0x180A".into(),
            standard: true,
            characteristics: vec![
                chr(
                    "Manufacturer Name String",
                    "dis_manufacturer",
                    "0x2A29".into(),
                    &["read"],
                    g2p,
                    "UTF-8 string.",
                    vec![],
                    "UTF-8",
                    20,
                ),
                chr(
                    "Model Number String",
                    "dis_model",
                    "0x2A24".into(),
                    &["read"],
                    g2p,
                    "UTF-8 string, e.g. \"Iris v0\".",
                    vec![],
                    "UTF-8",
                    20,
                ),
                chr(
                    "Firmware Revision String",
                    "dis_fw",
                    "0x2A26".into(),
                    &["read"],
                    g2p,
                    "UTF-8 string.",
                    vec![],
                    "UTF-8",
                    20,
                ),
                chr(
                    "Hardware Revision String",
                    "dis_hw",
                    "0x2A27".into(),
                    &["read"],
                    g2p,
                    "UTF-8 string.",
                    vec![],
                    "UTF-8",
                    20,
                ),
            ],
        },
    ]
}

// ------------------------------------------------------------------ link maths

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkParams {
    pub conn_interval_ms: f64,
    pub slow_interval_ms: f64,
    pub mtu: u32,
    /// "1M" or "2M".
    pub phy: String,
    /// Data Length Extension (251-byte link-layer payloads).
    pub dle: bool,
    pub encrypted: bool,
    /// Packets the phone lets the peripheral send per connection event (typical 4–7; phone-dependent).
    pub max_packets_per_event: u32,
    /// Peripheral (slave) latency: connection events the peripheral may skip.
    pub peripheral_latency: u32,
    /// Speech-recogniser time on the phone (an assumption, not a BLE quantity).
    pub asr_ms: f64,
    /// Audio block duration buffered before sending.
    pub audio_frame_ms: f64,
    pub caption_chars: u32,
    pub audio_rate_hz: f64,
    pub audio_bits: u32,
}

impl Default for LinkParams {
    fn default() -> Self {
        LinkParams {
            conn_interval_ms: 30.0,
            slow_interval_ms: 300.0,
            mtu: 247,
            phy: "2M".into(),
            dle: true,
            encrypted: true,
            max_packets_per_event: 6,
            peripheral_latency: 0,
            asr_ms: 300.0,
            audio_frame_ms: 20.0,
            caption_chars: 40,
            audio_rate_hz: 16000.0,
            audio_bits: 4,
        }
    }
}

/// Bytes of the audio notification header (`iris_audio_hdr_t`).
pub const AUDIO_HDR: u32 = 6;

#[derive(Clone, Debug, Serialize)]
pub struct Slot {
    pub t_us: f64,
    pub dur_us: f64,
    /// "central" or "peripheral".
    pub from: String,
    pub bytes: u32,
    pub data: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LinkResult {
    pub params: LinkParams,
    pub bit_us: f64,
    pub ll_payload_max: u32,
    pub att_data_per_notification: u32,
    pub ll_packets_per_notification: u32,
    pub data_packet_us: f64,
    pub empty_packet_us: f64,
    pub exchange_us: f64,
    pub packets_time_limited: u32,
    pub packets_per_event: u32,
    pub app_bytes_per_event: f64,
    pub throughput_kbps: f64,
    pub slow_throughput_kbps: f64,
    pub raw_phy_kbps: f64,
    pub efficiency: f64,
    pub audio_demand_kbps: f64,
    pub audio_margin: f64,
    /// Share of the fast-interval application throughput the audio uplink uses.
    pub audio_util_pct: f64,
    pub audio_frame_samples: u32,
    pub audio_frame_payload_bytes: u32,
    pub audio_notifications_per_frame: u32,
    pub audio_bytes_per_frame: u32,
    /// Uncompressed 16-bit PCM rate for comparison.
    pub pcm_kbps: f64,
    pub compression_ratio: f64,
    pub caption_downlink_avg_ms: f64,
    pub caption_downlink_worst_ms: f64,
    pub audio_uplink_avg_ms: f64,
    /// Audio frame + uplink + downlink (BLE and buffering only).
    pub caption_link_latency_ms: f64,
    /// Including the assumed speech-recogniser time.
    pub caption_end_to_end_ms: f64,
    pub event: Vec<Slot>,
    pub notes: Vec<String>,
}

pub fn link(p: &LinkParams) -> LinkResult {
    let mut p = p.clone();
    p.conn_interval_ms = p.conn_interval_ms.clamp(7.5, 4000.0);
    p.slow_interval_ms = p.slow_interval_ms.clamp(7.5, 4000.0);
    p.mtu = p.mtu.clamp(23, 517);
    p.max_packets_per_event = p.max_packets_per_event.clamp(1, 64);
    let two_m = p.phy.eq_ignore_ascii_case("2M");
    let bit_us = if two_m { 0.5 } else { 1.0 };
    let preamble = if two_m { 2 } else { 1 };
    let mic = if p.encrypted { 4 } else { 0 };
    let ll_max = if p.dle { 251 } else { 27 };
    let data = p.mtu - 3;
    let l2cap_sdu = data + 3 + 4;
    let ll_pkts = l2cap_sdu.div_ceil(ll_max);
    let ll_payload = |b: u32| (preamble + 4 + 2 + b + mic + 3) as f64 * 8.0 * bit_us;
    // the last fragment is shorter; average payload per packet
    let avg_payload = l2cap_sdu as f64 / ll_pkts as f64;
    let data_us = ll_payload(avg_payload.ceil() as u32);
    let full_us = ll_payload(ll_max.min(l2cap_sdu));
    let empty_us = (preamble + 4 + 2 + 3) as f64 * 8.0 * bit_us;
    let ifs = 150.0;
    let exchange = data_us + ifs + empty_us + ifs;
    let interval_us = p.conn_interval_ms * 1000.0;
    // keep ~1.25 ms of each interval for the radio's own scheduling margin
    let time_limited = ((interval_us - 1250.0).max(exchange) / exchange).floor() as u32;
    let per_event = time_limited.min(p.max_packets_per_event).max(1);
    let app_bytes_per_packet = data as f64 / ll_pkts as f64;
    let app_per_event = per_event as f64 * app_bytes_per_packet;
    let thr = app_per_event * 8.0 / p.conn_interval_ms; // bits per ms = kbit/s
    let slow_thr = app_per_event * 8.0 / p.slow_interval_ms;
    let raw = 1000.0 / bit_us;
    // audio: one 20 ms frame of IMA ADPCM per notification (split if it exceeds the MTU),
    // each notification carrying the 6-byte audio header (seq, predictor, step index, count)
    let frame_samples = (p.audio_rate_hz * p.audio_frame_ms / 1000.0).round().max(1.0);
    let frame_payload = (frame_samples * p.audio_bits as f64 / 8.0).ceil() as u32;
    let per_notif = data.saturating_sub(AUDIO_HDR).max(1);
    let notifs_per_frame = frame_payload.div_ceil(per_notif);
    let frame_bytes = frame_payload + AUDIO_HDR * notifs_per_frame;
    let pcm_kbps = p.audio_rate_hz * 16.0 / 1000.0;
    let audio_demand = frame_bytes as f64 * 8.0 / p.audio_frame_ms.max(1e-3);
    let cap_bytes = 4 + p.caption_chars;
    let cap_pkts = (cap_bytes + 3 + 4).div_ceil(ll_max) as f64;
    let air = cap_pkts * exchange / 1000.0;
    let ci = p.conn_interval_ms;
    let down_avg = ci / 2.0 + air;
    let down_worst = ci * (1.0 + p.peripheral_latency as f64) + air;
    let up_avg = ci / 2.0 + exchange / 1000.0;
    let link_lat = p.audio_frame_ms + up_avg + down_avg;
    let mut event = Vec::new();
    let mut t = 0.0;
    for k in 0..per_event {
        event.push(Slot {
            t_us: t,
            dur_us: empty_us,
            from: "central".into(),
            bytes: 0,
            data: false,
        });
        t += empty_us + ifs;
        event.push(Slot {
            t_us: t,
            dur_us: data_us,
            from: "peripheral".into(),
            bytes: if k + 1 == per_event && per_event > 1 {
                avg_payload.ceil() as u32
            } else {
                ll_max.min(l2cap_sdu)
            },
            data: true,
        });
        t += data_us + ifs;
    }
    let mut notes = vec![
        format!(
            "{} PHY, {} link-layer payloads, MTU {} → {} bytes of data per notification in {} link-layer packet(s)",
            if two_m { "LE 2M" } else { "LE 1M" },
            if p.dle { "251-byte (DLE)" } else { "27-byte" },
            p.mtu,
            data,
            ll_pkts
        ),
        format!(
            "one exchange = {:.0} µs data + 150 µs + {:.0} µs empty + 150 µs = {:.0} µs; {} fit in a {:.1} ms interval, the phone allows {} (assumption)",
            data_us, empty_us, exchange, time_limited, p.conn_interval_ms, p.max_packets_per_event
        ),
    ];
    if full_us > interval_us {
        notes.push("a single packet is longer than the interval — raise the interval".into());
    }
    if audio_demand > slow_thr {
        notes.push(format!(
            "the slow interval ({:.0} ms) carries only {:.0} kbit/s — captions need the fast interval, which is why the power policy switches the radio per context",
            p.slow_interval_ms, slow_thr
        ));
    }
    LinkResult {
        bit_us,
        ll_payload_max: ll_max,
        att_data_per_notification: data,
        ll_packets_per_notification: ll_pkts,
        data_packet_us: data_us,
        empty_packet_us: empty_us,
        exchange_us: exchange,
        packets_time_limited: time_limited,
        packets_per_event: per_event,
        app_bytes_per_event: app_per_event,
        throughput_kbps: thr,
        slow_throughput_kbps: slow_thr,
        raw_phy_kbps: raw,
        efficiency: thr / raw,
        audio_demand_kbps: audio_demand,
        audio_margin: thr / audio_demand.max(1e-9),
        audio_util_pct: audio_demand / thr.max(1e-9) * 100.0,
        audio_frame_samples: frame_samples as u32,
        audio_frame_payload_bytes: frame_payload,
        audio_notifications_per_frame: notifs_per_frame,
        audio_bytes_per_frame: frame_bytes,
        pcm_kbps,
        compression_ratio: pcm_kbps / (p.audio_rate_hz * p.audio_bits as f64 / 1000.0).max(1e-9),
        caption_downlink_avg_ms: down_avg,
        caption_downlink_worst_ms: down_worst,
        audio_uplink_avg_ms: up_avg,
        caption_link_latency_ms: link_lat,
        caption_end_to_end_ms: link_lat + p.asr_ms,
        event,
        notes,
        params: p,
    }
}

// ------------------------------------------------------------------ radio power

/// Peripheral radio power model. Currents are typical nRF52840 figures with
/// the DC/DC regulator enabled at 3 V (Nordic, *nRF52840 Product
/// Specification*, radio current consumption tables) — check the current
/// revision; per-event overheads are assumptions to replace with a
/// power-profiler measurement.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioModel {
    pub supply_v: f64,
    /// Transmit at 0 dBm.
    pub tx_ma: f64,
    /// Receive.
    pub rx_ma: f64,
    /// System ON sleep with the RTC running and RAM retained.
    pub sleep_ua: f64,
    /// Receive window widening + radio ramp-up before each event (µs, at the RX current).
    pub pre_event_us: f64,
    /// Protocol-stack processing per event (µs, at the CPU current).
    pub cpu_us: f64,
    pub cpu_ma: f64,
}

impl Default for RadioModel {
    fn default() -> Self {
        RadioModel {
            supply_v: 3.0,
            tx_ma: 4.8,
            rx_ma: 4.6,
            sleep_ua: 3.0,
            pre_event_us: 150.0,
            cpu_us: 200.0,
            cpu_ma: 3.0,
        }
    }
}

/// Energy of one connection event carrying `n` peripheral packets (fractional
/// allowed = average), each answering a central packet: µJ.
/// E = V·[I_rx·(t_pre + n·(t_c + T_IFS)) + I_tx·n·(t_p + T_IFS) + I_cpu·t_cpu].
pub fn event_energy_uj(l: &LinkResult, n: f64, periph_us: f64, rm: &RadioModel) -> f64 {
    let n = n.max(1.0);
    let rx_us = rm.pre_event_us + n * (l.empty_packet_us + 150.0);
    let tx_us = n * (periph_us + 150.0);
    rm.supply_v * (rm.rx_ma * rx_us + rm.tx_ma * tx_us + rm.cpu_ma * rm.cpu_us) / 1000.0
}

/// Average radio power (mW) of a link kept alive with empty packets.
pub fn idle_link_mw(l: &LinkResult, interval_ms: f64, rm: &RadioModel) -> f64 {
    let e = event_energy_uj(l, 1.0, l.empty_packet_us, rm);
    rm.supply_v * rm.sleep_ua / 1000.0 + e / interval_ms.max(7.5)
}

/// Average radio power (mW) while streaming the audio uplink at `interval_ms`,
/// or `None` if one event cannot carry what accumulates in one interval.
pub fn stream_mw(l: &LinkResult, interval_ms: f64, rm: &RadioModel) -> Option<f64> {
    let per_event_cap = l.packets_per_event_at(interval_ms);
    let notif_bytes = l.audio_bytes_per_frame as f64 / l.audio_notifications_per_frame.max(1) as f64;
    let ll_per_notif = ((notif_bytes + 7.0) / l.ll_payload_max as f64).ceil();
    let n = interval_ms / l.params.audio_frame_ms.max(1e-3) * l.audio_notifications_per_frame as f64 * ll_per_notif;
    if n > per_event_cap as f64 + 1e-9 {
        return None;
    }
    let e = event_energy_uj(l, n, l.data_packet_us, rm);
    Some(rm.supply_v * rm.sleep_ua / 1000.0 + e / interval_ms.max(7.5))
}

impl LinkResult {
    /// Packets per event at another interval (same phone limit and packet timing).
    pub fn packets_per_event_at(&self, interval_ms: f64) -> u32 {
        let t = ((interval_ms * 1000.0 - 1250.0).max(self.exchange_us) / self.exchange_us).floor() as u32;
        t.min(self.params.max_packets_per_event).max(1)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Sweep {
    pub interval_ms: Vec<f64>,
    pub throughput_kbps: Vec<f64>,
    pub idle_mw: Vec<f64>,
    /// `null` where the interval cannot carry the audio stream.
    pub stream_mw: Vec<Option<f64>>,
    pub caption_latency_ms: Vec<f64>,
    pub model: RadioModel,
    /// Longest interval (from the sweep) that still carries the audio stream.
    pub max_stream_interval_ms: f64,
    pub idle_fast_mw: f64,
    pub idle_slow_mw: f64,
    pub stream_fast_mw: Option<f64>,
}

/// Connection interval → throughput, radio power (idle link and audio streaming) and latency.
pub fn sweep(l: &LinkResult, rm: &RadioModel) -> Sweep {
    let grid = [
        7.5, 10.0, 12.5, 15.0, 20.0, 25.0, 30.0, 37.5, 45.0, 50.0, 60.0, 75.0, 100.0, 125.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0,
        750.0, 1000.0, 1500.0, 2000.0, 3000.0, 4000.0,
    ];
    let bytes_per_packet = l.att_data_per_notification as f64 / l.ll_packets_per_notification as f64;
    let mut sw = Sweep {
        interval_ms: grid.to_vec(),
        throughput_kbps: vec![],
        idle_mw: vec![],
        stream_mw: vec![],
        caption_latency_ms: vec![],
        model: rm.clone(),
        max_stream_interval_ms: 0.0,
        idle_fast_mw: idle_link_mw(l, l.params.conn_interval_ms, rm),
        idle_slow_mw: idle_link_mw(l, l.params.slow_interval_ms, rm),
        stream_fast_mw: stream_mw(l, l.params.conn_interval_ms, rm),
    };
    for &ci in &grid {
        sw.throughput_kbps
            .push(l.packets_per_event_at(ci) as f64 * bytes_per_packet * 8.0 / ci);
        sw.idle_mw.push(idle_link_mw(l, ci, rm));
        let st = stream_mw(l, ci, rm);
        if st.is_some() {
            sw.max_stream_interval_ms = ci;
        }
        sw.stream_mw.push(st);
        sw.caption_latency_ms.push(l.params.audio_frame_ms + ci + l.exchange_us / 1000.0);
    }
    sw
}

// ------------------------------------------------------------------ outputs

/// `iris_ble_protocol.h`: UUIDs and packed payload structs.
pub fn c_header(mtu: u32, base: &str) -> String {
    let services = gatt(mtu, base);
    let mut s = String::new();
    s += "/* iris_ble_protocol.h — Iris GATT protocol (generated by Protoforge).\n";
    s += " * The 128-bit UUIDs are PROJECT-SPECIFIC: a name-based (version 5) UUID of the project name, so the\n * firmware and the phone app derive the same values. They are not registered with the Bluetooth SIG.\n";
    s += " * All multi-byte fields are little-endian, structs are packed. */\n";
    s += "#ifndef IRIS_BLE_PROTOCOL_H\n#define IRIS_BLE_PROTOCOL_H\n#include <stdint.h>\n\n";
    s += "#if defined(__GNUC__) || defined(__clang__)\n#define IRIS_PACKED __attribute__((packed))\n#else\n#define IRIS_PACKED\n#endif\n\n";
    s += &format!("#define IRIS_BLE_MTU {mtu}\n#define IRIS_HUD_SERVICE_UUID \"{base}\"\n");
    let hud = &services[0];
    let bytes = |u: &str| uuid_le_bytes(u).iter().map(|b| format!("0x{b:02x}")).collect::<Vec<_>>().join(", ");
    s += &format!(
        "/* 128-bit UUIDs, little-endian byte order (as BLE stacks store them) */\n#define IRIS_UUID128_HUD_SERVICE {{{}}}\n",
        bytes(base)
    );
    for c in &hud.characteristics {
        s += &format!(
            "#define IRIS_UUID128_{} {{{}}} /* {} */\n",
            c.c_name.to_uppercase(),
            bytes(&c.uuid),
            c.uuid
        );
        s += &format!("#define IRIS_UUID_STR_{} \"{}\"\n", c.c_name.to_uppercase(), c.uuid);
    }
    s += "\n/* Bluetooth SIG assigned numbers */\n#define IRIS_UUID16_BATTERY_SERVICE 0x180F\n#define IRIS_UUID16_BATTERY_LEVEL 0x2A19\n#define IRIS_UUID16_DEVICE_INFORMATION 0x180A\n#define IRIS_UUID16_CCCD 0x2902\n\n";
    for c in hud.characteristics.iter().filter(|c| !c.fields.is_empty()) {
        s += &format!(
            "/* {} ({}) — {} */\ntypedef struct IRIS_PACKED {{\n",
            c.name,
            c.properties.join(", "),
            c.description
        );
        for fd in &c.fields {
            s += &format!("    {} {}; /* {} */\n", fd.ctype, fd.name, fd.desc);
        }
        if !c.trailing.is_empty() {
            s += &format!("    /* followed by: {} */\n", c.trailing);
        }
        s += &format!("}} iris_{}_hdr_t;\n", c.c_name);
        s += &format!(
            "typedef char iris_{0}_size_check[(sizeof(iris_{0}_hdr_t) == {1}) ? 1 : -1];\n\n",
            c.c_name, c.header_bytes
        );
    }
    s += "#endif /* IRIS_BLE_PROTOCOL_H */\n";
    s
}

pub fn markdown(mtu: u32, l: &LinkResult, base: &str) -> String {
    let mut s = String::from("# Iris BLE GATT specification\n\nGenerated by Protoforge's Prototype & Firmware Lab. Multi-byte fields are little-endian; payload headers are packed.\n\nThe custom 128-bit UUIDs are **project-specific** (a version-5, name-based UUID of the project name, RFC 4122); they are not registered with the Bluetooth SIG.\n\n");
    for sv in gatt(mtu, base) {
        s += &format!(
            "## {} — `{}`{}\n\n",
            sv.name,
            sv.uuid,
            if sv.standard { " (Bluetooth SIG)" } else { "" }
        );
        s += "| Characteristic | UUID | Properties | Direction | Payload |\n|---|---|---|---|---|\n";
        for c in &sv.characteristics {
            let payload = if c.fields.is_empty() {
                c.trailing.clone()
            } else {
                let mut p = c
                    .fields
                    .iter()
                    .map(|x| format!("{} {}", x.ctype, x.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !c.trailing.is_empty() {
                    p += &format!(" + {}", c.trailing);
                }
                format!("{} B header: {p}", c.header_bytes)
            };
            s += &format!(
                "| {} | `{}` | {} | {} | {} |\n",
                c.name,
                c.uuid,
                c.properties.join(", "),
                c.direction,
                payload
            );
        }
        s += "\n";
    }
    s += &format!(
        "## Link budget (typical — depends on the phone)\n\n- PHY {}, MTU {}, DLE {}, interval {:.1} ms, {} packets/event → **{:.0} kbit/s** application throughput\n- Audio uplink needs {:.0} kbit/s (margin ×{:.1})\n- Caption latency (audio frame + uplink + downlink): {:.0} ms; with an assumed {:.0} ms recogniser: {:.0} ms\n",
        l.params.phy,
        l.params.mtu,
        if l.params.dle { "on" } else { "off" },
        l.params.conn_interval_ms,
        l.packets_per_event,
        l.throughput_kbps,
        l.audio_demand_kbps,
        l.audio_margin,
        l.caption_link_latency_ms,
        l.params.asr_ms,
        l.caption_end_to_end_ms
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_time_matches_hand_calculation() {
        // 1M PHY, no DLE, unencrypted, MTU 23: 20 B data → 27 B LL payload in one packet
        let r = link(&LinkParams {
            phy: "1M".into(),
            dle: false,
            encrypted: false,
            mtu: 23,
            conn_interval_ms: 7.5,
            max_packets_per_event: 64,
            ..Default::default()
        });
        assert_eq!(r.att_data_per_notification, 20);
        assert_eq!(r.ll_packets_per_notification, 1);
        // (1+4+2+27+3)·8 = 296 µs; empty (1+4+2+3)·8 = 80 µs
        assert!((r.data_packet_us - 296.0).abs() < 1e-9);
        assert!((r.empty_packet_us - 80.0).abs() < 1e-9);
        assert!((r.exchange_us - 676.0).abs() < 1e-9);
        // (7500 − 1250) / 676 = 9.2 → 9 packets → 9·20 B per 7.5 ms = 192 kbit/s
        assert_eq!(r.packets_per_event, 9);
        assert!((r.throughput_kbps - 192.0).abs() < 1e-9);
        // 2M + DLE + MTU 247: 251-byte packets, (2+4+2+251+4+3)·8·0.5 = 1064 µs
        let r = link(&LinkParams::default());
        assert_eq!(r.ll_packets_per_notification, 1);
        assert!((r.data_packet_us - 1064.0).abs() < 1e-9);
        assert_eq!(r.packets_per_event, 6);
        assert!((r.throughput_kbps - 6.0 * 244.0 * 8.0 / 30.0).abs() < 1e-9);
        assert!(r.audio_margin > 3.0, "{}", r.audio_margin);
    }

    #[test]
    fn sha1_and_uuid_v5_match_published_vectors() {
        let hx = |d: [u8; 20]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        // FIPS 180 examples
        assert_eq!(hx(sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hx(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hx(sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        // Python's documented example: uuid.uuid5(uuid.NAMESPACE_DNS, 'python.org')
        assert_eq!(
            uuid_v5("6ba7b810-9dad-11d1-80b4-00c04fd430c8", "python.org"),
            "886313e1-3b8a-5372-9b90-0c9aee199e5d"
        );
    }

    #[test]
    fn uuid_bytes_are_little_endian_and_structs_sized() {
        let base = service_base("Iris");
        assert_eq!(base, service_base(" iris "), "deterministic and case-insensitive");
        assert_ne!(base, service_base("Other product"));
        assert_eq!(&base[4..8], "0000");
        assert_eq!(&base[14..15], "5", "version-5 nibble survives");
        let c3 = char_uuid(&base, 3);
        assert_eq!(&c3[4..8], "0003");
        assert_eq!(&c3[8..], &base[8..]);
        let b = uuid_le_bytes(&base);
        assert_eq!(b.len(), 16);
        assert_eq!(b[15], u8::from_str_radix(&base[0..2], 16).unwrap());
        assert_eq!(b[0], u8::from_str_radix(&base[34..36], 16).unwrap());
        let h = c_header(247, &base);
        assert!(h.contains("iris_nav_hdr_t"));
        assert!(h.contains("sizeof(iris_nav_hdr_t) == 11"));
        assert!(h.contains(&base));
        assert_eq!(h.matches('{').count(), h.matches('}').count());
        let g = gatt(247, &base);
        assert_eq!(g[1].uuid, "0x180F");
        assert_eq!(g[1].characteristics[0].uuid, "0x2A19");
        assert!(zephyr_encode_args(&base).starts_with(&format!("0x{}", &base[0..8])));
    }

    #[test]
    fn audio_budget_and_radio_power_by_hand() {
        let l = link(&LinkParams::default());
        // 16 kHz × 20 ms = 320 samples × 4 bit = 160 B + 6 B header, one notification per frame
        assert_eq!(l.audio_frame_samples, 320);
        assert_eq!(l.audio_frame_payload_bytes, 160);
        assert_eq!(l.audio_notifications_per_frame, 1);
        assert!((l.audio_demand_kbps - 166.0 * 8.0 / 20.0).abs() < 1e-9);
        assert!((l.compression_ratio - 4.0).abs() < 1e-12);
        assert!((l.audio_util_pct - 66.4 / l.throughput_kbps * 100.0).abs() < 1e-9);
        let rm = RadioModel::default();
        // idle event, 2M PHY: empty = (2+4+2+3)·8·0.5 = 44 µs (+4 B MIC? no: empty PDUs carry no MIC)
        assert!((l.empty_packet_us - 44.0).abs() < 1e-9);
        // E = 3·[4.6·(150 + 44 + 150) + 4.8·(44 + 150) + 3·200]/1000 µJ
        let e = 3.0 * (4.6 * 344.0 + 4.8 * 194.0 + 600.0) / 1000.0;
        assert!((event_energy_uj(&l, 1.0, l.empty_packet_us, &rm) - e).abs() < 1e-9);
        assert!((idle_link_mw(&l, 30.0, &rm) - (0.009 + e / 30.0)).abs() < 1e-9);
        // streaming at 30 ms: 1.5 frames per event → 1.5 packets
        let st = stream_mw(&l, 30.0, &rm).unwrap();
        let e_st = 3.0 * (4.6 * (150.0 + 1.5 * 194.0) + 4.8 * 1.5 * (l.data_packet_us + 150.0) + 600.0) / 1000.0;
        assert!((st - (0.009 + e_st / 30.0)).abs() < 1e-9);
        let sw = sweep(&l, &rm);
        assert!(sw.idle_mw.windows(2).all(|w| w[1] < w[0]), "longer interval, less idle power");
        // 6 packets/event × 1 frame/packet: the stream fits up to 6 × 20 = 120 ms
        assert!((sw.max_stream_interval_ms - 100.0).abs() < 1e-9, "{}", sw.max_stream_interval_ms);
        assert!(sw.stream_mw.last().unwrap().is_none());
    }
}
