// TypeScript mirror of the Systems Studio engine outputs
// (crates/forge-core/src/systems/{analysis,budgets,quality,sensitivity,templates}.rs).
import type { Cmp, Project, Status } from '../../engine/types';

export interface BudgetItem { id: string; label: string; subsystem: string; value: number; kind: 'component' | 'frame' | 'regulator' | 'conversion' }
export interface SubsystemShare { id: string; name: string; value: number; fraction: number }
export interface Budget {
  key: string; label: string; unit: string; total: number; limit: number | null; cmp: Cmp | null;
  requirement: string | null; metric: string | null; verified_value: number | null; status: Status;
  margin: number | null; margin_abs: number | null; items: BudgetItem[]; by_subsystem: SubsystemShare[]; note: string;
}
export interface SubsystemRollup { id: string; name: string; components: number; mass_g: number; cost_usd: number; power_mw: number }
export interface Budgets { mass: Budget; power: Budget; battery_life: Budget; cost: Budget; subsystems: SubsystemRollup[] }

export interface N2Link { id: string; kind: string; description: string }
export interface N2Cell { row: number; col: number; links: N2Link[] }
export interface N2 {
  ids: string[]; names: string[]; domains: string[]; cells: N2Cell[]; fan_out: number[]; fan_in: number[];
  feedback: number; suggested_order: number[]; suggested_feedback: number; dangling: string[];
}

export interface NeedCoverage {
  id: string; text: string; stakeholder: string; priority: number; requirements: string[];
  pass: number; fail: number; unverified: number; status: 'satisfied' | 'failing' | 'partial' | 'unverified' | 'uncovered';
}
export interface SubsystemCoverage { id: string; name: string; lab: string; requirements: string[]; pass: number; fail: number; unverified: number; interfaces: number }
export interface Count { key: string; count: number }
export interface Coverage {
  needs: NeedCoverage[]; subsystems: SubsystemCoverage[]; needs_total: number; needs_covered: number; needs_satisfied: number;
  reqs_total: number; reqs_traced: number; reqs_allocated: number; reqs_with_metric: number; reqs_metric_found: number;
  methods: Count[]; kinds: Count[]; untraced: string[]; unallocated: string[]; no_metric: string[]; missing_metric: string[];
  empty_subsystems: string[]; completeness: number;
}

export interface VItem { label: string; status: 'pass' | 'fail' | 'todo' | 'info'; detail: string; link: string | null }
export interface VStage {
  id: string; title: string; leg: 'left' | 'bottom' | 'right'; level: number;
  status: 'done' | 'partial' | 'attention' | 'empty'; progress: number; summary: string; items: VItem[];
  pairs_with: string | null; link: string;
}
export interface VModel { stages: VStage[]; overall: number }

export interface CheckStatus { text: string; status: 'met' | 'unmet' | 'manual'; auto: boolean; detail: string }
export interface GateStatus { id: string; title: string; stage: string; criteria: CheckStatus[]; deliverables: CheckStatus[]; passed: boolean; current: boolean }

export interface QualityIssue { severity: 'error' | 'warning' | 'info'; id: string; message: string; characteristic: string; hint: string }
export interface RiskResult { id: string; rpn: number; level: 'low' | 'medium' | 'high' }
export interface TradeResult { id: string; scores: number[]; winner: number; weight_sensitivity: (number | null)[] }
export interface TornadoBar { criterion: number; name: string; base_lead: number; lo_lead: number; hi_lead: number; lo_winner: number; hi_winner: number; flip: number | null }
export interface TradeDetail {
  result: TradeResult; title: string; weights: number[]; contributions: number[][]; ranking: number[]; lead: number;
  decision_index: number | null; decision_agrees: boolean; swing: number; tornado: TornadoBar[];
}

export interface TraceNode { id: string; label: string; kind: 'need' | 'requirement' | 'subsystem' | 'metric' | 'lab'; layer: number }
export interface TraceEdge { from: string; to: string; kind: string }
export interface TraceGraph { nodes: TraceNode[]; edges: TraceEdge[] }

export interface Studio {
  budgets: Budgets; n2: N2; coverage: Coverage; vmodel: VModel; gates: GateStatus[]; quality: QualityIssue[];
  fmea: RiskResult[]; trades: TradeDetail[]; trace: TraceGraph;
}

export interface Template { id: string; name: string; description: string; icon: string; tags: string[]; project: Project }
export interface Param { path: string; section: string; name: string; value: number }
export interface SensitivityRow { id: string; metric: string | null; values: (number | null)[]; statuses: Status[]; margins: (number | null)[]; flips: boolean; moves: boolean }
export interface Sensitivity { param: string; baseline: number; values: number[]; baseline_index: number; rows: SensitivityRow[]; pass: number[]; fail: number[]; flipped: string[] }
