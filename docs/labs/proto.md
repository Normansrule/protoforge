# Prototype & Firmware Lab

Route: `#/proto` · Engine: `crates/forge-core/src/firmware/` · Metrics: `firmware.ram_kb`,
`firmware.flash_kb`, `firmware.ram_margin_pct`, `firmware.flash_margin_pct`, `firmware.cpu_load_pct`,
`firmware.ble_throughput_kbps`, `firmware.ble_audio_util_pct`, `firmware.caption_latency_ms`,
`firmware.hud_frame_ms`

The software half of "one full prototype that can eventually become a product": build **Iris v0**
on a breadboard from development boards, generate its firmware from the other labs' designs,
simulate the heads-up display (HUD) pixel for pixel, and design the Bluetooth Low Energy (BLE)
protocol to the phone.

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Breadboard build | Pick one of three builds — XIAO nRF52840 Sense (primary, battery powered), nRF52840 DK (Zephyr bring-up on the final pin map) or ESP32-S3 (simulatable in Wokwi); watch the wiring diagram draw itself wire by wire; tick off the build checklist (click a step to highlight its wires; download it as Markdown); read the development-board bill of materials (BOM) with distributor search links (searches, not quotes); compare battery life with the custom board for any Power Lab scenario and cell size; read "what differs from the custom board". | Choosing dev boards that mirror the product; the path breadboard → DK → custom board. |
| Wiring | Light one bus at a time (power, ground, battery, I²C, SPI, PDM, I²S, GPIO) and watch animated signal flow along its wires; on-board traces (the XIAO's internal IMU I²C and PDM microphone) are drawn dashed; the pin map derived from the project's interfaces (IF-01 … IF-10) gives, per signal, the proposed custom-board pin and the pin on this build; wiring table; CSV downloads. | SPI vs I²C vs PDM, what each wire does, how an architecture's interfaces become pins. |
| Firmware generator | Read the "starting point" banner, choose a target (XIAO Arduino, ESP32-S3 + Wokwi, Zephyr for the custom board); browse every generated file; download the project as a zip; follow the "Open in Wokwi" steps with the generated `diagram.json`; see which lab each number came from, the learned power-policy lookup table, the build commands and an animated architecture diagram (interrupts, threads, queues; IMU → filter → HUD, mic → decimation → BLE, touch → FSM → UI, battery → policy → radio). | Code generation from models, fixed point, interrupt/thread split, RTOS structure. |
| HUD simulator | The engine renders the firmware's framebuffer (128×64 prototype OLED or the 640×400 micro-OLED, 1 or 4 bits per pixel), composited over a street scene at the Optics Lab's field of view; navigation distance counts down, captions scroll in word by word, the teleprompter scrolls; press-and-hold the touch pad (or use the tap / double tap / long press buttons) to drive the gesture state machine live; turn your head with the yaw slider to see the world-locked arrow; the orange box on the exact-pixels view marks each update's dirty rectangle. **Frame budget**: SPI clock, bits per pixel on the link, refresh rate and partial updates → full-frame time, maximum frame rate (plot vs SPI clock), per-screen dirty rectangles, SPI bytes/s, link duty and panel + link power; text sizes in arcminutes at the Optics Lab's angular resolution. | UI description vs renderer, integer rasterisation, display bandwidth, partial updates, OLED power vs lit pixels, angular text size, world- vs head-locked content. |
| BLE design | Sliders for connection interval, MTU, packets per event, peripheral latency, PHY, data length extension (DLE) and encryption; an animated timing diagram of two connection intervals and a zoom into one connection event; throughput, audio utilisation and caption latency; **connection-interval trade plots** (throughput, radio power for an idle link and while streaming audio, caption latency); the IMA ADPCM audio budget (PCM → 4:1 → headers → link capacity); the project-specific UUID card; the GATT table; an animated phone ↔ glasses sequence chart; downloads of the C header, Markdown and JSON specs. | GATT, air-time arithmetic, what really limits BLE throughput, latency budgets. |
| Resources | RAM and flash bars against the nRF52840's 256 KB / 1 MB with the recommended ceilings; what-if controls (bits per pixel, double buffering, over-the-air update slots, audio blocks, IMU batch, stack allowances); every line marked computed or assumption; **CPU load per task** (stacked bar + table, what-if on display refresh and connection interval); thread/interrupt table. | Static memory budgeting, update-slot flash budgeting, CPU budgeting from rates × cycles. |
| How it works | Explainers: embedded architecture, interrupts vs polling, RTOS threads, fixed point, BLE GATT basics, power states; references. | |

Downloads: BOM CSV, build checklist (Markdown), wiring and pin-map CSV, firmware zip per target, individual files, `iris_ble_protocol.h`,
GATT spec (Markdown, JSON), memory breakdown CSV.

## The breadboard builds (`boards.rs`)

* **XIAO build** — Seeed Studio XIAO nRF52840 Sense (same BLE system-on-chip family as Iris,
  on-board LSM6DS3TR-C IMU on an internal I²C bus, PDM microphone, LiPo charger) + a 7-pin
  SSD1306 128×64 **SPI** OLED (Iris drives its micro-OLED over SPI too, IF-01): SCK D8, MOSI D10,
  CS D7, DC D2, RES D3 + TTP223 touch module on D0 + LiPo on the BAT+/BAT− pads. Pin names are
  those of Seeed's published Arduino variant (checked against `variants/Seeed_XIAO_nRF52840_Sense/variant.h`:
  SPI on D8/D9/D10, `PIN_WIRE1_*` for the IMU, `PIN_PDM_*`, `PIN_VBAT`, `VBAT_ENABLE`).
