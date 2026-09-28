# Chip Lab

The Chip Lab (`#/chip`) goes from Boolean algebra to a Tiny Tapeout submission for **IRIS-DSP**, the optional co-processor in the Iris
smart glasses. IRIS-DSP holds the PDM decimation filter and the touch-gesture state machine. The engine is
`crates/forge-core/src/chip/`, compiled to WebAssembly.

| Tab | What you do there |
|---|---|
| **Logic minimiser** | Enter an expression (`& * · \| + ^ ! ~ '`, parentheses, implicit AND) or a minterm list with don't-cares (up to 8 variables). You get the truth table, an animated K-map (≤ 4 variables, Gray-code order, wrap-around groups), the Quine–McCluskey prime-implicant table, the minimum SOP and POS, the gate schematic and Verilog. One click sends the circuit to the simulator. |
| **Gate simulator** | Built-in circuits: half adder, full adder, 4-bit ripple-carry adder, 2:1 mux, SR latch, 4-bit counter, and the minimised function. You can use the scripted stimulus or click the inputs yourself. A playhead animates the waveforms, glitches are boxed in red, the critical path is highlighted on the schematic, wires are coloured by logic value, and the run exports as VCD. |
| **Gesture FSM** | A press-and-hold touch pad (mouse, touch or space bar) drives the Iris gesture FSM live at 1 kHz. The state diagram glows on the active state and the fired transition. There are scripted traces, a 3-second timeline and an event log. Tap, long-press and double-tap times are adjustable. Binary or one-hot Verilog comes with a self-checking testbench and a cost estimate. |
| **FIR → Verilog** | Architecture (decimating MAC or transposed form), adders (ripple or carry-lookahead), pipelining, word lengths, clock and gate delay, all stored in the project. Shows the CSD digit grid, the quantised coefficients, the quantised response, the cell and timing budget bars against 3000 cells and 20 ns, a block breakdown, and the RTL and testbench. |
| **Tape-out** | The `tt_um_iris_dsp` pin-out, every generated file (RTL, three self-checking testbenches, README) with a preview, a zip download, and exact commands for Icarus Verilog, Verilator, Yosys, cocotb and the Tiny Tapeout flow. |

## Engine API (`chip.*`)

| Method | Input → output |
|---|---|
| `chip.minimize` | `{expr}` or `{minterms, dont_cares, num_vars, vars}` → truth table, QM merge columns, primes (essential/chosen), SOP/POS (maths and Verilog), literal counts, K-map layout with group rectangles, netlist, Verilog |
| `chip.circuits` | `{}` → built-in circuit list |
| `chip.simulate` | `{circuit}` \| `{custom}` \| `{logic}`, optional `stimulus`, `delay_scale`, `glitch_ns` → placed gates, waveforms (step series), glitches, critical path, event count |
| `chip.fsm` | `{tap_ms, long_ms, gap_ms, encoding, trace \| touch[], state, timer, verilog}` → FSM description, per-tick state/timer trace, gesture events, Verilog, testbench, cost |
| `chip.iris_dsp` | `{project, arch?, adder?, pipeline?, encoding?, coeff_bits?, data_bits?, coeffs?}` → CIC spec, `FirGen` (CSD, RTL, testbench, cost), block costs, the totals, and the alternative architecture |
| `chip.tinytapeout` | same input → `{files[], design}` |
| `chip.csd` | `{values[]}` → CSD digits and non-zero counts |

### Metrics

The metrics cover the complete IRIS-DSP block: CIC + decimation FIR + gesture FSM + Tiny Tapeout glue.

| Key | Iris value (default decimating MAC, ripple adders) | Requirement |
|---|---|---|
| `chip.critical_path_ns` | **15.25 ns**, set by the 29-bit accumulator (the 16×12 multiplier stage is 14.75 ns) | R-015 ≤ 20 ns ✓ |
| `chip.est_cells` | **2044 cells**: CIC 582 + FIR 1234 + FSM 125 + glue 103. A pessimistic count, with 3 cells per adder bit, is 2896 | R-016 ≤ 3000 ✓ |

