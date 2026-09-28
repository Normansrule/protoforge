// Engine method catalogue, generated from the `dispatch` functions in
// crates/forge-core/src/*/ (grep for `"module.verb" =>`). `api.methods` only
// lists modules at runtime, so the Overview shows this static snapshot.
export const METHOD_CATALOG: Record<string, string[]> = {
  project: ['blank', 'iris', 'validate'],
  systems: ['evaluate', 'fmea', 'impact', 'lint', 'metrics', 'report', 'trace', 'trade', 'trades'],
  circuit: ['parse', 'presets', 'run_preset', 'simulate'],
  power: [],
  dsp: ['analyze', 'apply', 'beam_field', 'beamformer', 'decimation_design', 'fft', 'filter_response', 'fir_design', 'fixed_point', 'iir_design', 'leakage', 'pdm_chain', 'psd', 'signal', 'spectrum', 'window'],
  chip: ['circuits', 'csd', 'fsm', 'iris_dsp', 'minimize', 'simulate', 'tinytapeout'],
  control: ['analyze', 'cartpole', 'frequency', 'head_tracking', 'plants', 'response', 'root_locus', 'tune'],
  robotics: ['arm', 'cartesian_line', 'joint_move', 'pnp', 'profiles', 'pursuit', 'workspace'],
  rl: ['cartpole', 'gridworld', 'power_frontier', 'power_policy', 'value_iteration'],
  optics: [],
  cad: ['3mf', 'enclosure', 'enclosure_defaults', 'fit', 'layout', 'mass', 'obj', 'parts', 'stl', 'triangulate', 'validate'],
  print: ['estimate', 'overhangs', 'settings_defaults', 'slice'],
  pcb: ['adopt', 'anneal', 'fab_files', 'footprints', 'ipc2221', 'layout', 'placement', 'route_trace'],
  molding: ['analyze', 'cooling', 'draft', 'materials', 'parts', 'thickness'],
  mfg: ['assumptions', 'cost', 'guides', 'molding_report', 'package', 'producers', 'stages', 'vendors'],
};
