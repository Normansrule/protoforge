//! Modified Nodal Analysis (MNA): matrix assembly ("stamping") and the
//! Newton–Raphson loop.
//!
//! Unknowns `x = [v1 … vN, j1 … jB]`: node voltages (ground removed) plus one
//! branch current for every element whose current cannot be written as a
//! function of node voltages (V sources, inductors, VCVS). Each element adds
//! its *stamp* to `A x = b`:
//!
//! ```text
//!   conductance g between a,b      voltage source V (branch k) a(+) b(−)
//!        a    b                          a    b    k
//!   a [ +g   −g ]                   a  [           +1 ]      b_k = V
//!   b [ −g   +g ]                   b  [           −1 ]
//!                                   k  [ +1   −1      ]
//! ```
//!
//! Nonlinear devices are replaced every iteration by their linearisation
//! (a conductance plus a current source: the Newton–Raphson "companion
//! model"); reactive elements in a transient by their integration companion
//! (trapezoidal or backward Euler).
//!
//! References: C.-W. Ho, A. E. Ruehli, P. A. Brennan, "The modified nodal
//! approach to network analysis", IEEE Trans. Circuits and Systems CAS-22(6),
//! 1975; J. Vlach and K. Singhal, *Computer Methods for Circuit Analysis and
//! Design*, 2nd ed., Van Nostrand Reinhold, 1994.

use crate::math::{Field, C64};

use super::devices::{self, diode_eval, fetlim, limvds, mos_eval, pnjlim, source_value, switch_state};
use super::parser::{Device, DiodeModel, MosModel, Netlist, Options, Source, SwModel};

/// Conductance used to pin a node to an `.ic` value / a capacitor to its IC.
pub const G_FORCE: f64 = 1e6;

/// One compiled element. Node numbers index the solution (0 = ground).
#[derive(Clone, Debug)]
pub enum CDev {
    R {
        a: usize,
        b: usize,
        g: f64,
    },
    C {
        a: usize,
        b: usize,
        c: f64,
        ic: Option<f64>,
    },
    L {
        a: usize,
        b: usize,
        l: f64,
        br: usize,
        ic: Option<f64>,
    },
    V {
        a: usize,
        b: usize,
        br: usize,
        src: Source,
    },
    I {
        a: usize,
        b: usize,
        src: Source,
    },
    D {
        a: usize,
        ai: usize,
        k: usize,
        m: DiodeModel,
        area: f64,
        gs: f64,
        vcrit: f64,
    },
    M {
        d: usize,
        g: usize,
        s: usize,
        b: usize,
        m: MosModel,
        w: f64,
        l: f64,
        mult: f64,
        caps: [(usize, usize, f64); 4],
    },
    E {
        a: usize,
        b: usize,
        ca: usize,
        cb: usize,
        gain: f64,
        br: usize,
    },
    G {
        a: usize,
        b: usize,
        ca: usize,
        cb: usize,
        gm: f64,
    },
    S {
        a: usize,
        b: usize,
        ca: usize,
        cb: usize,
        m: SwModel,
        init_on: bool,
    },
}

/// A compiled circuit ready for analysis.
#[derive(Clone, Debug)]
pub struct Ckt {
    /// Total nodes including ground and internal nodes (diode RS).
    pub nn: usize,
    /// Number of user-visible nodes (including ground) = netlist.nodes.len().
    pub n_user: usize,
    pub nbr: usize,
    pub size: usize,
    pub el: Vec<CDev>,
    pub names: Vec<String>,
    pub linear: bool,
    pub opts: Options,
    pub ics: Vec<(usize, f64)>,
    pub nodesets: Vec<(usize, f64)>,
}

