// Shapes returned by the firmware.* engine methods (crates/forge-core/src/firmware).

export type Build = 'xiao' | 'esp32s3';
export type Target = 'xiao_arduino' | 'esp32s3_wokwi' | 'zephyr';
export type Screen = 'clock' | 'navigation' | 'captions' | 'notification' | 'teleprompter';

export interface Pin { name: string; x: number; y: number; side: string }
export interface Part { id: string; label: string; sublabel: string; kind: 'board' | 'module' | 'battery'; x: number; y: number; w: number; h: number; pins: Pin[] }
export interface Wire { id: number; from_part: string; from_pin: string; to_part: string; to_pin: string; color: string; signal: string; note: string; route: [number, number][] }
export interface BomLine { item: string; qty: number; category: string; stands_in_for: string; notes: string }
export interface Step { title: string; detail: string; wires: number[] }
export interface Difference { aspect: string; prototype: string; custom: string; consequence: string }
export interface PowerRow { name: string; active_mw: number; sleep_mw: number; duty: number; avg_mw: number }
export interface PowerCompare {
  scenario: string; battery_mah: number;
  proto_rows: PowerRow[]; proto_avg_mw: number; proto_life_h: number; proto_regulator: string; proto_efficiency: number;
  custom_rows: PowerRow[]; custom_avg_mw: number; custom_life_h: number; custom_efficiency: number; ratio: number;
}
export interface Breadboard {
  build: Build; title: string; summary: string; width: number; height: number;
  parts: Part[]; wires: Wire[]; bom: BomLine[]; steps: Step[]; differences: Difference[]; power: PowerCompare; notes: string[];
}

export interface FwFile { path: string; content: string }
export interface Provenance { source: string; item: string; value: string; file: string }
export interface FwProject {
  target: Target; title: string; files: FwFile[]; provenance: Provenance[];
  policy: number[]; soc_buckets: number; policy_gain_pct: number;
  wokwi_diagram: unknown | null; commands: string[];
}

export interface ResItem { name: string; group: string; bytes: number; basis: 'computed' | 'assumption'; note: string }
export interface ThreadInfo { name: string; priority: number; stack: number; rate: string; work: string }
export interface ResourceParams {
  panel_w: number; panel_h: number; fb_bpp: number; double_buffer: boolean; mics: number; audio_blocks: number; imu_batch_ms: number;
  ble_ram_kb: number; kernel_ram_kb: number; heap_kb: number; ble_flash_kb: number; kernel_flash_kb: number; log_flash_kb: number;
  bytes_per_statement: number; dfu: boolean; bootloader_kb: number; storage_kb: number; stacks: [string, number][];
}
export interface Resources {
  params: ResourceParams; ram: ResItem[]; flash: ResItem[]; ram_kb: number; flash_kb: number;
  ram_capacity_kb: number; flash_capacity_kb: number; flash_budget_kb: number; ram_pct: number; flash_pct_of_budget: number;
  recommended_ram_kb: number; threads: ThreadInfo[]; notes: string[];
}

export interface LinkParams {
  conn_interval_ms: number; slow_interval_ms: number; mtu: number; phy: '1M' | '2M'; dle: boolean; encrypted: boolean;
  max_packets_per_event: number; peripheral_latency: number; asr_ms: number; audio_frame_ms: number; caption_chars: number;
  audio_rate_hz: number; audio_bits: number;
}
export interface Slot { t_us: number; dur_us: number; from: 'central' | 'peripheral'; bytes: number; data: boolean }
export interface LinkResult {
  params: LinkParams; bit_us: number; ll_payload_max: number; att_data_per_notification: number; ll_packets_per_notification: number;
  data_packet_us: number; empty_packet_us: number; exchange_us: number; packets_time_limited: number; packets_per_event: number;
  app_bytes_per_event: number; throughput_kbps: number; slow_throughput_kbps: number; raw_phy_kbps: number; efficiency: number;
  audio_demand_kbps: number; audio_margin: number; caption_downlink_avg_ms: number; caption_downlink_worst_ms: number;
  audio_uplink_avg_ms: number; caption_link_latency_ms: number; caption_end_to_end_ms: number; event: Slot[]; notes: string[];
}
export interface GattField { name: string; ctype: string; bytes: number; desc: string }
export interface GattChar { name: string; c_name: string; uuid: string; properties: string[]; direction: string; description: string; fields: GattField[]; header_bytes: number; trailing: string; max_len: number }
export interface GattService { name: string; uuid: string; standard: boolean; characteristics: GattChar[] }
export interface BleOut { services: GattService[]; link: LinkResult; c_header: string; markdown: string; json_spec: string }

export interface HudState {
  screen: Screen; hour: number; minute: number; battery_pct: number; ble_connected: boolean;
  nav_arrow_deg: number; nav_distance_m: number; nav_street: string; nav_eta_min: number;
  captions: string; caption_scroll_px: number; notif_app: string; notif_title: string; notif_body: string;
  prompter: string; prompter_scroll_px: number; prompter_progress: number;
}
export interface HudOut { width: number; height: number; bpp: number; stride: number; font_scale: number; data_b64: string; lit: number; hash: string }
export interface GestureOut { state: number; state_name: string; timer: number; events: { tick: number; output: string }[]; states: string[]; tap_ms: number; long_ms: number; gap_ms: number; c_source: string }
export interface ScreenDesc { id: Screen; title: string; c_table: string; context: string; widgets: { kind_name: string; field_name: string; kind: number; field: number; x: number; y: number; w: number; h: number; size: number; level: number }[] }
