// Prototype & Firmware Lab — turn Iris into a working breadboard prototype,
// generate its firmware, simulate its heads-up display and design its
// Bluetooth protocol. Everything is computed by the Rust engine (firmware.*).
import { LabPage } from '../../components/LabPage';
import { Card, Explainer, Formula, Stat, Tabs } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useEvaluation } from '../../state/project';
import { BleTab } from './BleTab';
import { BuildTab } from './BuildTab';
import { FirmwareTab } from './FirmwareTab';
import { HudTab } from './HudTab';
import { ResourcesTab } from './ResourcesTab';
import { WiringTab } from './Wiring';
import { usePersistent } from './util';
import './proto.css';

type Tab = 'build' | 'wiring' | 'firmware' | 'hud' | 'ble' | 'resources' | 'learn';

const REFS = {
  nordic: { label: 'Nordic Semiconductor, nRF52840 Product Specification', href: 'https://docs.nordicsemi.com' },
  core: { label: 'Bluetooth SIG, Bluetooth Core Specification v5.x' },
  assigned: { label: 'Bluetooth SIG Assigned Numbers', href: 'https://www.bluetooth.com/specifications/assigned-numbers/' },
  zephyr: { label: 'Zephyr Project documentation', href: 'https://docs.zephyrproject.org' },
  seeed: { label: 'Seeed Studio wiki: XIAO nRF52840 Sense', href: 'https://wiki.seeedstudio.com' },
  wokwi: { label: 'Wokwi documentation', href: 'https://docs.wokwi.com' },
  pio: { label: 'PlatformIO documentation', href: 'https://docs.platformio.org' },
  u8g2: { label: 'U8g2 graphics library (olikraus/u8g2)', href: 'https://github.com/olikraus/u8g2' },
  gfx: { label: 'Adafruit GFX Library', href: 'https://github.com/adafruit/Adafruit-GFX-Library' },
  white: { label: 'E. White, Making Embedded Systems, 2nd ed., O\'Reilly 2024' },
  esp: { label: 'Espressif Systems, ESP32-S3 Series Datasheet', href: 'https://www.espressif.com/en/support/documents/technical-documents' },
  ima: { label: 'Interactive Multimedia Association, Recommended Practices for Enhancing Digital Audio Compatibility in Multimedia Systems, rev. 3.00, 1992 (IMA ADPCM)' },
  arduino: { label: 'Arduino documentation', href: 'https://docs.arduino.cc' },
  rfc4122: { label: 'P. Leach, M. Mealling, R. Salz, RFC 4122: A Universally Unique IDentifier (UUID) URN Namespace, IETF 2005', href: 'https://www.rfc-editor.org/rfc/rfc4122' },
  nordicdk: { label: 'Nordic Semiconductor, nRF52840 DK hardware user guide', href: 'https://docs.nordicsemi.com' },
  brilliant: { label: 'Brilliant Labs open repositories (brilliantlabsAR)', href: 'https://github.com/brilliantlabsAR' },
  openglass: { label: 'BasedHardware/OpenGlass', href: 'https://github.com/BasedHardware/OpenGlass' },
  labrosse: { label: 'J. J. Labrosse, MicroC/OS-II: The Real-Time Kernel, 2nd ed., CMP Books 2002' },
  lyons: { label: 'R. G. Lyons, Understanding Digital Signal Processing, 3rd ed., Prentice Hall 2010' },
  gomez: { label: 'C. Gomez, J. Oller, J. Paradells, "Overview and evaluation of Bluetooth Low Energy", Sensors 12(9), 2012' },
  benini: { label: 'L. Benini, A. Bogliolo, G. De Micheli, "A survey of design techniques for system-level dynamic power management", IEEE Trans. VLSI Systems 8(3), 2000' },
};