**The transposed form would fail both requirements.** The architecture is a design decision stored in `design.extra.chip_fir_arch`, and
it defaults to `"mac"`. With the project defaults (12-bit coefficients, 16-bit data, 0.25 ns gates), a **transposed-form** FIR fails
both requirements: **57.3 ns** (a 3-level CSD shift-add tree of 28-bit ripple adders plus one 29-bit accumulate adder) and
**3586 cells**. Pipelining with carry-lookahead adders meets timing (9.75 ns) but needs ~8600 cells. The output rate is 16 kHz against a
50 MHz clock, so a single multiplier time-shared over the 31 taps has 3125 cycles available per output and needs 35. The lab shows this
comparison live. A reviewer can switch the architecture and watch R-015 and R-016 go red.

**Cell model assumptions** (sky130-like standard cells):

- 1 cell per full-adder bit for ripple adders, ≈ 4 per bit for Brent–Kung carry-lookahead;
- 1 cell per flip-flop bit (enable flops included) and per 2:1-mux bit;
- an array multiplier costs D·B AND cells + D·(B−1) FA cells + D+B for Baugh–Wooley sign handling;
- a coefficient ROM costs about B·N/4 cells;
- control logic is a small constant.

**Timing model assumptions:**

- ripple adder ≈ bits × 2 × t_gate;
- carry-lookahead ≈ (2⌈log₂ bits⌉ + 2) × t_gate;
- array multiplier ≈ (B−1)·2·t_gate + the final adder;
- register overhead ≈ 3·t_gate.

**Cross-check with real tools** (while building this lab). Yosys `synth` with ABC mapped to generic 2-input gates gave:

| Design | Generic gates | Flops | Model flops |
|---|---|---|---|
| MAC FIR | 2167 | 609 | 608 |
| CIC | 1293 | 296 | 297 |
| FSM | 97 | 13 | 13 |
| Whole top level | 3706 | 953 | — |

Counting a full adder as ~5 generic gates, this agrees with the standard-cell model to within ~10–25%. The pessimistic figure bounds a
mapping that does not use full-adder cells.

## Verification of the generated RTL

- The Rust tests compare cycle-level models of both FIR architectures with the reference integer convolution. They cover the CSD
  shift-add transposed chain (with the rounding constant riding the delay line) and the rotating-buffer MAC with its reversed ROM order.
- Each generated testbench embeds the expected outputs computed in Rust:
  - `tb_iris_fir.v`: 80 decimated outputs from an impulse, a full-scale step, a tone and random samples.
  - `tb_iris_gesture_fsm.v`: the state after every one of 2800 ticks, plus the gesture pulse counts.
  - `tb_tt_um_iris_dsp.v`: PDM bits from the Rust sigma-delta modulator, with every PCM sample compared bit-exactly against the Rust CIC + FIR model.
- All three testbenches **pass in Icarus Verilog 12** for the MAC, transposed and pipelined-CLA/one-hot variants. `verilator --lint-only -Wall` is clean.

## The maths

- **Quine–McCluskey**: implicants are (value, mask) pairs, merged pairwise by popcount group until nothing merges. Essential primes are
  taken first; the rest are chosen by **Petrick's method**, expanding the product of sums as bitsets with absorption, then picking the
  fewest implicants and then the fewest literals. A greedy cover is the fallback above 4000 products. POS is the minimum SOP of F′
  under De Morgan.
- **K-maps**: rows use the first ⌊n/2⌋ variables and columns the rest, each in Gray order. The rows and columns an implicant covers
  are split into cyclic runs, so wrap-around groups become several rectangles.
- **Event-driven simulation**: 0/1/X values; a min-heap of (time, sequence) events; transport delays, so pulses shorter than a gate
  delay pass through, which is how glitches appear; rising-edge D flip-flops. Static timing is a longest-path relaxation in which
  flip-flops break paths.
- **FSM**: a data-driven Moore machine with guards on `touch`, `timer ≥ N` and `timer < N`. The first matching transition wins. The
  timer saturates and clears on every state change. The Verilog is Verilog-2001: `case` next-state logic in binary, or reverse-case
  `case (1'b1)` in one-hot.
- **CSD**: non-adjacent form, digits −1/0/+1, no two adjacent non-zeros, the minimum number of non-zeros. The Iris FIR needs 62 CSD
  digits against 74 binary ones. Multiplying by a constant with k non-zero digits costs k−1 adders in a ⌈log₂k⌉-deep tree.