impl Ckt {
    pub fn compile(nl: &Netlist) -> Ckt {
        let mut nn = nl.nodes.len();
        let mut nbr = 0;
        let mut el = Vec::with_capacity(nl.elements.len());
        let mut linear = true;
        for e in &nl.elements {
            let n = &e.nodes;
            let d = match &e.dev {
                Device::Resistor { r } => CDev::R {
                    a: n[0],
                    b: n[1],
                    g: 1.0 / r,
                },
                Device::Capacitor { c, ic } => CDev::C {
                    a: n[0],
                    b: n[1],
                    c: *c,
                    ic: *ic,
                },
                Device::Inductor { l, ic } => {
                    nbr += 1;
                    CDev::L {
                        a: n[0],
                        b: n[1],
                        l: *l,
                        br: nbr - 1,
                        ic: *ic,
                    }
                }
                Device::Vsource { src } => {
                    nbr += 1;
                    CDev::V {
                        a: n[0],
                        b: n[1],
                        br: nbr - 1,
                        src: src.clone(),
                    }
                }
                Device::Isource { src } => CDev::I {
                    a: n[0],
                    b: n[1],
                    src: src.clone(),
                },
                Device::Diode { model, area, .. } => {
                    linear = false;
                    let rs = model.rs / area.max(1e-30);
                    let ai = if rs > 0.0 {
                        nn += 1;
                        nn - 1
                    } else {
                        n[0]
                    };
                    CDev::D {
                        a: n[0],
                        ai,
                        k: n[1],
                        m: model.clone(),
                        area: *area,
                        gs: if rs > 0.0 { 1.0 / rs } else { 0.0 },
                        vcrit: devices::vcrit(model, *area),
                    }
                }
                Device::Mosfet { model, w, l, mult, .. } => {
                    linear = false;
                    let (dn, gn, sn, bn) = (n[0], n[1], n[2], n[3]);
                    let caps = [
                        (gn, sn, model.cgso * w * mult),
                        (gn, dn, model.cgdo * w * mult),
                        (bn, dn, model.cbd * mult),
                        (bn, sn, model.cbs * mult),
                    ];
                    CDev::M {
                        d: dn,
                        g: gn,
                        s: sn,
                        b: bn,
                        m: model.clone(),
                        w: *w,
                        l: *l,
                        mult: *mult,
                        caps,
                    }
                }
                Device::Vcvs { gain } => {
                    nbr += 1;
                    CDev::E {
                        a: n[0],
                        b: n[1],
                        ca: n[2],
                        cb: n[3],
                        gain: *gain,
                        br: nbr - 1,
                    }
                }
                Device::Vccs { gm } => CDev::G {
                    a: n[0],
                    b: n[1],
                    ca: n[2],
                    cb: n[3],
                    gm: *gm,
                },
                Device::Switch { model, on, .. } => {
                    linear = false;
                    CDev::S {
                        a: n[0],
                        b: n[1],
                        ca: n[2],
                        cb: n[3],
                        m: model.clone(),
                        init_on: *on,
                    }
                }
            };
            el.push(d);
        }
        // branch indices come after all node unknowns
        let nv = nn - 1;
        for d in el.iter_mut() {
            match d {
                CDev::L { br, .. } | CDev::V { br, .. } | CDev::E { br, .. } => *br += nv,
                _ => {}
            }
        }
        Ckt {
            nn,
            n_user: nl.nodes.len(),
            nbr,
            size: nv + nbr,
            el,
            names: nl.elements.iter().map(|e| e.name.clone()).collect(),
            linear,
            opts: nl.options.clone(),
            ics: nl.ics.clone(),
            nodesets: nl.nodesets.clone(),
        }
    }
}

/// Map a node number to its row/column (ground has none).
#[inline]
pub fn ix(n: usize) -> Option<usize> {
    if n == 0 {
        None
    } else {
        Some(n - 1)
    }
}

/// Voltage of node `n` in solution `x`.
#[inline]
pub fn vx(x: &[f64], n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        x[n - 1]
    }
}

/// Dense MNA system `A x = b`, row-major in one flat buffer.
pub struct Sys<T: Field> {
    pub n: usize,
    pub a: Vec<T>,
    pub b: Vec<T>,
}

