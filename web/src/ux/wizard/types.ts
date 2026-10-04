// TypeScript mirror of the new-product wizard's engine API
// (crates/forge-core/src/systems/wizard/{mod,catalog}.rs).
import type { Project } from '../../engine/types';

export type Applies = 'always' | 'battery' | 'radio' | 'electronics' | 'quantum' | 'mechanical' | 'product';
export type MetricClass = 'generic' | 'conditional' | 'partial' | 'iris' | 'template';

export interface KindOption {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  help: string;
  examples: string[];
  default_capabilities: string[];
  defaults: Record<string, number>;
  contact: 'skin' | 'hand' | 'touch';
  surface_limit_c: number;
  scope: string;
}

export interface CapOption {
  id: string;
  name: string;
  group: string;
  help: string;
  excludes: string[];
  electronic: boolean;
}

export interface TargetOption {
  id: string;
  label: string;
  unit: string;
  default: number;
  min: number;
  max: number;
  step: number;
  log: boolean;
  help: string;
  applies: Applies;
}

export interface MetricRow {
  keys: string[];
  lab: string;
  class: MetricClass;
  responds_to: string;
  note: string;
  bound: boolean;
}

export interface WizardOptions {
  kinds: KindOption[];
  capabilities: CapOption[];
  targets: TargetOption[];
  metric_table: MetricRow[];
}

export interface Answers {
  name: string;
  idea: string;
  users: string;
  kind: string;
  capabilities: string[];
  targets: Record<string, number>;
}

export interface Explanation { id: string; title: string; plain: string; math: string[]; result: string }
export interface WizardWarning { level: 'warn' | 'info'; text: string }
export interface LabNote { lab: string; status: 'yours' | 'partial' | 'iris' | 'unused'; note: string }

export interface Summary {
  mass_g: number;
  avg_power_mw: number;
  peak_power_mw: number;
  battery: string;
  battery_mah: number;
  life_h: number | null;
  bom_usd: number;
  bom_volume_usd: number;
  board_mm: [number, number];
  board_layers: number;
  enclosure: string;
  surface_c: number | null;
  surface_limit_c: number;
  bound_requirements: number;
  test_requirements: number;
}

export interface WizardOut {
  project: Project;
  explanations: Explanation[];
  warnings: WizardWarning[];
  labs: LabNote[];
  summary: Summary;
  capabilities: string[];
  targets: Record<string, number>;
}