function LearnTab() {
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Explainer title="Embedded architecture: from breadboard to product" refs={[REFS.white, REFS.seeed, REFS.nordic]}>
          Iris's firmware is organised the way most battery-powered gadgets are: <b>drivers</b> talk to hardware (I²C sensors, the PDM microphone peripheral, the SPI display, the radio), <b>services</b> turn raw data into meaning (the complementary filter, the gesture state machine, the audio encoder, the power policy), and the <b>application</b> decides what to show. Keeping those layers separate is what lets the same generated modules run on a XIAO under Arduino, an ESP32-S3 in a simulator and the custom board under Zephyr — only the drivers change.
        </Explainer>
        <Explainer title="Interrupts vs polling" refs={[REFS.white, REFS.nordic]}>
          Polling asks a peripheral "anything new?" in a loop — simple, but it burns power and misses short events. An <b>interrupt</b> makes the hardware call a short handler the moment something happens. The rule: do the least possible in the handler (copy a sample, step a state machine, post an event) and let a thread do the rest. Iris samples the touch pad in a 1 kHz timer interrupt and lets the microphone's EasyDMA fill whole 20 ms blocks without the CPU at all.
          <Formula>{'CPU load from an ISR = f_irq × t_handler   (1 kHz × 5 µs = 0.5 %)'}</Formula>
        </Explainer>
        <Explainer title="Real-time operating system (RTOS) threads" refs={[REFS.zephyr, REFS.labrosse]}>
          A preemptive RTOS runs the highest-priority thread that is ready. Audio gets the highest thread priority because a missed block is an audible glitch; the IMU next (a late sample adds filter error); the display after that (a late frame is barely noticed); the power manager last. Threads block on queues and semaphores instead of spinning, so when nothing is ready the idle thread puts the chip to sleep. Shared data (the HUD state) is guarded by a mutex; events travel through a message queue.
        </Explainer>
        <Explainer title="Fixed-point arithmetic" refs={[REFS.lyons]}>
          The Cortex-M4F has a single-precision floating-point unit, but fixed point is still how filters ship on small chips: deterministic, fast, and identical on every compiler. Q15 stores x ∈ [−1, 1) as round(x·2¹⁵); a product of two Q15 numbers is Q30, shifted back by 15 with rounding and saturation. The generator converts the DSP Lab's decimation filter to Q15 and its voice high-pass to Q14, and the tests check the integer filter against the floating-point design.
          <Formula>{'Q15: x̂ = clamp(round(x·32768), −32768, 32767)     error ≤ 2⁻¹⁶'}</Formula>
        </Explainer>
        <Explainer title="Bluetooth Low Energy GATT basics" refs={[REFS.core, REFS.assigned, REFS.gomez]}>
          The phone (central) connects to the glasses (peripheral) and they meet at <b>connection events</b> every interval (7.5 ms to 4 s). In between, both radios sleep — that is where the "low energy" comes from. Data is organised as services and characteristics; the phone writes navigation, captions and notifications, the glasses notify audio, gestures and status. Throughput is packets per event × payload / interval, and it is the phone that has the final say on both.
        </Explainer>
        <Explainer title="Power states and a learned policy" refs={[REFS.benini, REFS.nordic]}>
          Every block has an active and a sleep power; battery life is set by how much time each spends in which state. Iris has two big knobs — display mode (off / low refresh / full) and radio interval (fast / slow). The RL Lab learned, with Q-learning, which combination to use for each battery level and user context; the firmware simply looks the answer up in a 24-byte table once a minute and whenever the screen changes.
          <Formula>{'P_avg = Σ_i [P_active,i · d_i + P_sleep,i · (1 − d_i)] / η      life = E_battery / P_avg'}</Formula>
        </Explainer>
      </div>
      <Card title="References and further study (English)">
        <ul className="small" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.9 }}>
          {Object.values(REFS).map((r) => (
            <li key={r.label}>{'href' in r && r.href ? <a href={r.href} target="_blank" rel="noreferrer">{r.label}</a> : r.label}</li>
          ))}
        </ul>
        <p className="muted small" style={{ marginTop: 10 }}>
          Closest open references to what this lab builds: Brilliant Labs publish the firmware and hardware of their open smart-glasses on GitHub (nRF52-based, Bluetooth to a phone app), and OpenGlass turns a small ESP32-S3 camera board into AI glasses. Iris borrows their overall shape — a Bluetooth peripheral with a thin phone protocol — and adds a generated, simulation-verified firmware stack.
        </p>
      </Card>
    </div>
  );
}

export default function Lab() {
  const [tab, setTab] = usePersistent<Tab>('pf.tab', 'build');
  const ev = useEvaluation();
  const m = (k: string) => ev.data?.metrics.find((x) => x.key === k);
  const ram = m('firmware.ram_kb');
  const flash = m('firmware.flash_kb');
  const cpu = m('firmware.cpu_load_pct');
  const util = m('firmware.ble_audio_util_pct');
  const frame = m('firmware.hud_frame_ms');
  return (
    <div className="pf-root">
      <LabPage
        kicker="Prototype & Software"
        title="Prototype & Firmware Lab"
        subtitle={<>Build <b>Iris v0</b> on a breadboard from development boards, light up its buses wire by wire, generate its firmware from the other labs' designs (filters, gesture state machine, learned power policy), simulate the heads-up display pixel for pixel, and design the Bluetooth Low Energy (BLE) protocol to the phone.</>}
        metricPrefixes={['firmware.']}
      >
        <div className="grid grid-4">
          <Stat label="Firmware RAM" value={ram ? fmtNum(ram.value) : '—'} unit="KB" tone={ram && ram.value <= 192 ? 'pass' : 'warn'} note={ram ? `${fmtNum(100 - (ram.value / 256) * 100)} % margin of 256 KB` : ''} />
          <Stat label="Flash image" value={flash ? fmtNum(flash.value) : '—'} unit="KB" tone={flash && flash.value <= 472 ? 'pass' : 'warn'} note="≤ 472 KB leaves room for updates" />
          <Stat label="CPU load" value={cpu ? fmtNum(cpu.value) : '—'} unit="%" tone={cpu && cpu.value <= 50 ? 'pass' : 'warn'} note={frame ? `busiest context · full HUD frame ${fmtNum(frame.value)} ms` : 'busiest context'} />
          <Stat label="BLE audio link use" value={util ? fmtNum(util.value) : '—'} unit="%" tone={util && util.value <= 50 ? 'pass' : 'warn'} note="ADPCM uplink of the fast interval" />
        </div>
        <Tabs<Tab>
          value={tab}
          onChange={setTab}
          tabs={[
            { id: 'build', label: 'Breadboard build' },
            { id: 'wiring', label: 'Wiring' },
            { id: 'firmware', label: 'Firmware generator' },
            { id: 'hud', label: 'HUD simulator' },
            { id: 'ble', label: 'BLE design' },
            { id: 'resources', label: 'Resources' },
            { id: 'learn', label: 'How it works' },
          ]}
        />
        {tab === 'build' && <BuildTab onWiring={() => setTab('wiring')} />}
        {tab === 'wiring' && <WiringTab />}
        {tab === 'firmware' && <FirmwareTab />}
        {tab === 'hud' && <HudTab />}
        {tab === 'ble' && <BleTab />}
        {tab === 'resources' && <ResourcesTab />}
        {tab === 'learn' && <LearnTab />}
      </LabPage>
    </div>
  );
}