impl<T: Field> Sys<T> {
    pub fn new(n: usize) -> Self {
        Sys {
            n,
            a: vec![T::zero(); n * n],
            b: vec![T::zero(); n],
        }
    }
    pub fn clear(&mut self) {
        self.a.iter_mut().for_each(|v| *v = T::zero());
        self.b.iter_mut().for_each(|v| *v = T::zero());
    }
    /// Row-major rows, for `crate::math::solve`.
    pub fn rows(&self) -> Vec<Vec<T>> {
        self.a.chunks(self.n.max(1)).map(|r| r.to_vec()).collect()
    }
    #[inline]
    pub fn add(&mut self, r: Option<usize>, c: Option<usize>, v: T) {
        if let (Some(r), Some(c)) = (r, c) {
            let k = r * self.n + c;
            self.a[k] = self.a[k] + v;
        }
    }
    #[inline]
    pub fn diag(&mut self, i: usize, v: T) {
        let k = i * self.n + i;
        self.a[k] = self.a[k] + v;
    }
    #[inline]
    pub fn rhs(&mut self, r: Option<usize>, v: T) {
        if let Some(r) = r {
            self.b[r] = self.b[r] + v;
        }
    }
    /// Conductance `g` between nodes p and n.
    pub fn cond(&mut self, p: usize, n: usize, g: T) {
        let (pi, ni) = (ix(p), ix(n));
        self.add(pi, pi, g);
        self.add(ni, ni, g);
        self.add(pi, ni, -g);
        self.add(ni, pi, -g);
    }
    /// Constant current `i` flowing from p through the element to n.
    pub fn cur(&mut self, p: usize, n: usize, i: T) {
        self.rhs(ix(p), -i);
        self.rhs(ix(n), i);
    }
    /// Transconductance: current gm·(v_cp − v_cn) flowing from p to n.
    pub fn vccs(&mut self, p: usize, n: usize, cp: usize, cn: usize, gm: T) {
        let (pi, ni, cpi, cni) = (ix(p), ix(n), ix(cp), ix(cn));
        self.add(pi, cpi, gm);
        self.add(pi, cni, -gm);
        self.add(ni, cpi, -gm);
        self.add(ni, cni, gm);
    }
    /// Branch-current unknown `br` entering node a and leaving node b (KCL
    /// incidence) plus the branch-row voltage terms `v_a − v_b`.
    pub fn branch(&mut self, a: usize, b: usize, br: usize, one: T) {
        let (ai, bi) = (ix(a), ix(b));
        self.add(ai, Some(br), one);
        self.add(bi, Some(br), -one);
        self.add(Some(br), ai, one);
        self.add(Some(br), bi, -one);
    }
}

/// Charge/flux history of one reactive branch.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dyn {
    /// Charge (capacitor) or flux linkage (inductor) at the last time point.
    pub q: f64,
    /// Its time derivative: current (capacitor) or voltage (inductor).
    pub i: f64,
}

/// Accepted per-element state carried between time points.
#[derive(Clone, Copy, Debug, Default)]
pub struct ElState {
    pub dy: [Dyn; 4],
    pub on: bool,
}

/// Per-iteration memory: last junction voltages (for limiting) and switch
/// states within the current Newton solve.
#[derive(Clone, Copy, Debug, Default)]
pub struct IterMem {
    pub v: [f64; 3],
    pub init: bool,
    pub on: bool,
    pub flips: u32,
}

/// What the matrix represents.
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    /// DC operating point. `t = Some(t0)` evaluates source waveforms at t0
    /// (transient initial point), `None` uses DC values. `uic_init` replaces
    /// capacitors by their IC voltage and inductors by their IC current.
    Dc { t: Option<f64>, uic_init: bool, force_ic: bool },
    /// Transient step to time `t`: reactive elements use `i = a0·q − hist`.
    Tran { t: f64, a0: f64, trap: bool },
}