* **nRF52840 DK build** — Nordic's development kit with breakouts wired to exactly the pins in
  `targets::ZEPHYR_PINS` (the generated devicetree overlay): SPI OLED on P1.15/P1.13/P1.12/P1.11/P1.10,
  PDM microphone breakout on P1.01/P1.02, BMI270 IMU breakout on P0.26/P0.27 (+ INT P0.28), TTP223
  on P0.29, and a 10 kΩ potentiometer on AIN0 (P0.02) that simulates the battery-sense voltage so
  the learned power policy can be walked through its buckets. Build the Zephyr skeleton with
  `-DIRIS_PANEL_SSD1306=ON` to drive the stand-in OLED (page conversion + the standard SSD1306
  init sequence); the same code drives the micro-OLED on the custom board.
* **ESP32-S3 build** — ESP32-S3-DevKitC-1 + MPU-6050-class IMU and SSD1306 on I²C (GPIO 8/9),
  INMP441-class I²S microphone (GPIO 4/5/6), TTP223 or a push button on GPIO 7. Pin names are
  those of Wokwi's DevKitC-1 board definition (`wokwi-boards/boards/esp32-s3-devkitc-1/board.json`).
  The display is I²C here because Wokwi's SSD1306 part is I²C-only; Wokwi simulates neither
  Bluetooth nor I²S on the ESP32-S3 (its ESP32 guide lists the peripherals).
* The wiring model is parts (outlines + pin coordinates; `onboard` chips drawn inside a board)
  and wires (pin → pin, colour, **bus**, signal, on-board flag, orthogonal route). Tests: every
  route starts and ends on its pins with only horizontal/vertical segments and stays on the
  canvas; step highlights reference real wires; the DK wiring equals the overlay pins.
* **Pin map** (`boards::pinmap`) — iterates `Project::interfaces`: SPI to the optics subsystem →
  SCK/MOSI/CS/DC/RST; SPI to the co-processor → "not prototyped" (the SoC's hardware PDM
  decimator replaces it); PDM → CLK/DATA (I²S on the ESP32 build); I²C → SDA/SCL/INT/touch;
  POWER to the SoC → VBAT sense + supply; POWER to the display → DISP_EN; BLE → antenna.
  Mechanical and thermal interfaces are skipped. A test checks that every electrical interface
  of Iris is mapped.
* **BOM search links** are Octopart and Digi-Key keyword searches (percent-encoded), never prices.
* **Power comparison** — the prototype's modules become a project clone and both go through
  `power::budget`: `P_avg = Σ qty·[P_a·d + P_s·(1 − d)] / η`, `life = C·V_nom·usable / P_avg`.
  On-board silicon uses the Iris BOM figures scaled by V_rail / 1.8 V (3.3 V XIAO, 3.0 V DK);
  the OLED (40 mW typical), TTP223, ESP32-S3, MPU-6050, INMP441 and board overheads are
  **stated assumptions**; the DK's potentiometer is computed (3.0² / 10 kΩ = 0.9 mW — the reason
  Iris's real divider is 2.5 MΩ, R-014). The nRF breadboards draw *less* than the custom board
  only because a 128×64 OLED replaces the 640×400 micro-OLED — reported, not tuned.

## Generated firmware (`modules.rs`, `hud.rs`, `adpcm.rs`, `targets.rs`)

