// Shapes returned by systems.next_actions / next_levers / solve_lever
// (crates/forge-core/src/systems/next).
import type { Cmp, Status } from '../../engine/types';

export interface Where {
  route: string;
  tab: string | null;
  label: string;
  page: string;
  tab_label: string | null;
  href: string;
}

export interface Lever {
  param: string;
  label: string;
  unit: string;
  value: number;
  step: number;
  integer: boolean;
  delta: number;
  effect_pct: number;
  direction: 'increase' | 'decrease';
  margin_per_step: number;
  estimate: number | null;
  lo: number;
  hi: number;
}

export type ActionKind =
  | 'failing' | 'unverified' | 'goal' | 'risk' | 'quality_error' | 'marginal' | 'gate'
  | 'no_verification' | 'traceability' | 'quality' | 'quality_info';

export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';

export interface Action {
  id: string;
  kind: ActionKind;
  severity: Severity;
  score: number;
  score_note: string;
  title: string;
  why: string;
  requirements: string[];
  metric: string | null;
  metric_label: string | null;
  value: number | null;
  target: number | null;
  target_hi: number | null;
  cmp: Cmp | null;
  criterion: string | null;
  unit: string;
  margin: number | null;
  note: string;
  where: Where;
  module: string | null;
  levers: Lever[];
  levers_status: 'measured' | 'lazy' | 'none' | 'n/a';
}

export interface Summary {
  requirements: number;
  pass: number;
  pass_with_margin: number;
  marginal: number;
  failing: number;
  open_goals: number;
  unverified: number;
  readiness_pct: number;
  marginal_threshold: number;
  counts: Record<string, number>;
  actions: number;
  levers_measured: string[];
  levers_lazy: string[];
  headline: string;
}

export interface NextActionsOut {
  actions: Action[];
  summary: Summary;
}

export interface LeversOut {
  metric: string;
  module: string | null;
  requirement_id: string | null;
  candidates: number;
  levers: Lever[];
}

export interface Change {
  kind: 'design' | 'component';
  path: string;
  refdes: string | null;
  label: string;
  unit: string;
  before: number;
  after: number;
  assumption: string | null;
}

export interface SideEffect {
  id: string;
  text: string;
  metric: string | null;
  unit: string;
  criterion: string;
  before_status: Status;
  after_status: Status;
  before_value: number | null;
  after_value: number | null;
  before_margin: number | null;
  after_margin: number | null;
  verdict_changed: boolean;
  better: boolean;
  goal: boolean;
}

export interface Solve {
  requirement_id: string;
  metric: string;
  module: string;
  param: string;
  label: string;
  unit: string;
  metric_unit: string;
  criterion: string;
  found: boolean;
  message: string;
  lo: number;
  hi: number;
  before_param: number;
  after_param: number;
  before_value: number | null;
  after_value: number | null;
  before_margin: number | null;
  after_margin: number | null;
  aimed_margin: number;
  runs: number;
  changes: Change[];
  side_effects: SideEffect[];
  pass_before: number;
  pass_after: number;
  fail_before: number;
  fail_after: number;
  open_goals_before: number;
  open_goals_after: number;
}