/// Integration history term for a reactive branch.
#[inline]
pub fn hist(d: &Dyn, a0: f64, trap: bool) -> f64 {
    a0 * d.q + if trap { d.i } else { 0.0 }
}

pub struct Load<'a> {
    pub mode: Mode,
    pub st: &'a [ElState],
    pub srcfac: f64,
    pub gshunt: f64,
}

impl Ckt {
    pub fn src_value(&self, s: &Source, mode: Mode) -> f64 {
        match mode {
            Mode::Dc { t: None, .. } => s.dc,
            Mode::Dc { t: Some(t), .. } | Mode::Tran { t, .. } => source_value(s, t),
        }
    }

    /// Initial-condition voltage for a capacitor between a and b under UIC.
    pub fn uic_cap_v(&self, a: usize, b: usize, ic: Option<f64>) -> f64 {
        if let Some(v) = ic {
            return v;
        }
        let f = |n: usize| self.ics.iter().find(|(k, _)| *k == n).map(|(_, v)| *v).unwrap_or(0.0);
        f(a) - f(b)
    }

    /// Assemble the linearised MNA system at solution guess `x`. Returns
    /// `true` when any device limited its voltages or changed state (so the
    /// iteration cannot be declared converged yet).
    pub fn load(&self, x: &[f64], ld: &Load, mem: &mut [IterMem], sys: &mut Sys<f64>) -> bool {
        let gmin = self.opts.gmin;
        let mut noncon = false;
        let (tran, a0, trap) = match ld.mode {
            Mode::Tran { a0, trap, .. } => (true, a0, trap),
            _ => (false, 0.0, false),
        };
        let uic_init = matches!(ld.mode, Mode::Dc { uic_init: true, .. });
        for (k, d) in self.el.iter().enumerate() {
            let st = &ld.st[k];
            match d {
                CDev::R { a, b, g } => sys.cond(*a, *b, *g),
                CDev::C { a, b, c, ic } => {
                    if tran {
                        let geq = a0 * c;
                        sys.cond(*a, *b, geq);
                        sys.cur(*a, *b, -hist(&st.dy[0], a0, trap));
                    } else if uic_init {
                        let v0 = self.uic_cap_v(*a, *b, *ic);
                        sys.cond(*a, *b, G_FORCE);
                        sys.cur(*a, *b, -G_FORCE * v0);
                    }
                }
                CDev::L { a, b, l, br, ic } => {
                    let bri = Some(*br);
                    if uic_init {
                        sys.add(ix(*a), bri, 1.0);
                        sys.add(ix(*b), bri, -1.0);
                        sys.add(bri, bri, 1.0);
                        sys.rhs(bri, ic.unwrap_or(0.0));
                    } else {
                        sys.branch(*a, *b, *br, 1.0);
                        if tran {
                            sys.add(bri, bri, -a0 * l);
                            sys.rhs(bri, -hist(&st.dy[0], a0, trap));
                        }
                    }
                }
                CDev::V { a, b, br, src } => {
                    sys.branch(*a, *b, *br, 1.0);
                    sys.rhs(Some(*br), self.src_value(src, ld.mode) * ld.srcfac);
                }
                CDev::I { a, b, src } => sys.cur(*a, *b, self.src_value(src, ld.mode) * ld.srcfac),
                CDev::E { a, b, ca, cb, gain, br } => {
                    sys.branch(*a, *b, *br, 1.0);
                    sys.add(Some(*br), ix(*ca), -gain);
                    sys.add(Some(*br), ix(*cb), *gain);
                }
                CDev::G { a, b, ca, cb, gm } => sys.vccs(*a, *b, *ca, *cb, *gm),
                CDev::S { a, b, ca, cb, m, init_on } => {
                    let vc = vx(x, *ca) - vx(x, *cb);
                    let mm = &mut mem[k];
                    if !mm.init {
                        mm.init = true;
                        mm.on = if tran { st.on } else { *init_on };
                        mm.flips = 0;
                    }
                    let prev = if tran { st.on } else { mm.on };
                    let s = switch_state(m, vc, prev);
                    if s != mm.on {
                        if mm.flips < 8 {
                            mm.on = s;
                            mm.flips += 1;
                        }
                        noncon = true;
                    }
                    sys.cond(*a, *b, if mm.on { 1.0 / m.ron } else { 1.0 / m.roff });
                }
                CDev::D {
                    a,
                    ai,
                    k: kk,
                    m,
                    area,
                    gs,
                    vcrit,
                } => {
                    if *gs > 0.0 {
                        sys.cond(*a, *ai, *gs);
                    }
                    let vte = m.n * devices::vt();
                    let mut vd = vx(x, *ai) - vx(x, *kk);
                    let mm = &mut mem[k];
                    if mm.init {
                        let (v, lim) = pnjlim(vd, mm.v[0], vte, *vcrit);
                        vd = v;
                        noncon |= lim;
                    } else {
                        mm.init = true;
                        // start a forward-biased junction at a sane value
                        if vd > *vcrit {
                            vd = *vcrit;
                        }
                    }
                    mm.v[0] = vd;
                    let op = diode_eval(m, *area, vd);
                    let mut g = op.gd + gmin;
                    let mut i = op.id + gmin * vd;
                    if tran && op.c > 0.0 {
                        let geq = a0 * op.c;
                        let ic = a0 * op.q - hist(&st.dy[0], a0, trap);
                        g += geq;
                        i += ic;
                    }
                    sys.cond(*ai, *kk, g);
                    sys.cur(*ai, *kk, i - g * vd);
                }
                CDev::M {
                    d: dn,
                    g: gn,
                    s: sn,
                    b: bn,
                    m,
                    w,
                    l,
                    mult,
                    caps,
                } => {
                    let p = if m.pmos { -1.0 } else { 1.0 };
                    let vs = vx(x, *sn);
                    let mut vgs = p * (vx(x, *gn) - vs);
                    let mut vds = p * (vx(x, *dn) - vs);
                    let vbs = p * (vx(x, *bn) - vs);
                    let vto = p * m.vto;
                    let mm = &mut mem[k];
                    if mm.init {
                        let (ogs, ods) = (mm.v[0], mm.v[1]);
                        if ods >= 0.0 {
                            let (a1, l1) = fetlim(vgs, ogs, vto);
                            vgs = a1;
                            let (a2, l2) = limvds(vds, ods);
                            vds = a2;
                            noncon |= l1 || l2;
                        } else {
                            let vgd = vgs - vds;
                            let (a1, l1) = fetlim(vgd, ogs - ods, vto);
                            let (a2, l2) = limvds(-vds, -ods);
                            vds = -a2;
                            vgs = a1 + vds;
                            noncon |= l1 || l2;
                        }
                    } else {
                        mm.init = true;
                    }
                    mm.v = [vgs, vds, vbs];
                    let tv = [vs + p * vds, vs + p * vgs, vs, vs + p * vbs];
                    let op = mos_eval(m, *w, *l, *mult, tv);
                    let nodes = [*dn, *gn, *sn, *bn];
                    let mut ieq = op.id;
                    for t in 0..4 {
                        ieq -= op.g[t] * tv[t];
                        sys.add(ix(*dn), ix(nodes[t]), op.g[t]);
                        sys.add(ix(*sn), ix(nodes[t]), -op.g[t]);
                    }
                    sys.cur(*dn, *sn, ieq);
                    sys.cond(*dn, *sn, gmin);
                    if tran {
                        for (ci, (na, nb, c)) in caps.iter().enumerate() {
                            if *c > 0.0 {
                                sys.cond(*na, *nb, a0 * c);
                                sys.cur(*na, *nb, -hist(&st.dy[ci], a0, trap));
                            }
                        }
                    }
                }
            }
        }
        if let Mode::Dc { force_ic: true, .. } = ld.mode {
            for (n, v) in &self.ics {
                sys.cond(*n, 0, G_FORCE);
                sys.cur(*n, 0, -G_FORCE * v);
            }
        }
        if ld.gshunt > 0.0 {
            for i in 0..self.nn - 1 {
                sys.diag(i, ld.gshunt);
            }
        }
        noncon
    }