| File | From | Content |
|---|---|---|
| `iris_config.h` | project + HUD budget | IMU rate, fusion α (with its crossover frequency), audio rate, gesture thresholds, panel size, display bus and clock, frame bytes and wire time (`IRIS_DISP_FRAME_US`), connection intervals, policy step |
| `iris_filters.h` | DSP Lab | PDM decimation FIR (`dsp::pdm::design_chain`) as `int16_t` Q15; voice high-pass (`dsp::iir::butterworth`, 2nd order at `voice_band_lo_hz`) as Q14 |
| `iris_dsp.{h,c}` | — | Q14 direct-form-I biquad with 64-bit accumulator, Q15 FIR, integer RMS, IMA ADPCM encoder |
| `iris_gesture.{h,c}` | Chip Lab | the gesture Moore machine (`chip::fsm::gesture_fsm`) as a `switch`, first-matching-guard semantics, saturating timer |
| `iris_policy.{h,c}` | RL Lab + Power Lab | `IRIS_POLICY[buckets][4]` from `rl::power::train` (same config as its metric, memoised), OCV table from `power::battery::ocv` |
| `iris_fusion.{h,c}` | Control Lab | complementary filter with `design.imu.fusion_alpha` |
| `iris_gfx.{h,c}`, `iris_hud.{h,c}` | this lab | integer rasteriser + the HUD widget tables and interpreter |
| `iris_ble_protocol.h` | this lab | project-specific UUIDs (little-endian byte arrays and strings), packed payload structs with compile-time size checks |

Targets:

* `xiao_arduino` — `platformio.ini` (Seeed's platform, board `seeed-xiao-afruitnrf52-nrf52840-sense`,
  checked to exist in `Seeed-Studio/platform-seeedboards`), sketch folder `iris_v0_xiao/` (opens in
  the Arduino IDE too), Adafruit Bluefruit BLE API, `PDM.h`, Seeed LSM6DS3 library, U8g2
  `U8G2_SSD1306_128X64_NONAME_F_4W_HW_SPI` (`drawXBM` of the generated framebuffer).
* `esp32s3_wokwi` — single-file `src/sketch.ino` (all modules amalgamated), `diagram.json`
  (`board-esp32-s3-devkitc-1`, `board-ssd1306`, `wokwi-mpu6050`, `wokwi-pushbutton`, pin names
  checked against the wokwi-docs / wokwi-boards repositories), `libraries.txt`, `wokwi.toml`,
  `platformio.ini`. Wokwi simulates neither Bluetooth nor the S3's I²S, so both are compiled out
  by default and a scripted demo runs.
* `zephyr` — `CMakeLists.txt`, `prj.conf`, `boards/nrf52840dk_nrf52840.overlay` (proposed Iris
  pin map on an nRF52840 DK: TWIM, SPIM3, PDM, SAADC, `zephyr,user` GPIOs), `src/main.c` with a
  1 kHz timer interrupt for touch, sensor / audio / UI / power threads, a message queue and a
  mutex, GATT service definition (UUIDs via `BT_UUID_128_ENCODE`) and connection-parameter
  updates; `CMakeLists.txt` option `IRIS_PANEL_SSD1306` for the DK breadboard. Clearly marked as a
  skeleton (micro-OLED init sequence, sensor attributes, pairing are TODOs).

**Generated code is a starting point to compile and test on hardware** (the page says so in a
banner). Nothing can be compiled in this environment, so the generator only uses APIs documented by
each library and the tests check structure: balanced brackets in every C/C++/devicetree file
(ignoring comments and literals), expected symbols per target, amalgamation without local
includes, the exact Wokwi part list.

## HUD rendering (`gfx.rs`, `font.rs`, `hud.rs`)

* A screen is a table of widgets on a 128 × 64 design grid: TEXT (field or literal, aligned,
  clipped), WRAP (greedy word wrap, top- or bottom-anchored with pixel scroll), RECT, FILL,
  HLINE, ARROW (7-point polygon rotated by the nav bearing), BAR, BATT. Screens: clock, navigation,
  captions, notification, teleprompter.
* Mapping: `x_px = u·W/128`, `y_px = v·H/64`, stroke scale `fs = max(1, min(⌊W/128⌋, ⌊H/64⌋))`
  (1 on the prototype OLED, 5 on 640 × 400).
* Integer-only primitives shared with C: X11 misc-fixed 5×7 font (public domain), Bresenham
  lines, midpoint circles, even–odd scanline fill sampled at pixel centres, rotation with a Q14
  sine table and symmetric rounding. Framebuffers: 1 bpp XBM order (U8g2 `drawXBM`), 4 bpp
  high-nibble-first.
