// Teaching blocks for the Circuit Lab: the maths behind the simulator, each
// with a small live demo computed right here in the browser.
import { useMemo, useState } from 'react';
import { Explainer, Formula, Slider, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { si } from './util';

export const REFS = {
  nagel: { label: 'L. W. Nagel, "SPICE2: A Computer Program to Simulate Semiconductor Circuits", UC Berkeley ERL-M520, 1975' },
  ho: { label: 'C.-W. Ho, A. Ruehli, P. Brennan, "The modified nodal approach to network analysis", IEEE Trans. Circuits and Systems CAS-22(6), 1975' },
  sh: { label: 'H. Shichman & D. A. Hodges, "Modeling and simulation of insulated-gate field-effect transistor switching circuits", IEEE JSSC SC-3(3), 1968' },
  vlach: { label: 'J. Vlach & K. Singhal, Computer Methods for Circuit Analysis and Design, 2nd ed., Van Nostrand Reinhold, 1994' },
  ngspice: { label: 'ngspice User’s Manual', href: 'https://ngspice.sourceforge.io/docs.html' },
};

// ---------------------------------------------------------------------------
// MNA worked example
// ---------------------------------------------------------------------------

function fmtG(v: number) {
  if (v === 0) return '0';
  if (Math.abs(v) === 1) return v > 0 ? '1' : '−1';
  return si(v, 'S', 3).replace('-', '−');
}

export function MnaExplainer() {
  const [V, setV] = useState(1.8);
  const [R1, setR1] = useState(1000);
  const [R2, setR2] = useState(2000);
  const G1 = 1 / R1;
  const G2 = 1 / R2;
  const A = [
    [G1, -G1, 1],
    [-G1, G1 + G2, 0],
    [1, 0, 0],
  ];
  const b = [0, 0, V];
  const x = [V, (V * R2) / (R1 + R2), -V / (R1 + R2)];
  const cells = (row: number[]) => row.map((v, k) => <span key={k} className="cl-mcell">{fmtG(v)}</span>);
  return (
    <Explainer title="1 · Modified Nodal Analysis: every element adds a stamp" refs={[REFS.ho, REFS.vlach]}>
      <p>
        Kirchhoff's current law at every node, written with node voltages as unknowns, gives <i>G·v = i</i>. Voltage sources (and inductors, VCVS)
        cannot be written as a conductance, so MNA adds their <b>branch current</b> as an extra unknown and their <b>voltage equation</b> as an extra row.
        The simulator builds <i>A·x = b</i> by adding each element's small <i>stamp</i> and solves it with LU decomposition.
      </p>
      <Formula>{`resistor a–b:  A[a][a]+=G  A[b][b]+=G  A[a][b]−=G  A[b][a]−=G       (G = 1/R)
V source k:    A[a][k]+=1  A[b][k]−=1  A[k][a]+=1  A[k][b]−=1   b[k] = V
current I a→b: b[a] −= I   b[b] += I`}</Formula>
      <p className="small muted">Worked example — V1 drives node 1, R1 links nodes 1–2, R2 goes from node 2 to ground. Drag the values:</p>
      <div className="grid grid-3" style={{ gap: 10 }}>
        <Slider label="V1" value={V} min={0.5} max={5} step={0.1} unit="V" onChange={setV} />
        <Slider label="R1" value={R1} min={100} max={100000} log unit="Ω" format={(v) => si(v, '')} onChange={setR1} />
        <Slider label="R2" value={R2} min={100} max={100000} log unit="Ω" format={(v) => si(v, '')} onChange={setR2} />
      </div>
      <div className="cl-matrix">
        <div className="cl-mat">
          <div className="cl-mhead"><span>v1</span><span>v2</span><span>i(V1)</span></div>
          {A.map((r, k) => <div key={k} className="cl-mrow">{cells(r)}</div>)}
        </div>
        <div className="cl-mop">·</div>
        <div className="cl-mat col"><div className="cl-mhead"><span>x</span></div>{['v1', 'v2', 'i(V1)'].map((n) => <div key={n} className="cl-mrow"><span className="cl-mcell">{n}</span></div>)}</div>
        <div className="cl-mop">=</div>
        <div className="cl-mat col"><div className="cl-mhead"><span>b</span></div>{b.map((v, k) => <div key={k} className="cl-mrow"><span className="cl-mcell">{k === 2 ? si(v, 'V') : '0'}</span></div>)}</div>
        <div className="cl-mop">⇒</div>
        <div className="cl-mat col"><div className="cl-mhead"><span>x =</span></div>
          <div className="cl-mrow"><span className="cl-mcell">{si(x[0], 'V', 4)}</span></div>
          <div className="cl-mrow"><span className="cl-mcell">{si(x[1], 'V', 4)}</span></div>
          <div className="cl-mrow"><span className="cl-mcell">{si(x[2], 'A', 4)}</span></div>
        </div>
      </div>
      <p className="small">
        The source current comes out <b>negative</b>: SPICE measures it flowing <i>into</i> the + terminal, so a source that delivers power reports I &lt; 0 —
        exactly what the operating-point table shows.
      </p>
    </Explainer>
  );
}

// ---------------------------------------------------------------------------
// Newton–Raphson with junction limiting
// ---------------------------------------------------------------------------

const VT = 0.025865;

function pnjlim(vnew: number, vold: number, vte: number, vcrit: number): number {
  if (vnew > vcrit && Math.abs(vnew - vold) > 2 * vte) {
    if (vold > 0) {
      const arg = 1 + (vnew - vold) / vte;
      return arg > 0 ? vold + vte * Math.log(arg) : vcrit;
    }
    return vte * Math.log(vnew / vte);
  }
  return vnew;
}

function newtonDiode(V: number, R: number, IS: number, limit: boolean) {
  const vcrit = VT * Math.log(VT / (Math.SQRT2 * IS));
  const out: { k: number; vd: number; id: number }[] = [];
  let vd = 0; // voltage where the diode is evaluated
  for (let k = 0; k < 1000; k++) {
    const e = Math.exp(Math.min(vd / VT, 700));
    const id = IS * (e - 1);
    const gd = (IS * e) / VT;
    out.push({ k, vd, id });
    // companion model: diode = gd in parallel with Ieq = id − gd·vd ; solve the 1-node circuit
    const ieq = id - gd * vd;
    const vnew = (V / R - ieq) / (1 / R + gd);
    if (Math.abs(vnew - vd) < 1e-9 + 1e-6 * Math.abs(vnew)) {
      out.push({ k: k + 1, vd: vnew, id: IS * (Math.exp(vnew / VT) - 1) });
      break;
    }
    vd = limit ? pnjlim(vnew, vd, VT, vcrit) : vnew;
  }
  return out;
}

export function NewtonExplainer() {
  const [limit, setLimit] = useState(true);
  const [V, setV] = useState(5);
  const withL = useMemo(() => newtonDiode(V, 1000, 1e-14, true), [V]);
  const without = useMemo(() => newtonDiode(V, 1000, 1e-14, false), [V]);
  const shown = limit ? withL : without;
  return (
    <Explainer title="2 · Newton–Raphson and junction limiting" refs={[REFS.nagel, REFS.vlach]}>
      <p>
        A diode's current is exponential, so the MNA equations are nonlinear. Newton–Raphson replaces the diode, at the present guess <i>V<sub>k</sub></i>,
        by its tangent — a conductance and a current source (the <b>companion model</b>) — solves the now-linear circuit, and repeats until the answer stops moving.
      </p>
      <Formula>{`g_d = (I_S / nV_T)·e^(V_k/nV_T)        I_eq = I_d(V_k) − g_d·V_k
x_(k+1) = A(x_k)⁻¹ · b(x_k)          converged when |Δx| ≤ reltol·|x| + vntol  (1e-3, 1 µV)
pnjlim:  if V_new > V_crit:  V_new = V_old + nV_T·ln(1 + (V_new − V_old)/nV_T)`}</Formula>
      <p>
        From 0 V, the first tangent predicts the diode sees the whole supply. Evaluating <i>e</i><sup>V/V<sub>T</sub></sup> at {V} V gives ≈10<sup>{Math.round(V / VT / Math.LN10)}</sup> A;
        plain Newton then crawls back one <i>V<sub>T</sub></i> per iteration. SPICE's <b>pnjlim</b> limits each step logarithmically:
        <b> {withL.length - 1} iterations</b> instead of <b>{without.length - 1}</b>. (If Newton still fails, the simulator falls back to gmin stepping, then source stepping.)
      </p>
      <div className="grid grid-2" style={{ gap: 10 }}>
        <Slider label="Supply V (1 kΩ + diode, IS = 10 fA)" value={V} min={1} max={12} step={0.5} unit="V" onChange={setV} />
        <Toggle label="Use pnjlim junction limiting" value={limit} onChange={setLimit} />
      </div>
      <Plot
        height={200}
        series={[
          { name: 'with limiting', x: withL.map((p) => p.k), y: withL.map((p) => p.vd), width: limit ? 2.5 : 1.2 },
          { name: 'plain Newton', x: without.map((p) => p.k), y: without.map((p) => p.vd), dash: '5 4', width: limit ? 1.2 : 2.5 },
        ]}
        xLabel="iteration"
        yLabel="diode voltage evaluated (V)"
        formatX={(v) => v.toFixed(0)}
      />
      <div className="small muted">Converged: V<sub>d</sub> = {shown[shown.length - 1].vd.toFixed(4)} V, I<sub>d</sub> = {si(shown[shown.length - 1].id, 'A')}.</div>
    </Explainer>
  );
}

// ---------------------------------------------------------------------------
// Transient integration
// ---------------------------------------------------------------------------

export function TransientExplainer() {
  const [h, setH] = useState(0.5);
  const data = useMemo(() => {
    const T = 8;
    const tx: number[] = [];
    const ex: number[] = [];
    for (let k = 0; k <= 400; k++) {
      const t = (T * k) / 400;
      tx.push(t);
      ex.push(1 - Math.exp(-t));
    }
    const ts: number[] = [0];
    const be: number[] = [0];
    const tr: number[] = [0];
    let vb = 0, vt = 0;
    for (let t = h; t <= T + 1e-9; t += h) {
      vb = (vb + h) / (1 + h); // (C/h)(v⁺ − v) = (1 − v⁺)/R, τ = 1
      vt = (vt * (1 - h / 2) + h) / (1 + h / 2);
      ts.push(t);
      be.push(vb);
      tr.push(vt);
    }
    return { tx, ex, ts, be, tr };
  }, [h]);
  const err = (arr: number[]) => Math.max(...arr.map((v, k) => Math.abs(v - (1 - Math.exp(-data.ts[k])))));
  return (
    <Explainer title="3 · Transient: companion models and why the step size matters" refs={[REFS.nagel, REFS.vlach]}>
      <p>
        Each time step turns a capacitor into a conductance plus a current source built from its history. The simulator uses the second-order
        <b> trapezoidal</b> rule, and <b>backward Euler</b> for the first step and right after every source corner (breakpoint), where the trapezoidal
        rule would ring. Step size is chosen from the <b>local truncation error</b> (≈ h³·v‴/12), estimated by comparing each solution with a
        quadratic prediction; steps that are too inaccurate or fail to converge are rejected and retried smaller.
      </p>
      <Formula>{`backward Euler:  i₊ = (C/h)·v₊ − (C/h)·v                     (1st order, very damped)
trapezoidal:     i₊ = (2C/h)·v₊ − [(2C/h)·v + i]            (2nd order, can ring if h ≫ τ)
inductor (trap): v₊ = (2L/h)·i₊ − [(2L/h)·i + v]`}</Formula>
      <Slider label="Step h relative to the RC time constant τ" value={h} min={0.05} max={4} step={0.05} unit="× τ" onChange={setH} />
      <Plot
        height={210}
        series={[
          { name: 'exact 1 − e^(−t/τ)', x: data.tx, y: data.ex, width: 2 },
          { name: `trapezoidal (max err ${(err(data.tr) * 100).toFixed(1)} %)`, x: data.ts, y: data.tr, width: 2 },
          { name: `backward Euler (max err ${(err(data.be) * 100).toFixed(1)} %)`, x: data.ts, y: data.be, width: 2, dash: '5 4' },
        ]}
        xLabel="t / τ"
        yLabel="v / V"
        formatX={(v) => v.toFixed(1)}
      />
      <p className="small muted">Push h past 2τ: the trapezoidal answer overshoots and oscillates around the true curve — numerical ringing — while backward Euler stays smooth but lags.</p>
    </Explainer>
  );
}

// ---------------------------------------------------------------------------
// AC and devices
// ---------------------------------------------------------------------------

export function AcExplainer() {
  return (
    <Explainer title="4 · AC analysis: linearise, then sweep jω" refs={[REFS.nagel, REFS.ngspice]}>
      <p>
        AC analysis first finds the DC operating point, then replaces every nonlinear device by its small-signal model at that point (a diode by
        <i> g<sub>d</sub></i>, a MOSFET by <i>g<sub>m</sub></i>, <i>g<sub>ds</sub></i>), capacitors by <i>jωC</i> and inductors by <i>jωL</i>, and solves the complex MNA
        system once per frequency. The result is exact for small signals: magnitude in dB and phase, the Bode plot.
      </p>
      <Formula>{`Y_C = jωC        Z_L = jωL        |H|_dB = 20·log10|V_out/V_in|        f_−3dB : |H| = |H(0)|/√2`}</Formula>
    </Explainer>
  );
}

export function DevicesExplainer() {
  return (
    <Explainer title="5 · Device models used here" refs={[REFS.sh, REFS.nagel, REFS.ngspice]}>
      <Formula>{`diode:   I = I_S·(e^(V/nV_T) − 1),  V_T = kT/q = 25.85 mV at 27 °C,  + series R_S, C_J(V) and TT·I charge
MOSFET level 1 (Shichman–Hodges), β = KP·W/L:
  cutoff    V_GS ≤ V_T:                 I_D = 0
  triode    V_DS < V_GS − V_T:          I_D = β·(V_GS − V_T − V_DS/2)·V_DS·(1 + λV_DS)
  saturation:                            I_D = (β/2)·(V_GS − V_T)²·(1 + λV_DS)
switch (SW): R = RON if V_c > VT + VH, ROFF if V_c < VT − VH, else unchanged (hysteresis)`}</Formula>
      <p className="small">
        Level 1 is the textbook square law: good for intuition and hand calculations, not for modern short-channel transistors (use BSIM models in ngspice for that).
      </p>
    </Explainer>
  );
}