    /// Small-signal complex system at angular frequency `w`, linearised at the
    /// operating point `x` (with switch states from `mem`).
    pub fn load_ac(&self, x: &[f64], mem: &[IterMem], w: f64, sys: &mut Sys<C64>) {
        let gmin = self.opts.gmin;
        let jw = C64::new(0.0, w);
        for (k, d) in self.el.iter().enumerate() {
            match d {
                CDev::R { a, b, g } => sys.cond(*a, *b, C64::real(*g)),
                CDev::C { a, b, c, .. } => sys.cond(*a, *b, jw * *c),
                CDev::L { a, b, l, br, .. } => {
                    sys.branch(*a, *b, *br, C64::ONE);
                    sys.add(Some(*br), Some(*br), -(jw * *l));
                }
                CDev::V { a, b, br, src } => {
                    sys.branch(*a, *b, *br, C64::ONE);
                    sys.rhs(Some(*br), C64::from_polar(src.ac_mag, src.ac_phase_deg.to_radians()));
                }
                CDev::I { a, b, src } => sys.cur(*a, *b, C64::from_polar(src.ac_mag, src.ac_phase_deg.to_radians())),
                CDev::E { a, b, ca, cb, gain, br } => {
                    sys.branch(*a, *b, *br, C64::ONE);
                    sys.add(Some(*br), ix(*ca), C64::real(-gain));
                    sys.add(Some(*br), ix(*cb), C64::real(*gain));
                }
                CDev::G { a, b, ca, cb, gm } => sys.vccs(*a, *b, *ca, *cb, C64::real(*gm)),
                CDev::S { a, b, m, .. } => sys.cond(*a, *b, C64::real(if mem[k].on { 1.0 / m.ron } else { 1.0 / m.roff })),
                CDev::D {
                    a, ai, k: kk, m, area, gs, ..
                } => {
                    if *gs > 0.0 {
                        sys.cond(*a, *ai, C64::real(*gs));
                    }
                    let vd = vx(x, *ai) - vx(x, *kk);
                    let op = diode_eval(m, *area, vd);
                    sys.cond(*ai, *kk, C64::new(op.gd + gmin, w * op.c));
                }
                CDev::M {
                    d: dn,
                    g: gn,
                    s: sn,
                    b: bn,
                    m,
                    w: ww,
                    l,
                    mult,
                    caps,
                } => {
                    let tv = [vx(x, *dn), vx(x, *gn), vx(x, *sn), vx(x, *bn)];
                    let op = mos_eval(m, *ww, *l, *mult, tv);
                    let nodes = [*dn, *gn, *sn, *bn];
                    for (node, g) in nodes.iter().zip(op.g) {
                        sys.add(ix(*dn), ix(*node), C64::real(g));
                        sys.add(ix(*sn), ix(*node), C64::real(-g));
                    }
                    sys.cond(*dn, *sn, C64::real(gmin));
                    for (na, nb, c) in caps {
                        if *c > 0.0 {
                            sys.cond(*na, *nb, jw * *c);
                        }
                    }
                }
            }
        }
    }