* Tests: font rows, XBM bit order, polygon area (rectangle exact, triangle ≈ shoelace), Bresenham
  pixel count, deterministic hashes for every screen at 128×64 and 640×400 (1 and 4 bpp), wrap
  cases, field formatting and the C tables.

## HUD frame budget (`display.rs`)

* Full frame: `bytes = W·H·b/8`, `t = 8·bytes / f_SCK + t_overhead`. Iris: 640 × 400 × 1 bit =
  32 000 B → 32.05 ms at 8 MHz (+50 µs) → at most 31 frames/s (metric `firmware.hud_frame_ms`).
  The nRF52840's SPIM3 goes to 32 MHz (other SPIM instances 8 MHz). Test: hand values for 1 bpp
  at 8 MHz and 8 bpp at 32 MHz (64 ms).
* Partial updates: each screen is rendered twice with the firmware renderer (clock +1 minute,
  navigation −1 m, captions +1 word, teleprompter +1 scroll step, notification unchanged) and the
  bounding box of changed pixels is the dirty rectangle sent through the controller's address
  window. SPI bytes/s: `frame_bytes · fps` (naive full refresh) vs `dirty_bytes · update rate`.
* Panel power: `P = P_static + P_emit · APL`, APL = lit fraction measured from the framebuffer;
  calibrated so the BOM's 140 mW corresponds to 25 % lit with 30 % static (assumptions, sliders).
  Link power: SoC active power while the transfer runs (upper bound — EasyDMA lets the CPU sleep).
* Text size: cap height = 6 font rows × font scale × size px; angular size = px × arcmin/px from
  `optics::paraxial::analyze` (≈ 2.24′/px for Iris → 67′ body text, "large" by the Optics Lab's
  legibility scale; a 20/20 letter is 5′).

## BLE design (`ble.rs`)

* Custom HUD service: a **project-specific, deterministic** 128-bit UUID — version 5 (SHA-1,
  RFC 4122 URL name space) of `protoforge:<project name>:hud-service`, hex digits 5–8 cleared so
  characteristics are numbered 0001 … 0007. Same project → same UUIDs in the firmware and a phone
  app; not registered with the Bluetooth SIG. Tests: FIPS 180 SHA-1 vectors and Python's documented
  `uuid5(NAMESPACE_DNS, 'python.org')`. Characteristics: Notification, Navigation, Captions (phone → glasses), Microphone audio,
  Gesture events, Status (glasses → phone) and Display control; Battery Service 0x180F (Battery
  Level 0x2A19) and Device Information 0x180A (0x2A29, 0x2A24, 0x2A26, 0x2A27).
* Air time: `(preamble 1|2 + AA 4 + header 2 + payload + MIC 4 if encrypted + CRC 3) × 8 × t_bit`,
  exchange `T_data + 150 µs + T_empty + 150 µs`, packets per event `min(⌊(T_ci − 1.25 ms)/exchange⌋,
  phone limit)`, throughput `n · (MTU − 3) · 8 / T_ci`. Hand-checked test: 1M PHY, 27-byte
  payloads, MTU 23, 7.5 ms → 296 µs + 80 µs empty, 9 packets, 192 kbit/s.
* Audio uplink: IMA ADPCM (4:1), 16 kHz × 4 bit = 64 kbit/s; one 20 ms frame = 320 samples =
  160 B + 6-byte header per notification → 66.4 kbit/s, 17 % of the 390 kbit/s fast link
  (`firmware.ble_audio_util_pct`). Test: 440 Hz tone round trip SNR > 20 dB, encoder state equals
  decoder state, the 166-byte frame by hand.
* Radio power vs connection interval: `E_event = V·[I_rx·(t_wake + n·(t_poll + 150 µs)) +
  I_tx·n·(t_packet + 150 µs) + I_cpu·t_stack]`, `P = V·I_sleep + E_event / T_ci`, with typical
  nRF52840 currents (TX 4.8 mA at 0 dBm, RX 4.6 mA, DC/DC, 3 V; check the current Product
  Specification) and assumed 150 µs wake-up / 200 µs stack time per event. Idle link: 320 µW at
  30 ms, 40 µW at 300 ms; streaming audio 1.15 mW at 30 ms; the stream fits up to a 100 ms interval
  with 6 packets per event. Test: hand calculation of one event and the streaming limit.
* Caption latency (metric) = audio frame + T_ci/2 + air (up) + T_ci/2 + air (down); the phone's
  speech recogniser time is a separate, stated assumption (default 300 ms).

