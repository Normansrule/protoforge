// Shared state of the conduction / spreader tabs (what-if choices).
import { useState } from 'react';
import { useProject } from '../../state/project';
import type { PodConfig } from './types';
import { fluidsDesign } from './util';

export interface PodControls {
  scenario: string;
  display_heat_in: string;
  spreader: string;
  spreader_um: number;
  spreader_location: string;
  shell_material: string;
  emissivity: number | null;
  air_speed_m_s: number;
  voxel_mm: number;
  precond: string;
}

export function usePodControls() {
  const project = useProject((s) => s.project);
  const d = fluidsDesign(project);
  return useState<PodControls>({
    scenario: 'display_on',
    display_heat_in: d.display_heat_in,
    spreader: d.spreader,
    spreader_um: d.spreader_um,
    spreader_location: d.spreader_location,
    shell_material: project.design.frame.material,
    emissivity: d.emissivity ?? null,
    air_speed_m_s: d.air_speed_m_s,
    voxel_mm: 1.5,
    precond: 'ssor',
  });
}

export function configOf(c: PodControls): PodConfig {
  return {
    voxel_mm: c.voxel_mm,
    scenario: c.scenario,
    display_heat_in: c.display_heat_in,
    spreader: c.spreader,
    spreader_um: c.spreader_um,
    spreader_location: c.spreader_location,
    shell_material: c.shell_material,
    emissivity: c.emissivity ?? undefined,
    air_speed_m_s: c.air_speed_m_s,
    precond: c.precond,
  };
}