    /// Current through every element (SPICE sign conventions: into `n+`
    /// through the element to `n−`; MOSFET: into the drain) for solution `x`.
    pub fn element_currents(&self, x: &[f64], ld: &Load, mem: &[IterMem]) -> Vec<f64> {
        let (tran, a0, trap) = match ld.mode {
            Mode::Tran { a0, trap, .. } => (true, a0, trap),
            _ => (false, 0.0, false),
        };
        let gmin = self.opts.gmin;
        self.el
            .iter()
            .enumerate()
            .map(|(k, d)| match d {
                CDev::R { a, b, g } => (vx(x, *a) - vx(x, *b)) * g,
                CDev::C { a, b, c, .. } => {
                    if tran {
                        a0 * c * (vx(x, *a) - vx(x, *b)) - hist(&ld.st[k].dy[0], a0, trap)
                    } else {
                        0.0
                    }
                }
                CDev::L { br, .. } | CDev::V { br, .. } | CDev::E { br, .. } => x[*br],
                CDev::I { src, .. } => self.src_value(src, ld.mode) * ld.srcfac,
                CDev::G { ca, cb, gm, .. } => gm * (vx(x, *ca) - vx(x, *cb)),
                CDev::S { a, b, m, .. } => (vx(x, *a) - vx(x, *b)) / if mem[k].on { m.ron } else { m.roff },
                CDev::D { ai, k: kk, m, area, .. } => {
                    let vd = vx(x, *ai) - vx(x, *kk);
                    let op = diode_eval(m, *area, vd);
                    let mut i = op.id + gmin * vd;
                    if tran && op.c > 0.0 {
                        i += a0 * op.q - hist(&ld.st[k].dy[0], a0, trap);
                    }
                    i
                }
                CDev::M {
                    d: dn,
                    g: gn,
                    s: sn,
                    b: bn,
                    m,
                    w,
                    l,
                    mult,
                    ..
                } => {
                    let op = mos_eval(m, *w, *l, *mult, [vx(x, *dn), vx(x, *gn), vx(x, *sn), vx(x, *bn)]);
                    op.id + gmin * (vx(x, *dn) - vx(x, *sn))
                }
            })
            .collect()
    }

