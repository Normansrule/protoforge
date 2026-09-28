// TypeScript mirrors of crates/forge-core/src/chip/*.rs outputs.

export interface KCell { row: number; col: number; minterm: number; value: '0' | '1' | 'x' }
export interface KRect { r0: number; r1: number; c0: number; c1: number }
export interface KGroup { term: string; pattern: string; rects: KRect[]; essential: boolean }
export interface KMap {
  rows: number; cols: number; row_vars: string; col_vars: string; row_labels: string[]; col_labels: string[];
  cells: KCell[]; groups: KGroup[];
}
export interface PrimeInfo { pattern: string; term: string; minterms: number[]; essential: boolean; chosen: boolean }
export interface LogicGate { id: string; kind: string; inputs: string[]; output: string; col: number; row: number }
export interface MinimizeResult {
  vars: string[]; table: string[]; minterms: number[]; dont_cares: number[]; qm_columns: string[][]; primes: PrimeInfo[];
  method: string; sop: string; pos: string; sop_verilog: string; pos_verilog: string; sop_literals: number; pos_literals: number;
  kmap: KMap | null; netlist: LogicGate[]; verilog: string; sigma: string; pi: string;
}

export type GateKind = 'AND' | 'OR' | 'NOT' | 'NAND' | 'NOR' | 'XOR' | 'XNOR' | 'BUF' | 'TIE0' | 'TIE1' | 'DFF';
export interface PlacedGate { id: string; kind: GateKind; inputs: string[]; output: string; delay_ns: number; level: number; row: number }
export interface Wave { net: string; role: 'input' | 'output' | 'internal'; points: [number, number][]; transitions: number }
export interface Glitch { net: string; t0: number; t1: number }
export interface CriticalPath { delay_ns: number; gates: string[]; nets: string[]; from: string; to: string; fmax_mhz: number | null }
export interface Clock { net: string; period_ns: number; start_ns: number }
export interface Stimulus { changes: [number, string, number][]; clocks: Clock[]; t_end_ns: number }
export interface SimResult {
  name: string; title: string; description: string; inputs: string[]; outputs: string[]; gates: PlacedGate[]; levels: number;
  t_end_ns: number; waves: Wave[]; glitches: Glitch[]; critical: CriticalPath; events: number; has_loop: boolean; stimulus: Stimulus;
}
export interface CircuitInfo { name: string; title: string; description: string; inputs: string[] }

export interface Guard { touch: boolean | null; timer_ge: number | null; timer_lt: number | null }
export interface FsmState { name: string; outputs: string[]; description: string; x: number; y: number }
export interface Transition { from: string; to: string; guard: Guard }
export interface Fsm { name: string; tick_hz: number; outputs: string[]; states: FsmState[]; transitions: Transition[]; reset: string }
export interface GestureEvent { tick: number; output: string; state: string }
export interface FsmTrace { states: number[]; timers: number[]; events: GestureEvent[]; final_state: number; final_timer: number; fired: number[] }
export interface FsmCost { state_ffs: number; timer_ffs: number; timer_incr_cells: number; comparator_cells: number; next_state_cells: number; output_cells: number; total_cells: number; critical_ns: number }
export interface FsmOut { fsm: Fsm; touch: number[]; trace: FsmTrace; timer_bits: number; state_bits: number; verilog: string | null; testbench: string | null; cost: FsmCost }

export type Arch = 'transposed' | 'mac';
export type AdderKind = 'ripple' | 'cla';
export type Encoding = 'binary' | 'one_hot';

export interface CoefInfo {
  index: number; value: number; quantized: number; code: number; csd: number[]; nonzero: number; binary_nonzero: number;
  adders: number; tree_depth: number; shared_with: number | null;
}
export interface BlockCost { name: string; cells: number; ffs: number; adder_cells: number; other_cells: number; critical_ns: number; critical_desc: string; detail: string }
export interface FirGen {
  arch: Arch; taps: number; coeff_bits: number; frac_bits: number; data_bits: number; acc_bits: number; decim: number; coefs: CoefInfo[];
  total_csd_digits: number; total_binary_digits: number; unique_multipliers: number; cost: BlockCost; verilog: string; testbench: string;
  x_test: number[]; y_expected: number[]; cycles_per_output: number; min_input_spacing: number;
}
export interface DspChipCost {
  blocks: BlockCost[]; total_cells: number; total_cells_pessimistic: number; critical_ns: number; critical_block: string; critical_desc: string;
  clock_period_ns: number; slack_ns: number; cell_budget: number; timing_ok: boolean; area_ok: boolean; suggestions: string[];
}
export interface ResolvedChoice { arch: Arch; adder: AdderKind; pipeline: boolean; encoding: Encoding; coeff_bits: number; data_bits: number; tap_ms: number; long_ms: number; gap_ms: number }
export interface CicSpec { order: number; rate: number; diff_delay: number; width: number }
export interface IrisDsp { choice: ResolvedChoice; coeff_source: string; cic: CicSpec; fir: FirGen; fsm: Fsm; summary: DspChipCost; alternative: DspChipCost }
export interface TtFile { name: string; content: string }
export interface TtOut { files: TtFile[]; design: IrisDsp }