## Iris gesture FSM

The FSM samples `touch` at 1 kHz.

- IDLE → PRESS on touch.
- PRESS → LONG_PRESS when held with timer ≥ 600.
- PRESS → TAP_WAIT when released with timer < 250.
- PRESS → IDLE on a slower release.
- TAP_WAIT → DOUBLE_TAP on a second touch.
- TAP_WAIT → TAP when timer ≥ 250.
- TAP → IDLE, and DOUBLE_TAP or LONG_PRESS → WAIT_RELEASE → IDLE on release.

The outputs `tap`, `double_tap` and `long_press` are one-tick Moore pulses.

## Tiny Tapeout package

`tt_um_iris_dsp` uses the standard ports. `ui_in[0]` is PDM data and `ui_in[1]` the PDM clock (the host drives the microphone clock and
mirrors it here); both pass through two-flop synchronisers and an edge detector. `ui_in[2]` is touch and `ui_in[3]` selects the output
byte. `uo_out` carries the PCM byte. `uio_out` carries the sample strobe, the gesture pulses and the FSM state, with `uio_oe = 8'hFF`.
A counter divides the system clock down to the 1 kHz tick.

The README tells the user to start from the current `tt*-verilog-template` repository and fill in `info.yaml` from it, rather than
inventing field names. Hardening is by OpenLane/OpenROAD in GitHub Actions or locally via tt-support-tools.

## Limitations

- The cell and timing figures come from a first-order model, not synthesis. There is no wire load, fan-out, setup/hold analysis or
  real liberty timing. Use Yosys + OpenSTA (and the TT hardening reports) for sign-off numbers.
- The CIC in the co-processor uses pipelined integrators, which delay the output by N−1 input samples relative to the DSP Lab's
  textbook CIC. The end-to-end testbench uses the matching model.
- The simulator has no strengths, tri-states or inertial delays, and each gate type has a fixed delay. It is meant for teaching-sized
  circuits (≤ 200 k events).
- The minimiser handles single-output functions only, not multi-output sharing. Exact Petrick can exhaust its budget on large cyclic
  functions, in which case the greedy fallback is reported as such.
- Only the Iris gesture FSM is generated. The data model supports other FSMs but the UI does not edit them.

## Graduating to professional tools

- **Yosys**: synthesis (`synth -top tt_um_iris_dsp; stat`), and `abc -liberty` with the sky130 library for real cell counts.
- **Icarus Verilog / Verilator**: event-driven and cycle-based simulation. Verilator `--lint-only -Wall` for lint.
- **cocotb**: Python testbenches, for example feeding real microphone recordings and checking against SciPy.
- **OpenLane / OpenROAD**: RTL-to-GDS on the SkyWater sky130 PDK, including static timing with OpenSTA.
- **Tiny Tapeout**: https://tinytapeout.com, plus https://github.com/TinyTapeout/tt-support-tools and the template repositories.

## References

- W. V. Quine, "The problem of simplifying truth functions", *Amer. Math. Monthly* 59, 1952.
- E. J. McCluskey, "Minimization of Boolean functions", *Bell Syst. Tech. J.* 35, 1956.
- S. R. Petrick, "A direct determination of the irredundant forms of a Boolean function from the set of prime implicants", AFCRC-TR-56-110, 1956.
- M. Karnaugh, "The map method for synthesis of combinational logic circuits", *Trans. AIEE* 72, 1953.
- E. F. Moore, "Gedanken-experiments on sequential machines", *Automata Studies*, 1956; G. H. Mealy, *Bell Syst. Tech. J.* 34, 1955.
- N. Weste & D. Harris, *CMOS VLSI Design: A Circuits and Systems Perspective*, 4th ed., Addison-Wesley 2011.
- K. K. Parhi, *VLSI Digital Signal Processing Systems*, Wiley 1999.
- G. W. Reitwiesner, "Binary arithmetic", *Advances in Computers* 1, 1960 (canonical signed digits).
- E. B. Hogenauer, "An economical class of digital filters for decimation and interpolation", *IEEE Trans. ASSP* 29(2), 1981.
- IEEE Std 1364-2005, Verilog HDL (scheduling semantics).
- Tiny Tapeout — https://tinytapeout.com