    /// Charges of every reactive branch at solution `x` (for committing a
    /// time point). Returns per element the 4 charge slots.
    pub fn charges(&self, x: &[f64]) -> Vec<[f64; 4]> {
        self.el
            .iter()
            .map(|d| match d {
                CDev::C { a, b, c, .. } => [c * (vx(x, *a) - vx(x, *b)), 0.0, 0.0, 0.0],
                CDev::L { l, br, .. } => [l * x[*br], 0.0, 0.0, 0.0],
                CDev::D { ai, k, m, area, .. } => [diode_eval(m, *area, vx(x, *ai) - vx(x, *k)).q, 0.0, 0.0, 0.0],
                CDev::M { caps, .. } => {
                    let mut q = [0.0; 4];
                    for (i, (na, nb, c)) in caps.iter().enumerate() {
                        q[i] = c * (vx(x, *na) - vx(x, *nb));
                    }
                    q
                }
                _ => [0.0; 4],
            })
            .collect()
    }
}

/// Real solver workspace: the MNA buffers plus an LU factorisation that is
/// reused when the matrix is unchanged (linear circuits at a fixed step size
/// only need a forward/back substitution per time point).
///
/// `crate::math::solve` is used for the complex AC systems; this in-place,
/// allocation-free variant keeps transient runs fast in the browser.
pub struct Work {
    pub sys: Sys<f64>,
    lu: Vec<f64>,
    perm: Vec<usize>,
    key: Vec<f64>,
    valid: bool,
    /// LU factorisations performed.
    pub factors: usize,
    /// Linear solves (factorisations + reuses).
    pub solves: usize,
}

impl Work {
    pub fn new(n: usize) -> Self {
        Work {
            sys: Sys::new(n),
            lu: vec![0.0; n * n],
            perm: (0..n).collect(),
            key: vec![f64::NAN; n * n],
            valid: false,
            factors: 0,
            solves: 0,
        }
    }

