# Prototype & Firmware Lab

Route: `#/proto` · Engine: `crates/forge-core/src/firmware/` · Metrics: `firmware.ram_kb`,
`firmware.flash_kb`, `firmware.ble_throughput_kbps`, `firmware.caption_latency_ms`

The software half of "one full prototype that can eventually become a product": build **Iris v0**
on a breadboard from development boards, generate its firmware from the other labs' designs,
simulate the heads-up display (HUD) pixel for pixel, and design the Bluetooth Low Energy (BLE)
protocol to the phone.

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Breadboard build | Pick the XIAO nRF52840 Sense build (primary) or the ESP32-S3 build (Wokwi-simulatable); watch the wiring diagram draw itself wire by wire (hover a wire to light both ends); tick off the assembly steps (click one to highlight its wires); read the development-board bill of materials (BOM), the wiring table and "what differs from the custom board"; compare battery life with the custom board for any Power Lab scenario and cell size. | Choosing dev boards that mirror the product, I²C wiring and colour codes, why a prototype draws more power than the product. |
| Firmware | Choose a target (XIAO Arduino, ESP32-S3 + Wokwi, Zephyr for the custom board); browse every generated file; download the project as a zip; follow the "Open in Wokwi" steps with the generated `diagram.json`; see which lab each number came from, the learned power-policy lookup table, the build commands and an animated architecture diagram (interrupts, threads, queues; IMU → filter → HUD, mic → decimation → BLE, touch → FSM → UI, battery → policy → radio). | Code generation from models, fixed point, interrupt/thread split, RTOS structure. |
| HUD simulator | The engine renders the firmware's framebuffer (128×64 prototype OLED or the 640×400 micro-OLED, 1 or 4 bits per pixel), composited over a street scene at the Optics Lab's field of view; navigation distance counts down, captions scroll in word by word, the teleprompter scrolls; press-and-hold the touch pad (or use the tap / double tap / long press buttons) to drive the gesture state machine live; turn your head with the yaw slider to see the world-locked arrow. | UI description vs renderer, integer rasterisation, angular text size, world- vs head-locked content. |
| Bluetooth | Sliders for connection interval, MTU, packets per event, peripheral latency, PHY, data length extension (DLE) and encryption; an animated timing diagram of two connection intervals and a zoom into one connection event; throughput, audio margin and caption latency; the GATT table; an animated phone ↔ glasses sequence chart; downloads of the C header, Markdown and JSON specs. | GATT, air-time arithmetic, what really limits BLE throughput, latency budgets. |
| Resources | RAM and flash bars against the nRF52840's 256 KB / 1 MB with the recommended ceilings; what-if controls (bits per pixel, double buffering, over-the-air update slots, audio blocks, IMU batch, stack allowances); every line marked computed or assumption; thread/interrupt table. | Static memory budgeting on microcontrollers, update-slot flash budgeting. |
| How it works | Explainers: embedded architecture, interrupts vs polling, RTOS threads, fixed point, BLE GATT basics, power states; references. | |

Downloads: BOM and wiring CSV, firmware zip per target, individual files, `iris_ble_protocol.h`,
GATT spec (Markdown, JSON), memory breakdown CSV.

## The breadboard builds (`boards.rs`)

* **XIAO build** — Seeed Studio XIAO nRF52840 Sense (same BLE system-on-chip family as Iris,
  on-board 6-axis IMU, PDM microphone and LiPo charger) + 0.96 in SSD1306 (or 1.3 in SH1106)
  I²C OLED on D4/D5 + TTP223 touch module on D1 + LiPo on the BAT+/BAT− pads. Pin names are
  those of Seeed's published board variant (`D0`–`D10`, `PIN_VBAT`, `VBAT_ENABLE`); the
  on-board IMU sits on a separate internal I²C bus that the Seeed LSM6DS3 library selects.
* **ESP32-S3 build** — ESP32-S3-DevKitC-1 + MPU-6050-class IMU and SSD1306 on I²C (GPIO 8/9),
  INMP441-class I²S microphone (GPIO 4/5/6), TTP223 or a push button on GPIO 7. Pin names are
  those of Wokwi's DevKitC-1 board definition.
* The wiring model is parts (outlines + pin coordinates) and wires (pin → pin, colour, signal,
  orthogonal route). Tests check every route starts and ends on its pins and has only
  horizontal/vertical segments.
* **Power comparison** — the prototype's modules become a project clone (same subsystems, so
  the Power Lab's scenarios apply) and both go through `power::budget`:
  `P_avg = Σ qty·[P_a·d + P_s·(1 − d)] / η`, `life = C·V_nom·usable / P_avg`. The on-board
  silicon uses the Iris BOM figures scaled by 3.3 V / 1.8 V (same current, higher rail); the
  OLED (40 mW typical at ~25 % pixels lit), TTP223, ESP32-S3, MPU-6050, INMP441 and board
  overheads are **stated assumptions** to replace with measurements. The regulator efficiency
  is that of a linear regulator, 3.3 V / V_nom.

## Generated firmware (`modules.rs`, `hud.rs`, `adpcm.rs`, `targets.rs`)