## Resources (`resources.rs`)

RAM = framebuffer (W·H·bpp/8, ×2 if double-buffered) + DMIC blocks (blocks × f_s/50 × 2 B × mics)
+ ADPCM packet + IMU batch (rate × batch × 12 B) + HUD state + event queue + thread stacks + heap
+ **assumptions** for the Bluetooth stack (24 KB) and kernel (6 KB). Flash = application code
(C statements in the generated sources × 10 B, a heuristic) + exact constant tables + **assumptions**
for the Bluetooth stack (150 KB), kernel (40 KB) and log strings (8 KB). With over-the-air updates
the image budget is (1024 − 48 KB bootloader − 32 KB storage) / 2 = 472 KB. Replace the
assumptions with `west build -t ram_report` / `rom_report` once the skeleton builds.

CPU load (busiest context, live captions) = Σ rate × cycles / 64 MHz: gesture ISR 1 kHz × 150,
biquad 16 k × 30, ADPCM 16 k × 40, IMU + filter 200 Hz × 2400, HUD render 10 fps × (clear +
12 cycles per lit pixel of the real captions frame), SPI set-up, BLE events and notifications,
policy. PDM decimation is 0 on the CPU (hardware PDM peripheral). Cycle counts are estimates to
check with the DWT cycle counter; Iris ≈ 8 %.

## Iris values

| Metric | Iris value (v0.6) | Requirement (R-id when Iris carries it) |
|---|---|---|
| `firmware.ram_kb` | 92.8 KB | suggested ≤ 192 KB (75 % of 256 KB) |
| `firmware.ram_margin_pct` | 63.7 % | **R-031** ≥ 25 % ✓ |
| `firmware.flash_kb` | 206 KB | suggested ≤ 472 KB (fits one over-the-air update slot) |
| `firmware.flash_margin_pct` | 56.4 % | **R-032** ≥ 20 % ✓ |
| `firmware.cpu_load_pct` | 8.1 % | suggested ≤ 50 % in the busiest context |
| `firmware.ble_throughput_kbps` | 390 kbit/s | suggested ≥ 128 kbit/s (2× the ADPCM audio uplink) |
| `firmware.ble_audio_util_pct` | 17.0 % | suggested ≤ 50 % |
| `firmware.caption_latency_ms` | 52.8 ms | **R-033** ≤ 100 ms link latency ✓ (≤ 500 ms speech → caption with the recogniser, suggested) |
| `firmware.hud_frame_ms` | 32.05 ms | suggested ≤ 100 ms (≥ 10 full frames/s) |

## References

Nordic Semiconductor, *nRF52840 Product Specification* (docs.nordicsemi.com) · Bluetooth SIG,
*Bluetooth Core Specification* v5.x and Assigned Numbers (bluetooth.com/specifications/assigned-numbers)
· Zephyr Project documentation (docs.zephyrproject.org) · Nordic Semiconductor, *nRF52840 DK
hardware user guide* · Seeed Studio wiki, XIAO nRF52840 Sense (wiki.seeedstudio.com) · Wokwi
documentation (docs.wokwi.com) · Espressif Systems, *ESP32-S3 Series Datasheet* · Arduino
documentation (docs.arduino.cc) · P. Leach, M. Mealling, R. Salz, RFC 4122 (UUIDs), IETF 2005 ·
NIST FIPS 180-4, *Secure Hash Standard* · Solomon Systech, *SSD1306 datasheet* · PlatformIO documentation
(docs.platformio.org) · U8g2 (github.com/olikraus/u8g2), Adafruit GFX Library
(github.com/adafruit/Adafruit-GFX-Library) · E. White, *Making Embedded Systems*, 2nd ed.,
O'Reilly 2024 · J. J. Labrosse, *MicroC/OS-II: The Real-Time Kernel*, 2nd ed., CMP Books 2002 ·
R. G. Lyons, *Understanding Digital Signal Processing*, 3rd ed., Prentice Hall 2010 · C. Gomez,
J. Oller, J. Paradells, "Overview and evaluation of Bluetooth Low Energy", *Sensors* 12(9), 2012 ·
IMA, "Recommended Practices for Enhancing Digital Audio Compatibility in Multimedia Systems",
rev. 3.00, 1992 · J. E. Bresenham, *IBM Systems Journal* 4(1), 1965 · open glasses references:
Brilliant Labs (github.com/brilliantlabsAR), OpenGlass (github.com/BasedHardware/OpenGlass).