    /// Solve the system currently in `self.sys`.
    pub fn solve(&mut self) -> Option<Vec<f64>> {
        let n = self.sys.n;
        self.solves += 1;
        if !(self.valid && self.key == self.sys.a) {
            self.key.copy_from_slice(&self.sys.a);
            self.lu.copy_from_slice(&self.sys.a);
            self.valid = lu_factor(&mut self.lu, &mut self.perm, n);
            self.factors += 1;
            if !self.valid {
                return None;
            }
        }
        let mut x: Vec<f64> = self.perm.iter().map(|&p| self.sys.b[p]).collect();
        for i in 0..n {
            let row = &self.lu[i * n..i * n + i];
            let s: f64 = row.iter().zip(&x[..i]).map(|(a, b)| a * b).sum();
            x[i] -= s;
        }
        for i in (0..n).rev() {
            let row = &self.lu[i * n + i + 1..(i + 1) * n];
            let s: f64 = row.iter().zip(&x[i + 1..]).map(|(a, b)| a * b).sum();
            x[i] = (x[i] - s) / self.lu[i * n + i];
        }
        Some(x)
    }
}

/// In-place LU factorisation with partial pivoting (`P·A = L·U`, unit-lower
/// L stored below the diagonal). Returns false for a singular matrix.
fn lu_factor(a: &mut [f64], perm: &mut [usize], n: usize) -> bool {
    for (i, p) in perm.iter_mut().enumerate() {
        *p = i;
    }
    for k in 0..n {
        let mut p = k;
        let mut best = a[k * n + k].abs();
        for i in k + 1..n {
            let m = a[i * n + k].abs();
            if m > best {
                best = m;
                p = i;
            }
        }
        if best < 1e-300 || !best.is_finite() {
            return false;
        }
        if p != k {
            for j in 0..n {
                a.swap(k * n + j, p * n + j);
            }
            perm.swap(k, p);
        }
        let piv = a[k * n + k];
        for i in k + 1..n {
            let f = a[i * n + k] / piv;
            if f == 0.0 {
                continue;
            }
            a[i * n + k] = f;
            let (top, bot) = a.split_at_mut(i * n);
            let rk = &top[k * n + k + 1..k * n + n];
            let ri = &mut bot[k + 1..n];
            for (x, y) in ri.iter_mut().zip(rk) {
                *x -= f * y;
            }
        }
    }
    true
}

/// Outcome of a Newton–Raphson solve.
pub struct NrOut {
    pub x: Vec<f64>,
    pub iters: usize,
}

#[derive(Debug)]
pub enum NrErr {
    Singular,
    NoConvergence,
}

/// Newton–Raphson: iterate `A(x_k) x_{k+1} = b(x_k)` until successive
/// solutions agree within `reltol·|x| + vntol` (voltages) / `+ abstol`
/// (branch currents) and no device limited its step.
pub fn newton(ckt: &Ckt, x0: &[f64], ld: &Load, mem: &mut [IterMem], maxit: usize, ws: &mut Work) -> Result<NrOut, NrErr> {
    let nv = ckt.nn - 1;
    let o = &ckt.opts;
    let mut x = x0.to_vec();
    for it in 1..=maxit {
        ws.sys.clear();
        let noncon = ckt.load(&x, ld, mem, &mut ws.sys);
        let xn = ws.solve().ok_or(NrErr::Singular)?;
        if xn.iter().any(|v| !v.is_finite()) {
            return Err(NrErr::NoConvergence);
        }
        if ckt.linear {
            return Ok(NrOut { x: xn, iters: it });
        }
        // SPICE requires a second iteration so devices are evaluated near the
        // answer; in a transient the predictor already supplies such a point,
        // so the first iteration may converge if it lands within tolerance.
        let first_ok = matches!(ld.mode, Mode::Tran { .. });
        let conv = !noncon
            && (it > 1 || first_ok)
            && xn.iter().zip(&x).enumerate().all(|(i, (a, b))| {
                let tol = o.reltol * a.abs().max(b.abs()) + if i < nv { o.vntol } else { o.abstol.max(1e-9) };
                (a - b).abs() <= tol
            });
        x = xn;
        if conv {
            return Ok(NrOut { x, iters: it });
        }
    }
    Err(NrErr::NoConvergence)
}