| File | From | Content |
|---|---|---|
| `iris_config.h` | project | IMU rate, fusion α (with its crossover frequency), audio rate, gesture thresholds, panel size, connection intervals, policy step |
| `iris_filters.h` | DSP Lab | PDM decimation FIR (`dsp::pdm::design_chain`) as `int16_t` Q15; voice high-pass (`dsp::iir::butterworth`, 2nd order at `voice_band_lo_hz`) as Q14 |
| `iris_dsp.{h,c}` | — | Q14 direct-form-I biquad with 64-bit accumulator, Q15 FIR, integer RMS, IMA ADPCM encoder |
| `iris_gesture.{h,c}` | Chip Lab | the gesture Moore machine (`chip::fsm::gesture_fsm`) as a `switch`, first-matching-guard semantics, saturating timer |
| `iris_policy.{h,c}` | RL Lab + Power Lab | `IRIS_POLICY[buckets][4]` from `rl::power::train` (same config as its metric, memoised), OCV table from `power::battery::ocv` |
| `iris_fusion.{h,c}` | Control Lab | complementary filter with `design.imu.fusion_alpha` |
| `iris_gfx.{h,c}`, `iris_hud.{h,c}` | this lab | integer rasteriser + the HUD widget tables and interpreter |
| `iris_ble_protocol.h` | this lab | UUIDs (little-endian byte arrays), packed payload structs with compile-time size checks |

Targets:

* `xiao_arduino` — `platformio.ini` (Seeed's platform, board `seeed-xiao-afruitnrf52-nrf52840-sense`),
  sketch folder `iris_v0_xiao/` (opens in the Arduino IDE too), Adafruit Bluefruit BLE API,
  `PDM.h`, Seeed LSM6DS3 library, U8g2 (`drawXBM` of the generated framebuffer).
* `esp32s3_wokwi` — single-file `src/sketch.ino` (all modules amalgamated), `diagram.json`
  (`board-esp32-s3-devkitc-1`, `board-ssd1306`, `wokwi-mpu6050`, `wokwi-pushbutton`, pin names
  checked against the wokwi-docs / wokwi-boards repositories), `libraries.txt`, `wokwi.toml`,
  `platformio.ini`. Wokwi simulates neither Bluetooth nor the S3's I²S, so both are compiled out
  by default and a scripted demo runs.
* `zephyr` — `CMakeLists.txt`, `prj.conf`, `boards/nrf52840dk_nrf52840.overlay` (proposed Iris
  pin map on an nRF52840 DK: TWIM, SPIM3, PDM, SAADC, `zephyr,user` GPIOs), `src/main.c` with a
  1 kHz timer interrupt for touch, sensor / audio / UI / power threads, a message queue and a
  mutex, GATT service definition and connection-parameter updates. Clearly marked as a skeleton
  (display init sequence, sensor attributes, pairing are TODOs).

Nothing can be compiled in this environment, so the generator only uses APIs documented by
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

## BLE design (`ble.rs`)

* Custom HUD service `a1f40000-6d2b-4c9e-8f3a-5b7e2c1d0e9f` (generated for this example —
  make your own) with Notification, Navigation, Captions (phone → glasses), Microphone audio,
  Gesture events, Status (glasses → phone) and Display control; Battery Service 0x180F (Battery
  Level 0x2A19) and Device Information 0x180A (0x2A29, 0x2A24, 0x2A26, 0x2A27).
* Air time: `(preamble 1|2 + AA 4 + header 2 + payload + MIC 4 if encrypted + CRC 3) × 8 × t_bit`,
  exchange `T_data + 150 µs + T_empty + 150 µs`, packets per event `min(⌊(T_ci − 1.25 ms)/exchange⌋,
  phone limit)`, throughput `n · (MTU − 3) · 8 / T_ci`. Hand-checked test: 1M PHY, 27-byte
  payloads, MTU 23, 7.5 ms → 296 µs + 80 µs empty, 9 packets, 192 kbit/s.
* Audio uplink: IMA ADPCM, 16 kHz × 4 bit = 64 kbit/s plus a 6-byte block header. Test: 440 Hz
  tone round trip SNR > 20 dB, encoder state equals decoder state.
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

## Iris values

| Metric | Value | Suggested requirement |
|---|---|---|
| `firmware.ram_kb` | IRIS_RAM KB | ≤ 192 KB (75 % of 256 KB) |
| `firmware.flash_kb` | IRIS_FLASH KB | ≤ 472 KB (fits one over-the-air update slot) |
| `firmware.ble_throughput_kbps` | IRIS_THR kbit/s | ≥ 128 kbit/s (2× the ADPCM audio uplink) |
| `firmware.caption_latency_ms` | IRIS_LAT ms | ≤ 100 ms link latency (≤ 500 ms speech → caption with the recogniser) |

## References

Nordic Semiconductor, *nRF52840 Product Specification* (docs.nordicsemi.com) · Bluetooth SIG,
*Bluetooth Core Specification* v5.x and Assigned Numbers (bluetooth.com/specifications/assigned-numbers)
· Zephyr Project documentation (docs.zephyrproject.org) · Seeed Studio wiki, XIAO nRF52840 Sense
(wiki.seeedstudio.com) · Wokwi documentation (docs.wokwi.com) · PlatformIO documentation
(docs.platformio.org) · U8g2 (github.com/olikraus/u8g2), Adafruit GFX Library
(github.com/adafruit/Adafruit-GFX-Library) · E. White, *Making Embedded Systems*, 2nd ed.,
O'Reilly 2024 · J. J. Labrosse, *MicroC/OS-II: The Real-Time Kernel*, 2nd ed., CMP Books 2002 ·
R. G. Lyons, *Understanding Digital Signal Processing*, 3rd ed., Prentice Hall 2010 · C. Gomez,
J. Oller, J. Paradells, "Overview and evaluation of Bluetooth Low Energy", *Sensors* 12(9), 2012 ·
IMA, "Recommended Practices for Enhancing Digital Audio Compatibility in Multimedia Systems",
rev. 3.00, 1992 · J. E. Bresenham, *IBM Systems Journal* 4(1), 1965 · open glasses references:
Brilliant Labs (github.com/brilliantlabsAR), OpenGlass (github.com/BasedHardware/OpenGlass).
