// Types mirroring crates/forge-core/src/rf (serde snake_case).

export interface C64 { re: number; im: number }

export type ElKind = 'series_l' | 'series_c' | 'shunt_l' | 'shunt_c';
export interface Element { kind: ElKind; value: number; q: number }
export interface AntennaModel { r_ohm: number; x_ohm: number; q: number; fc_ghz: number }
export type LineKind = 'microstrip' | 'cpwg' | 'diff_microstrip';
export interface Feed { kind: LineKind; width_mm: number; gap_mm: number; length_mm: number }

export interface LinkDesign {
  phy: string; distance_m: number; path_exponent: number; tx_dbm: number; phone_tx_dbm: number; phone_gain_dbi: number;
  phone_sens_1m_dbm: number; head_loss_db: number; body_loss_db: number; availability: number;
}

export interface IrisRf {
  antenna: AntennaModel; rad_efficiency: number; directivity_dbi: number; network: Element[] | null; feed: Feed | null;
  er: number; tan_d: number; cu_um: number; q_l: number; q_c: number; series: string; link: LinkDesign;
}

export interface BandSummary { worst_rl_db: number; worst_vswr: number; worst_gt_db: number; centre_rl_db: number }
export interface Sweep { f_ghz: number[]; s11_re: number[]; s11_im: number[]; rl_db: number[]; vswr: number[]; gt_db: number[] }

export interface Step { label: string; delta_db: number; level_dbm: number }
export interface Direction { name: string; steps: Step[]; rx_dbm: number; sens_dbm: number; fade_db: number; margin_db: number }
export interface PhyRange { id: string; name: string; rate_kbps: number; margin_db: number; range_m: number }
export interface LinkOut {
  phy: string; path_loss_db: number; fspl_1m_db: number; fade_db: number; forward: Direction; reverse: Direction; margin_db: number;
  limiting: string; range_m: number; phys: PhyRange[]; dist_m: number[]; rx_fwd_dbm: number[]; rx_rev_dbm: number[]; notes: string[];
}

export interface Metric { key: string; label: string; value: number; unit: string; source: string; note: string }
export interface BomRow { refdes: string; name: string; value: string; footprint: string; note: string; in_bom: boolean }

export interface IrisOut {
  design: IrisRf; stored: boolean; h_mm: number;
  current_network: Element[]; current_labels: string[]; current_summary: BandSummary;
  recommended_network: Element[]; recommended_labels: string[]; recommended_summary: BandSummary;
  current_feed: Feed; current_feed_source: string; current_feed_z0: number; current_feed_eeff: number;
  recommended_feed: Feed; recommended_feed_z0: number;
  glasses_gain_dbi: number; match_loss_db: number; link: LinkOut; bom: BomRow[]; metrics: Metric[];
  suggested_extra: IrisRf; notes: string[];
}

export interface Solution {
  name: string; topology: string; ideal: Element[]; snapped: Element[]; labels: string[];
  ideal_summary: BandSummary; snapped_summary: BandSummary; sweep: Sweep; band: Sweep;
  path: [number, number][][]; loaded_q: number; touchstone: string;
}
export interface MatchOut {
  z_load: C64; gamma_load: C64; f0_ghz: number; bare: BandSummary; bare_sweep: Sweep; solutions: Solution[]; best: number;
  current: Solution | null; l_network_q: number; notes: string[];
}

export interface LineOut {
  kind: LineKind; width_mm: number; gap_mm: number; h_mm: number; z0_ohm: number; zdiff_ohm: number; eeff: number;
  wavelength_mm: number; electrical_deg: number; delay_ps: number; alpha_c_db_m: number; alpha_d_db_m: number; loss_db: number;
  microstrip_z0_ohm: number; sweep_w: number[]; sweep_z: number[]; notes: string[];
}

export type ElementKind = 'isotropic' | 'short' | 'half_wave' | 'dipole';
export interface PatternOut {
  directivity: number; directivity_dbi: number; realized_gain_dbi: number; total_efficiency_pct: number;
  beam_theta_deg: number; beam_phi_deg: number; hpbw_e_deg: number; hpbw_h_deg: number;
  cut_angle_deg: number[]; e_plane_db: number[]; h_plane_db: number[];
  mesh_positions: number[]; mesh_indices: number[]; mesh_level: number[];
}
export interface SizeRow { name: string; length_mm: number; eeff: number; note: string }
export interface SizeOut {
  lambda0_mm: number; rows: SizeRow[]; ground_len_wl: number; chu_q_chip: number; chu_q_board: number; chu_bw_board_pct: number;
  ka_chip: number; ka_board: number; notes: string[];
}

export interface Legend { id: number; name: string; eps_r: number; sigma: number | null }
export interface FdtdOut {
  preset: string; nx: number; ny: number; dx_mm: number; dt_ps: number; dt_max_ps: number; courant: number; stable: boolean;
  cells_per_wavelength: number; cells_per_wavelength_min: number; pml: number; steps: number;
  frames_b64: string; n_frames: number; frame_time_ns: number[]; frame_step: number[]; material_b64: string; legend: Legend[];
  scale: number; gamma: number; sources: [number, number][]; probe: [number, number]; probe_t_ns: number[]; probe_ez: number[];
  energy: number[]; absorbed_fraction: number | null; absorbed_db: number | null; notes: string[]; ms_hint: number;
}
