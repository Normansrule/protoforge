//! The product model: one JSON document that carries a product from idea to
//! factory. The systems studio edits it, every lab reads its `design`
//! parameters, and verification checks its requirements against the metrics
//! the labs compute.
//!
//! The shipped example is **Iris**, a pair of monocular heads-up-display
//! smart glasses (`examples/smart-glasses/project.json`). A blank template is
//! available via `project.blank`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};

/// The Iris smart-glasses example, embedded so the browser build has it.
pub const IRIS_JSON: &str = include_str!("../../../examples/smart-glasses/project.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub meta: Meta,
    pub concept: Concept,
    #[serde(default)]
    pub needs: Vec<Need>,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    #[serde(default)]
    pub subsystems: Vec<Subsystem>,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub trades: Vec<TradeStudy>,
    #[serde(default)]
    pub risks: Vec<Risk>,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    pub design: Design,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub codename: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Concept {
    #[serde(default)]
    pub problem: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub use_cases: Vec<UseCase>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub narrative: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Need {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub stakeholder: String,
    /// 1 = must, 2 = should, 3 = could
    #[serde(default = "one")]
    pub priority: u8,
}

fn one() -> u8 {
    1
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReqKind {
    #[default]
    Performance,
    Functional,
    Interface,
    Environmental,
    Safety,
    Regulatory,
    Manufacturing,
    Cost,
    Goal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyMethod {
    #[default]
    Analysis,
    Test,
    Inspection,
    Demonstration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cmp {
    /// value <= target
    #[default]
    Le,
    /// value >= target
    Ge,
    /// |value - target| <= tolerance (target_hi used as tolerance)
    Eq,
    /// target <= value <= target_hi
    Between,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Verification {
    #[serde(default)]
    pub method: VerifyMethod,
    /// Metric key produced by a lab, e.g. `power.battery_life_h`.
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub cmp: Cmp,
    #[serde(default)]
    pub target: f64,
    #[serde(default)]
    pub target_hi: Option<f64>,
    #[serde(default)]
    pub unit: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub kind: ReqKind,
    /// Need ids this requirement derives from.
    #[serde(default)]
    pub derives_from: Vec<String>,
    /// Subsystem ids responsible for meeting it.
    #[serde(default)]
    pub allocated_to: Vec<String>,
    #[serde(default)]
    pub verification: Verification,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    #[default]
    Electrical,
    Mechanical,
    Optical,
    Software,
    Silicon,
    Acoustic,
    Thermal,
    Manufacturing,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Subsystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub domain: Domain,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub functions: Vec<String>,
    /// Which lab in the web app designs/analyses this subsystem.
    #[serde(default)]
    pub lab: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub from: String,
    pub to: String,
    /// I2C, SPI, PDM, MIPI, BLE, USB, POWER, MECH, OPTICAL, THERMAL ...
    pub kind: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PowerProfile {
    /// Power while active, milliwatts.
    #[serde(default)]
    pub active_mw: f64,
    /// Power while idle/sleeping, milliwatts.
    #[serde(default)]
    pub sleep_mw: f64,
    /// Fraction of time active in the reference use profile (0..1).
    #[serde(default)]
    pub duty: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Component {
    /// Reference designator, e.g. `U1`.
    pub refdes: String,
    pub name: String,
    /// Generic part class; `example_part` is only a starting point to verify.
    #[serde(default)]
    pub part_class: String,
    #[serde(default)]
    pub example_part: String,
    #[serde(default)]
    pub subsystem: String,
    #[serde(default = "one_f")]
    pub qty: f64,
    #[serde(default)]
    pub unit_cost_usd: f64,
    #[serde(default)]
    pub mass_g: f64,
    #[serde(default)]
    pub power: PowerProfile,
    /// Footprint name understood by the PCB module (e.g. `QFN-48_6x6`, `0402`).
    #[serde(default)]
    pub footprint: String,
    /// Whether the part is placed on the main PCB.
    #[serde(default)]
    pub on_pcb: bool,
    /// Where it lives: `right_pod`, `left_pod`, `front`, ...
    #[serde(default)]
    pub location: String,
}

fn one_f() -> f64 {
    1.0
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Criterion {
    pub name: String,
    pub weight: f64,
    /// true when bigger raw scores are better (scores are 1..10 anyway).
    #[serde(default = "yes")]
    pub higher_is_better: bool,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeOption {
    pub name: String,
    /// One score per criterion, 1..10.
    pub scores: Vec<f64>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeStudy {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub decision: String,
    pub criteria: Vec<Criterion>,
    pub options: Vec<TradeOption>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Risk {
    pub id: String,
    pub item: String,
    pub failure_mode: String,
    #[serde(default)]
    pub effect: String,
    #[serde(default)]
    pub cause: String,
    /// 1..10 each (FMEA scale)
    pub severity: u8,
    pub occurrence: u8,
    pub detection: u8,
    #[serde(default)]
    pub mitigation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub deliverables: Vec<String>,
    #[serde(default)]
    pub exit_criteria: Vec<String>,
}

/// Domain parameters consumed by the labs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Design {
    #[serde(default)]
    pub frame: FrameParams,
    #[serde(default)]
    pub battery: BatteryParams,
    #[serde(default)]
    pub power_rails: PowerRails,
    #[serde(default)]
    pub optics: OpticsParams,
    #[serde(default)]
    pub audio: AudioParams,
    #[serde(default)]
    pub imu: ImuParams,
    #[serde(default)]
    pub pcb: PcbParams,
    #[serde(default)]
    pub thermal: ThermalParams,
    #[serde(default)]
    pub manufacturing: MfgParams,
    #[serde(default)]
    pub print: PrintParams,
    #[serde(default)]
    pub chip: ChipParams,
    /// Extra SPICE netlists supplied by the user (the circuit lab also ships presets).
    #[serde(default)]
    pub circuits: Vec<NamedNetlist>,
    /// Free-form extra parameters for custom products.
    #[serde(default)]
    pub extra: serde_json::Map<String, Value>,
}

/// Eyewear frame geometry. Sizes follow the "boxing system" convention
/// printed inside real frames: lens width – bridge – temple length (ISO 8624).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameParams {
    pub lens_width_mm: f64,
    pub lens_height_mm: f64,
    pub bridge_mm: f64,
    pub temple_length_mm: f64,
    /// Front thickness (Z, towards the face).
    pub front_thickness_mm: f64,
    /// Width of the rim material around each lens.
    pub rim_width_mm: f64,
    /// Superellipse exponent for the lens shape (2 = ellipse, 4+ = squarer).
    pub lens_shape_exponent: f64,
    pub temple_width_mm: f64,
    pub temple_height_mm: f64,
    /// Electronics pod on the right temple.
    pub pod_length_mm: f64,
    pub pod_width_mm: f64,
    pub pod_height_mm: f64,
    /// Battery pod on the left temple.
    pub battery_pod_length_mm: f64,
    pub wall_mm: f64,
    /// Draft angle applied to extruded walls (for injection molding).
    pub draft_deg: f64,
    pub pantoscopic_tilt_deg: f64,
    pub material: String,
    pub density_g_cm3: f64,
}

impl Default for FrameParams {
    fn default() -> Self {
        FrameParams {
            lens_width_mm: 50.0,
            lens_height_mm: 38.0,
            bridge_mm: 18.0,
            temple_length_mm: 145.0,
            front_thickness_mm: 5.0,
            rim_width_mm: 5.0,
            lens_shape_exponent: 3.0,
            temple_width_mm: 4.0,
            temple_height_mm: 8.0,
            pod_length_mm: 62.0,
            pod_width_mm: 11.0,
            pod_height_mm: 14.0,
            battery_pod_length_mm: 40.0,
            wall_mm: 1.2,
            draft_deg: 1.5,
            pantoscopic_tilt_deg: 8.0,
            material: "PETG".into(),
            density_g_cm3: 1.27,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BatteryParams {
    pub chemistry: String,
    pub capacity_mah: f64,
    pub nominal_v: f64,
    pub full_v: f64,
    pub cutoff_v: f64,
    pub internal_resistance_ohm: f64,
    pub mass_g: f64,
    /// Usable fraction of rated capacity (aging, temperature, cutoff margin).
    pub usable_fraction: f64,
}

impl Default for BatteryParams {
    fn default() -> Self {
        BatteryParams {
            chemistry: "LiPo".into(),
            capacity_mah: 180.0,
            nominal_v: 3.7,
            full_v: 4.2,
            cutoff_v: 3.3,
            internal_resistance_ohm: 0.35,
            mass_g: 4.0,
            usable_fraction: 0.85,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerRails {
    pub core_v: f64,
    pub buck_efficiency: f64,
    pub display_rail_v: f64,
    pub quiescent_ua: f64,
}

impl Default for PowerRails {
    fn default() -> Self {
        PowerRails { core_v: 1.8, buck_efficiency: 0.9, display_rail_v: 3.3, quiescent_ua: 5.0 }
    }
}

/// A simple "birdbath / beam-splitter" display: micro-display → collimating
/// lens → 45° combiner → eye.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OpticsParams {
    pub display_width_mm: f64,
    pub display_height_mm: f64,
    pub px_w: u32,
    pub px_h: u32,
    pub lens_focal_mm: f64,
    pub lens_diameter_mm: f64,
    /// Display-to-lens distance; slightly less than f gives a virtual image.
    pub display_to_lens_mm: f64,
    pub lens_to_combiner_mm: f64,
    pub eye_relief_mm: f64,
    pub combiner_reflectance: f64,
    pub display_luminance_nits: f64,
}

impl Default for OpticsParams {
    fn default() -> Self {
        OpticsParams {
            display_width_mm: 8.4,
            display_height_mm: 5.25,
            px_w: 640,
            px_h: 400,
            lens_focal_mm: 20.0,
            lens_diameter_mm: 10.0,
            display_to_lens_mm: 19.7,
            lens_to_combiner_mm: 8.0,
            eye_relief_mm: 18.0,
            combiner_reflectance: 0.3,
            display_luminance_nits: 3000.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioParams {
    pub mic_count: u32,
    pub mic_spacing_mm: f64,
    pub sample_rate_hz: f64,
    pub pdm_clock_hz: f64,
    pub decimation: u32,
    pub fir_taps: u32,
    pub voice_band_lo_hz: f64,
    pub voice_band_hi_hz: f64,
}

impl Default for AudioParams {
    fn default() -> Self {
        AudioParams {
            mic_count: 2,
            mic_spacing_mm: 14.0,
            sample_rate_hz: 16000.0,
            pdm_clock_hz: 1_024_000.0,
            decimation: 64,
            fir_taps: 31,
            voice_band_lo_hz: 100.0,
            voice_band_hi_hz: 4000.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ImuParams {
    pub rate_hz: f64,
    /// Gyro white noise density, deg/s/√Hz.
    pub gyro_noise_dps_rthz: f64,
    /// Constant gyro bias, deg/s.
    pub gyro_bias_dps: f64,
    /// Accelerometer noise density, mg/√Hz.
    pub accel_noise_mg_rthz: f64,
    /// Complementary-filter blend (0..1, weight on the gyro path).
    pub fusion_alpha: f64,
}

impl Default for ImuParams {
    fn default() -> Self {
        ImuParams { rate_hz: 200.0, gyro_noise_dps_rthz: 0.008, gyro_bias_dps: 0.5, accel_noise_mg_rthz: 0.16, fusion_alpha: 0.98 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PcbParams {
    pub width_mm: f64,
    pub height_mm: f64,
    pub layers: u32,
    pub thickness_mm: f64,
    pub min_trace_mm: f64,
    pub min_clearance_mm: f64,
    pub via_drill_mm: f64,
    pub via_pad_mm: f64,
    pub grid_mm: f64,
}

impl Default for PcbParams {
    fn default() -> Self {
        PcbParams {
            width_mm: 56.0,
            height_mm: 9.0,
            layers: 2,
            thickness_mm: 0.8,
            min_trace_mm: 0.15,
            min_clearance_mm: 0.15,
            via_drill_mm: 0.3,
            via_pad_mm: 0.6,
            grid_mm: 0.25,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ThermalParams {
    pub ambient_c: f64,
    /// Pod-to-ambient thermal resistance, °C/W (natural convection, plastic shell).
    pub r_pod_ambient_c_per_w: f64,
    /// Junction/board-to-pod thermal resistance, °C/W.
    pub r_board_pod_c_per_w: f64,
    /// Heat capacity of the pod assembly, J/°C.
    pub c_pod_j_per_c: f64,
    pub c_board_j_per_c: f64,
    /// Design limit for skin-contact surfaces.
    pub skin_limit_c: f64,
}

impl Default for ThermalParams {
    fn default() -> Self {
        ThermalParams {
            ambient_c: 30.0,
            r_pod_ambient_c_per_w: 55.0,
            r_board_pod_c_per_w: 20.0,
            c_pod_j_per_c: 6.0,
            c_board_j_per_c: 1.5,
            skin_limit_c: 41.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MfgParams {
    pub quantities: Vec<f64>,
    pub target_quantity: f64,
    pub molding_material: String,
    pub cavities: u32,
    pub mold_steel: String,
    pub assembly_minutes: f64,
    pub labor_rate_usd_h: f64,
}

impl Default for MfgParams {
    fn default() -> Self {
        MfgParams {
            quantities: vec![1.0, 10.0, 100.0, 1000.0, 10000.0],
            target_quantity: 1000.0,
            molding_material: "PC".into(),
            cavities: 1,
            mold_steel: "P20".into(),
            assembly_minutes: 12.0,
            labor_rate_usd_h: 30.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintParams {
    pub nozzle_mm: f64,
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub perimeters: u32,
    pub infill_pct: f64,
    pub print_speed_mm_s: f64,
    pub travel_speed_mm_s: f64,
    pub filament_diameter_mm: f64,
    pub nozzle_temp_c: f64,
    pub bed_temp_c: f64,
}

impl Default for PrintParams {
    fn default() -> Self {
        PrintParams {
            nozzle_mm: 0.4,
            layer_height_mm: 0.2,
            line_width_mm: 0.45,
            perimeters: 3,
            infill_pct: 25.0,
            print_speed_mm_s: 60.0,
            travel_speed_mm_s: 150.0,
            filament_diameter_mm: 1.75,
            nozzle_temp_c: 240.0,
            bed_temp_c: 80.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ChipParams {
    pub clock_mhz: f64,
    pub coeff_bits: u32,
    pub data_bits: u32,
    /// Gate delay used by the static timing estimate, ns.
    pub gate_delay_ns: f64,
    pub cell_budget: f64,
}

impl Default for ChipParams {
    fn default() -> Self {
        ChipParams { clock_mhz: 50.0, coeff_bits: 12, data_bits: 16, gate_delay_ns: 0.25, cell_budget: 3000.0 }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NamedNetlist {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub netlist: String,
}

/// A number produced by an analysis, addressable by requirements.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Metric {
    /// Dotted key, e.g. `power.battery_life_h`.
    pub key: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    /// Lab / module that produced it.
    pub source: String,
    #[serde(default)]
    pub note: String,
}

impl Metric {
    pub fn new(key: &str, label: &str, value: f64, unit: &str, source: &str) -> Self {
        Metric { key: key.into(), label: label.into(), value, unit: unit.into(), source: source.into(), note: String::new() }
    }
    pub fn with_note(mut self, note: &str) -> Self {
        self.note = note.into();
        self
    }
}

impl Project {
    /// The Iris smart-glasses reference project.
    pub fn iris() -> Project {
        serde_json::from_str(IRIS_JSON).expect("embedded Iris project must parse")
    }

    /// An empty project to start something new.
    pub fn blank(name: &str) -> Project {
        Project {
            meta: Meta {
                id: name.to_lowercase().replace(' ', "-"),
                name: name.into(),
                version: "0.1.0".into(),
                tagline: "A new product, from idea to atoms".into(),
                ..Default::default()
            },
            concept: Concept::default(),
            needs: vec![],
            requirements: vec![],
            subsystems: vec![],
            interfaces: vec![],
            components: vec![],
            trades: vec![],
            risks: vec![],
            milestones: vec![],
            design: Design::default(),
        }
    }

    pub fn component_mass_g(&self) -> f64 {
        self.components.iter().map(|c| c.mass_g * c.qty).sum()
    }

    pub fn bom_cost_usd(&self) -> f64 {
        self.components.iter().map(|c| c.unit_cost_usd * c.qty).sum()
    }
}

#[derive(Deserialize)]
struct BlankIn {
    #[serde(default)]
    name: Option<String>,
}

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "project.iris" => Some(crate::api::to_value(Project::iris())),
        "project.blank" => handle(input, |i: BlankIn| Ok(Project::blank(i.name.as_deref().unwrap_or("Untitled product")))),
        "project.validate" => handle(input, |p: Project| Ok(crate::systems::lint(&p))),
        _ => None,
    }
}
