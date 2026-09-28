// TypeScript mirrors of crates/forge-core/src/robotics/*.rs outputs.
export interface Arm2 { l1: number; l2: number; q1_min: number; q1_max: number; q2_min: number; q2_max: number }
export interface Ellipse { a: number; b: number; angle_deg: number }
export interface ArmSolution {
  elbow_up: boolean; q_deg: [number, number]; elbow: [number, number]; tip: [number, number]; within_limits: boolean;
  jacobian: [[number, number], [number, number]]; manipulability: number; condition: number | null; ellipse: Ellipse;
}
export interface ArmQuery { reachable: boolean; r_min: number; r_max: number; distance: number; solution: ArmSolution | null; alternate: ArmSolution | null; max_manipulability: number }
export interface Workspace { points: [number, number][]; r_min: number; r_max: number }

export interface ProfileSamples { name: string; duration: number; t: number[]; p: number[]; v: number[]; a: number[]; j: number[]; peak_v: number; peak_a: number; peak_j: number | null }
export interface JointMove { duration: number; t: number[]; q: number[][]; qd: number[][]; qdd: number[][]; own_min_time: number[] }
export interface LineOut {
  duration: number; t: number[]; x: number[]; y: number[]; q1: number[]; q2: number[]; q1d: number[]; q2d: number[];
  manipulability: number[]; reachable: boolean; joint_x: number[]; joint_y: number[]; peak_joint_speed_dps: number;
}

export interface PnpConfig {
  l1_mm: number; l2_mm: number; j1_vmax_dps: number; j1_amax_dps2: number; j2_vmax_dps: number; j2_amax_dps2: number;
  j4_vmax_dps: number; j4_amax_dps2: number; z_stroke_mm: number; z_vmax_mm_s: number; z_amax_mm_s2: number;
  pick_dwell_s: number; place_dwell_s: number; vision: boolean; vision_dwell_s: number; feeder_pitch_mm: number;
  feeder_y_mm: number; camera_xy_mm: [number, number]; board_origin_mm: [number, number]; board_handling_s: number;
  fiducial_dwell_s: number; flip_s: number; optimize: boolean; fps: number;
}
export interface Feeder { slot: number; key: string; footprint: string; refdes: string[]; xy: [number, number] }
export interface Placement { refdes: string; name: string; footprint: string; feeder: number; board_xy: [number, number]; xy: [number, number]; rot_deg: number; side: string; size_mm: [number, number]; t_placed: number }
export interface PnpEvent { t0: number; t1: number; kind: 'travel' | 'z' | 'vision' | 'dwell' | 'handling'; label: string; refdes: string | null }
export interface Breakdown { travel: number; z: number; vision: number; dwell: number; handling: number }
export interface PnpPlan {
  cycle_s: number; naive_cycle_s: number; per_part_s: number; cph: number; breakdown: Breakdown; feeders: Feeder[];
  placements: Placement[]; events: PnpEvent[];
  frames: { t: number[]; x: number[]; y: number[]; z: number[]; q1: number[]; q2: number[]; nozzle_deg: number[]; holding: number[] };
  travel_mm: number; board_w_mm: number; board_h_mm: number; config: PnpConfig; reach_mm: [number, number]; warnings: string[];
}

export interface PursuitOut {
  path_x: number[]; path_y: number[]; path_len: number; t: number[]; x: number[]; y: number[]; theta: number[];
  goal_x: number[]; goal_y: number[]; v_left: number[]; v_right: number[]; curvature: number[]; cross_track: number[];
  rms_cross_track: number; max_cross_track: number; lap_time: number;
}
