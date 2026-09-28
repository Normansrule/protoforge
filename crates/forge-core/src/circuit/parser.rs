//! SPICE netlist parser (a practical subset of the Berkeley SPICE3 / ngspice
//! input language), so a deck written here runs unchanged in `ngspice -b`.
//!
//! Supported:
//!
//! * Elements `R C L V I D M E G S` (see [`Device`]).
//! * Sources: `DC`, `AC mag [phase]`, `PULSE`, `SIN`, `PWL`, `EXP`.
//! * Directives: `.op .dc .tran .ac .model .ic .nodeset .param .options .end`
//!   (`.control … .endc` blocks and output directives are skipped).
//! * Comments (`*` lines, inline `;` or ` $ `), `+` continuation lines,
//!   case-insensitive names, node `0`/`gnd` as ground.
//! * Engineering suffixes `f p n u m k meg g t mil` (SPICE semantics: `M` is
//!   *milli*, `MEG` is mega, `1F` is one *femto*farad!) and `{expr}` parameter
//!   expressions with `+ - * / ^ **`, `pi` and the usual functions.
//!
//! Reference: ngspice User's Manual, chapters 2 (circuit description), 4
//! (voltage and current sources) and 11 (analyses),
//! <https://ngspice.sourceforge.io/docs.html>.

use std::collections::BTreeMap;

use serde::Serialize;

/// A parse or elaboration error tied to a (1-based) netlist line.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// Time-domain waveform attached to an independent source.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Wave {
    None,
    /// `PULSE(v1 v2 td tr tf pw per)`; `None` fields take the ngspice defaults
    /// (tr = tf = tstep, pw = per = tstop) when a transient is set up.
    Pulse {
        v1: f64,
        v2: f64,
        td: f64,
        tr: Option<f64>,
        tf: Option<f64>,
        pw: Option<f64>,
        per: Option<f64>,
    },
    /// `SIN(vo va freq td theta phase)`.
    Sin {
        vo: f64,
        va: f64,
        freq: Option<f64>,
        td: f64,
        theta: f64,
        phase_deg: f64,
    },
    /// `EXP(v1 v2 td1 tau1 td2 tau2)`.
    Exp {
        v1: f64,
        v2: f64,
        td1: f64,
        tau1: Option<f64>,
        td2: Option<f64>,
        tau2: Option<f64>,
    },
    /// `PWL(t1 v1 t2 v2 ...)`.
    Pwl {
        points: Vec<(f64, f64)>,
    },
}

/// Independent source description.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Source {
    pub dc: f64,
    pub ac_mag: f64,
    pub ac_phase_deg: f64,
    pub wave: Wave,
}

/// Diode model (`.model name D(...)`), SPICE2 junction diode.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DiodeModel {
    pub is: f64,
    pub n: f64,
    pub rs: f64,
    pub cjo: f64,
    pub vj: f64,
    pub m: f64,
    pub fc: f64,
    pub tt: f64,
    /// Reverse breakdown voltage (positive number; `inf` = none).
    pub bv: f64,
}

impl Default for DiodeModel {
    fn default() -> Self {
        DiodeModel {
            is: 1e-14,
            n: 1.0,
            rs: 0.0,
            cjo: 0.0,
            vj: 1.0,
            m: 0.5,
            fc: 0.5,
            tt: 0.0,
            bv: f64::INFINITY,
        }
    }
}

/// MOSFET level-1 (Shichman–Hodges) model.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MosModel {
    pub pmos: bool,
    pub vto: f64,
    pub kp: f64,
    pub lambda: f64,
    pub gamma: f64,
    pub phi: f64,
    /// Gate–source / gate–drain overlap capacitance per metre of width.
    pub cgso: f64,
    pub cgdo: f64,
    /// Zero-bias bulk junction capacitances (treated as constant).
    pub cbd: f64,
    pub cbs: f64,
}

impl Default for MosModel {
    fn default() -> Self {
        MosModel {
            pmos: false,
            vto: 0.0,
            kp: 2e-5,
            lambda: 0.0,
            gamma: 0.0,
            phi: 0.6,
            cgso: 0.0,
            cgdo: 0.0,
            cbd: 0.0,
            cbs: 0.0,
        }
    }
}

/// Voltage-controlled switch model (`.model name SW(RON ROFF VT VH)`).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SwModel {
    pub ron: f64,
    pub roff: f64,
    pub vt: f64,
    pub vh: f64,
}

impl Default for SwModel {
    fn default() -> Self {
        SwModel {
            ron: 1.0,
            roff: 1e12,
            vt: 0.0,
            vh: 0.0,
        }
    }
}

/// One circuit element with its electrical parameters resolved.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Device {
    Resistor {
        r: f64,
    },
    Capacitor {
        c: f64,
        ic: Option<f64>,
    },
    Inductor {
        l: f64,
        ic: Option<f64>,
    },
    Vsource {
        src: Source,
    },
    Isource {
        src: Source,
    },
    Diode {
        model: DiodeModel,
        model_name: String,
        area: f64,
    },
    Mosfet {
        model: MosModel,
        model_name: String,
        w: f64,
        l: f64,
        mult: f64,
    },
    /// Voltage-controlled voltage source (`E`).
    Vcvs {
        gain: f64,
    },
    /// Voltage-controlled current source (`G`).
    Vccs {
        gm: f64,
    },
    Switch {
        model: SwModel,
        model_name: String,
        on: bool,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Element {
    /// Name as written (e.g. `R1`); lookups are case-insensitive.
    pub name: String,
    /// Node indices into [`Netlist::nodes`] (0 = ground). Order follows the
    /// SPICE card: `n+ n-` for two-terminals, `d g s b` for MOSFETs,
    /// `n+ n- nc+ nc-` for E/G/S.
    pub nodes: Vec<usize>,
    pub dev: Device,
    pub line: usize,
}

impl Element {
    pub fn letter(&self) -> char {
        self.name.chars().next().unwrap_or('?').to_ascii_uppercase()
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcSweep {
    Dec,
    Oct,
    Lin,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Analysis {
    Op,
    Dc {
        src: String,
        start: f64,
        stop: f64,
        step: f64,
    },
    Tran {
        tstep: f64,
        tstop: f64,
        tstart: f64,
        tmax: Option<f64>,
        uic: bool,
    },
    Ac {
        sweep: AcSweep,
        n: usize,
        fstart: f64,
        fstop: f64,
    },
}

/// Solver options (`.options`).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Options {
    pub reltol: f64,
    pub abstol: f64,
    pub vntol: f64,
    pub gmin: f64,
    pub itl1: usize,
    pub itl4: usize,
    pub trtol: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            reltol: 1e-3,
            abstol: 1e-12,
            vntol: 1e-6,
            gmin: 1e-12,
            itl1: 100,
            itl4: 30,
            trtol: 7.0,
        }
    }
}

/// A parsed, fully elaborated netlist.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Netlist {
    pub title: String,
    /// Node names, `nodes[0] == "0"` is ground.
    pub nodes: Vec<String>,
    pub elements: Vec<Element>,
    pub analyses: Vec<Analysis>,
    /// `.ic v(node)=value` constraints (node index, volts).
    pub ics: Vec<(usize, f64)>,
    /// `.nodeset` initial guesses for the operating point.
    pub nodesets: Vec<(usize, f64)>,
    /// Final parameter values (after overrides).
    pub params: BTreeMap<String, f64>,
    pub options: Options,
    pub warnings: Vec<String>,
}

impl Netlist {
    pub fn node_index(&self, name: &str) -> Option<usize> {
        let k = canon_node(name);
        self.nodes.iter().position(|n| *n == k)
    }
    pub fn element_index(&self, name: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.name.eq_ignore_ascii_case(name))
    }
}

fn canon_node(name: &str) -> String {
    let l = name.to_ascii_lowercase();
    if l == "gnd" || l == "0" {
        "0".into()
    } else {
        l
    }
}

// ---------------------------------------------------------------------------
// Numbers and expressions
// ---------------------------------------------------------------------------

/// Parse a SPICE number such as `4.7k`, `10u`, `1meg`, `2.2e-6`, `5mil`,
/// `100nF` (trailing unit letters are ignored, as in SPICE).
pub fn parse_number(tok: &str) -> Option<f64> {
    let s = tok.trim();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    let mut saw_digit = false;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        if b[i].is_ascii_digit() {
            saw_digit = true;
        }
        i += 1;
    }
    if !saw_digit || s[digits_start..i].matches('.').count() > 1 {
        return None;
    }
    // exponent: e/E followed by optional sign and at least one digit
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        if j < b.len() && b[j].is_ascii_digit() {
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let mantissa: f64 = s[..i].parse().ok()?;
    let rest = s[i..].to_lowercase();
    if rest.is_empty() {
        return Some(mantissa);
    }
    // the remainder must be letters (suffix + unit), e.g. "k", "meg", "uF", "ohm"
    if !rest.chars().all(|c| c.is_alphabetic() || c == 'µ' || c == 'Ω' || c == '%') {
        return None;
    }
    let scale = if rest.starts_with("meg") {
        1e6
    } else if rest.starts_with("mil") {
        25.4e-6
    } else {
        match rest.chars().next().unwrap() {
            't' => 1e12,
            'g' => 1e9,
            'k' => 1e3,
            'm' => 1e-3,
            'u' | 'µ' => 1e-6,
            'n' => 1e-9,
            'p' => 1e-12,
            'f' => 1e-15,
            'a' => 1e-18,
            _ => 1.0,
        }
    };
    Some(mantissa * scale)
}

#[derive(Clone, Debug, PartialEq)]
enum ETok {
    Num(f64),
    Id(String),
    Op(&'static str),
    LP,
    RP,
    Comma,
}

fn lex_expr(src: &str) -> Result<Vec<ETok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let st = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                let mut j = i + 1;
                if j < c.len() && (c[j] == '+' || c[j] == '-') {
                    j += 1;
                }
                if j < c.len() && c[j].is_ascii_digit() {
                    while j < c.len() && c[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            while i < c.len() && (c[i].is_alphabetic() || c[i] == 'µ') {
                i += 1;
            }
            let t: String = c[st..i].iter().collect();
            out.push(ETok::Num(parse_number(&t).ok_or_else(|| format!("bad number `{t}`"))?));
        } else if ch.is_alphabetic() || ch == '_' {
            let st = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(ETok::Id(c[st..i].iter().collect::<String>().to_lowercase()));
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            if two == "**" {
                out.push(ETok::Op("^"));
                i += 2;
                continue;
            }
            out.push(match ch {
                '+' => ETok::Op("+"),
                '-' => ETok::Op("-"),
                '*' => ETok::Op("*"),
                '/' => ETok::Op("/"),
                '^' => ETok::Op("^"),
                '(' => ETok::LP,
                ')' => ETok::RP,
                ',' => ETok::Comma,
                _ => return Err(format!("unexpected character `{ch}` in expression")),
            });
            i += 1;
        }
    }
    Ok(out)
}

struct ExprParser<'a> {
    t: Vec<ETok>,
    i: usize,
    lookup: &'a mut dyn FnMut(&str) -> Result<f64, String>,
}

impl ExprParser<'_> {
    fn peek(&self) -> Option<&ETok> {
        self.t.get(self.i)
    }
    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        while let Some(ETok::Op(o)) = self.peek() {
            let o = *o;
            if o != "+" && o != "-" {
                break;
            }
            self.i += 1;
            let r = self.term()?;
            v = if o == "+" { v + r } else { v - r };
        }
        Ok(v)
    }
    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.unary()?;
        while let Some(ETok::Op(o)) = self.peek() {
            let o = *o;
            if o != "*" && o != "/" {
                break;
            }
            self.i += 1;
            let r = self.unary()?;
            v = if o == "*" { v * r } else { v / r };
        }
        Ok(v)
    }
    fn unary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(ETok::Op("-")) => {
                self.i += 1;
                Ok(-self.unary()?)
            }
            Some(ETok::Op("+")) => {
                self.i += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }
    fn power(&mut self) -> Result<f64, String> {
        let base = self.primary()?;
        if let Some(ETok::Op("^")) = self.peek() {
            self.i += 1;
            let e = self.unary()?;
            return Ok(base.powf(e));
        }
        Ok(base)
    }
    fn primary(&mut self) -> Result<f64, String> {
        let t = self.peek().cloned().ok_or("unexpected end of expression")?;
        self.i += 1;
        match t {
            ETok::Num(v) => Ok(v),
            ETok::LP => {
                let v = self.expr()?;
                if self.peek() != Some(&ETok::RP) {
                    return Err("missing `)`".into());
                }
                self.i += 1;
                Ok(v)
            }
            ETok::Id(name) => {
                if self.peek() == Some(&ETok::LP) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&ETok::RP) {
                        loop {
                            args.push(self.expr()?);
                            match self.peek() {
                                Some(ETok::Comma) => self.i += 1,
                                Some(ETok::RP) => break,
                                _ => return Err(format!("expected `,` or `)` in call to {name}")),
                            }
                        }
                    }
                    self.i += 1;
                    call_fn(&name, &args)
                } else if name == "pi" {
                    Ok(std::f64::consts::PI)
                } else {
                    (self.lookup)(&name)
                }
            }
            other => Err(format!("unexpected token {other:?}")),
        }
    }
}

fn call_fn(name: &str, a: &[f64]) -> Result<f64, String> {
    let one = |f: fn(f64) -> f64| -> Result<f64, String> {
        if a.len() != 1 {
            return Err(format!("{name}() takes 1 argument"));
        }
        Ok(f(a[0]))
    };
    let two = |f: fn(f64, f64) -> f64| -> Result<f64, String> {
        if a.len() != 2 {
            return Err(format!("{name}() takes 2 arguments"));
        }
        Ok(f(a[0], a[1]))
    };
    match name {
        "sqrt" => one(f64::sqrt),
        "exp" => one(f64::exp),
        "ln" | "log" => one(f64::ln),
        "log10" => one(f64::log10),
        "abs" => one(f64::abs),
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "atan" => one(f64::atan),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "int" => one(f64::trunc),
        "min" => two(f64::min),
        "max" => two(f64::max),
        "pow" | "pwr" => two(f64::powf),
        _ => Err(format!("unknown function `{name}`")),
    }
}

/// Evaluate an arithmetic expression; identifiers are resolved by `lookup`.
pub fn eval_expr(src: &str, lookup: &mut dyn FnMut(&str) -> Result<f64, String>) -> Result<f64, String> {
    let t = lex_expr(src)?;
    if t.is_empty() {
        return Err("empty expression".into());
    }
    let mut p = ExprParser { t, i: 0, lookup };
    let v = p.expr()?;
    if p.i != p.t.len() {
        return Err(format!("unexpected trailing input in `{src}`"));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// Line tokenizer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Expr(String),
    LP,
    RP,
    Eq,
}

impl Tok {
    fn word(&self) -> Option<&str> {
        if let Tok::Word(w) = self {
            Some(w)
        } else {
            None
        }
    }
    fn is_kw(&self, kw: &str) -> bool {
        self.word().is_some_and(|w| w.eq_ignore_ascii_case(kw))
    }
    fn show(&self) -> String {
        match self {
            Tok::Word(w) => w.clone(),
            Tok::Expr(e) => format!("{{{e}}}"),
            Tok::LP => "(".into(),
            Tok::RP => ")".into(),
            Tok::Eq => "=".into(),
        }
    }
}

fn strip_comment(line: &str) -> &str {
    let mut depth = 0i32;
    let b = line.as_bytes();
    for (i, &ch) in b.iter().enumerate() {
        match ch {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b';' if depth <= 0 => return &line[..i],
            b'$' if depth <= 0 && (i == 0 || b[i - 1].is_ascii_whitespace()) => return &line[..i],
            _ => {}
        }
    }
    line
}

fn tokenize(line: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut Vec<Tok>| {
        if !cur.is_empty() {
            out.push(Tok::Word(std::mem::take(cur)));
        }
    };
    while i < c.len() {
        let ch = c[i];
        match ch {
            ' ' | '\t' | ',' | '\r' => flush(&mut cur, &mut out),
            '(' => {
                flush(&mut cur, &mut out);
                out.push(Tok::LP);
            }
            ')' => {
                flush(&mut cur, &mut out);
                out.push(Tok::RP);
            }
            '=' => {
                flush(&mut cur, &mut out);
                out.push(Tok::Eq);
            }
            '{' => {
                flush(&mut cur, &mut out);
                let mut depth = 1;
                let st = i + 1;
                i += 1;
                while i < c.len() && depth > 0 {
                    if c[i] == '{' {
                        depth += 1;
                    } else if c[i] == '}' {
                        depth -= 1;
                    }
                    i += 1;
                }
                if depth != 0 {
                    return Err("unbalanced `{`".into());
                }
                out.push(Tok::Expr(c[st..i - 1].iter().collect()));
                continue;
            }
            '\'' => {
                flush(&mut cur, &mut out);
                let st = i + 1;
                i += 1;
                while i < c.len() && c[i] != '\'' {
                    i += 1;
                }
                if i >= c.len() {
                    return Err("unterminated `'` expression".into());
                }
                out.push(Tok::Expr(c[st..i].iter().collect()));
            }
            _ => cur.push(ch),
        }
        i += 1;
    }
    flush(&mut cur, &mut out);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

struct ParamTable {
    raw: BTreeMap<String, (String, usize)>,
    over: BTreeMap<String, f64>,
    cache: BTreeMap<String, f64>,
}

impl ParamTable {
    fn get(&mut self, name: &str, stack: &mut Vec<String>) -> Result<f64, String> {
        let k = name.to_lowercase();
        if let Some(v) = self.over.get(&k) {
            return Ok(*v);
        }
        if let Some(v) = self.cache.get(&k) {
            return Ok(*v);
        }
        let Some((expr, _line)) = self.raw.get(&k).cloned() else {
            return Err(format!("unknown parameter `{name}`"));
        };
        if stack.contains(&k) {
            return Err(format!("parameter `{name}` is defined in terms of itself"));
        }
        stack.push(k.clone());
        let v = self.eval_str(&expr, stack)?;
        stack.pop();
        self.cache.insert(k, v);
        Ok(v)
    }

    fn eval_str(&mut self, expr: &str, stack: &mut Vec<String>) -> Result<f64, String> {
        let e = expr.trim();
        let inner = e
            .strip_prefix('{')
            .and_then(|x| x.strip_suffix('}'))
            .or_else(|| e.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
            .unwrap_or(e);
        if let Some(v) = parse_number(inner) {
            return Ok(v);
        }
        let mut lookup = |n: &str| -> Result<f64, String> { self.get(n, stack) };
        eval_expr(inner, &mut lookup)
    }

    fn value(&mut self, t: &Tok) -> Result<f64, String> {
        match t {
            Tok::Word(w) => {
                if let Some(v) = parse_number(w) {
                    Ok(v)
                } else if w.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
                    && (self.raw.contains_key(&w.to_lowercase()) || self.over.contains_key(&w.to_lowercase()))
                {
                    self.get(w, &mut Vec::new())
                } else {
                    Err(format!("expected a number, found `{w}`"))
                }
            }
            Tok::Expr(e) => self.eval_str(e, &mut Vec::new()),
            other => Err(format!("expected a number, found `{}`", other.show())),
        }
    }
}

/// Parse the assignments of a `.param` card: `a=1 b = {2*a} c='a+b'`.
fn parse_param_assignments(rest: &str) -> Result<Vec<(String, String)>, String> {
    let c: Vec<char> = rest.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    let skip_ws = |i: &mut usize| {
        while *i < c.len() && (c[*i].is_whitespace() || c[*i] == ',') {
            *i += 1;
        }
    };
    loop {
        skip_ws(&mut i);
        if i >= c.len() {
            break;
        }
        let st = i;
        while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
            i += 1;
        }
        if st == i {
            return Err(format!("expected a parameter name at `{}`", c[st..].iter().collect::<String>()));
        }
        let name: String = c[st..i].iter().collect::<String>().to_lowercase();
        skip_ws(&mut i);
        if i >= c.len() || c[i] != '=' {
            return Err(format!("expected `=` after parameter `{name}`"));
        }
        i += 1;
        skip_ws(&mut i);
        let vst = i;
        if i < c.len() && (c[i] == '{' || c[i] == '\'') {
            let (open, close) = if c[i] == '{' { ('{', '}') } else { ('\'', '\'') };
            let mut depth = 0;
            loop {
                if i >= c.len() {
                    return Err(format!("unterminated expression for `{name}`"));
                }
                if c[i] == close && (depth == 1 || open == close) && i > vst {
                    i += 1;
                    break;
                }
                if c[i] == open {
                    depth += 1;
                } else if c[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
        } else {
            let mut depth = 0;
            while i < c.len() {
                if c[i] == '(' {
                    depth += 1;
                } else if c[i] == ')' {
                    depth -= 1;
                } else if depth == 0 && c[i].is_whitespace() {
                    break;
                }
                i += 1;
            }
        }
        let val: String = c[vst..i].iter().collect();
        if val.trim().is_empty() {
            return Err(format!("missing value for parameter `{name}`"));
        }
        out.push((name, val));
    }
    Ok(out)
}

/// Rewrite the `.param` cards of a deck so that parameters in `over` take the
/// given values. Used to produce the exact deck a user downloads for ngspice.
pub fn with_params(text: &str, over: &BTreeMap<String, f64>) -> String {
    if over.is_empty() {
        return text.to_string();
    }
    let over_l: BTreeMap<String, f64> = over.iter().map(|(k, v)| (k.to_lowercase(), *v)).collect();
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.len() >= 6 && t[..6].eq_ignore_ascii_case(".param") {
            let body = strip_comment(&t[6..]);
            let comment = &t[6 + body.len()..];
            if let Ok(assigns) = parse_param_assignments(body) {
                let parts: Vec<String> = assigns
                    .iter()
                    .map(|(n, v)| match over_l.get(n) {
                        Some(x) => format!("{n}={}", fmt_spice(*x)),
                        None => format!("{n}={}", v.trim()),
                    })
                    .collect();
                let pad = if comment.is_empty() {
                    String::new()
                } else {
                    format!(" {}", comment.trim_start())
                };
                out.push_str(&format!(".param {}{}\n", parts.join(" "), pad));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Format a number with a SPICE engineering suffix (`4.7k`, `10u`, `1meg`).
pub fn fmt_spice(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let a = v.abs();
    let (scale, suf) = if a >= 1e12 {
        (1e12, "t")
    } else if a >= 1e9 {
        (1e9, "g")
    } else if a >= 1e6 {
        (1e6, "meg")
    } else if a >= 1e3 {
        (1e3, "k")
    } else if a >= 1.0 {
        (1.0, "")
    } else if a >= 1e-3 {
        (1e-3, "m")
    } else if a >= 1e-6 {
        (1e-6, "u")
    } else if a >= 1e-9 {
        (1e-9, "n")
    } else if a >= 1e-12 {
        (1e-12, "p")
    } else {
        (1e-15, "f")
    };
    let m = v / scale;
    let s = format!("{m:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s}{suf}")
}

// ---------------------------------------------------------------------------
// Netlist parser
// ---------------------------------------------------------------------------

struct Card {
    line: usize,
    text: String,
}

struct RawModel {
    kind: String,
    params: Vec<(String, f64)>,
    line: usize,
}

/// Parse a SPICE deck. `overrides` replace `.param` values (used by the UI
/// sliders). On failure every error found is returned, each with its line.
pub fn parse(text: &str, overrides: &BTreeMap<String, f64>) -> Result<Netlist, Vec<ParseError>> {
    let mut errors: Vec<ParseError> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let err = |errors: &mut Vec<ParseError>, line: usize, m: String| errors.push(ParseError { line, message: m });

    // ---- 1. physical lines -> logical cards --------------------------------
    let mut cards: Vec<Card> = Vec::new();
    let mut title = String::new();
    let mut in_control = false;
    for (idx, raw) in text.lines().enumerate() {
        let ln = idx + 1;
        if idx == 0 {
            title = raw.trim().trim_start_matches('*').trim().to_string();
            // warn when line 1 looks like `R1 a b 1k` (a card, not a title)
            let t: Vec<&str> = raw.split_whitespace().collect();
            let looks_like_card = t.len() >= 4
                && t[0].chars().next().is_some_and(|c| "RCLVIrclvi".contains(c))
                && t[0].chars().skip(1).all(|c| c.is_alphanumeric() || c == '_')
                && parse_number(t[3].trim_start_matches("DC").trim_start_matches("dc")).is_some();
            if looks_like_card {
                warnings.push("line 1 is the title line (SPICE convention) and is not parsed as an element".into());
            }
            continue;
        }
        let t = raw.trim();
        let lower = t.to_lowercase();
        if in_control {
            if lower.starts_with(".endc") {
                in_control = false;
            }
            continue;
        }
        if lower.starts_with(".control") {
            in_control = true;
            continue;
        }
        if t.is_empty() || t.starts_with('*') {
            continue;
        }
        let body = strip_comment(t).trim_end();
        if let Some(cont) = body.strip_prefix('+') {
            match cards.last_mut() {
                Some(c) => {
                    c.text.push(' ');
                    c.text.push_str(cont);
                }
                None => err(&mut errors, ln, "continuation line `+` without a preceding card".into()),
            }
            continue;
        }
        if body.trim().is_empty() {
            continue;
        }
        if lower.starts_with(".end") && !lower.starts_with(".endc") && !lower.starts_with(".ends") {
            break;
        }
        cards.push(Card {
            line: ln,
            text: body.to_string(),
        });
    }

    // ---- 2. parameters and models ------------------------------------------
    let mut ptab = ParamTable {
        raw: BTreeMap::new(),
        over: overrides.iter().map(|(k, v)| (k.to_lowercase(), *v)).collect(),
        cache: BTreeMap::new(),
    };
    for c in &cards {
        let l = c.text.to_lowercase();
        if l.starts_with(".param") {
            match parse_param_assignments(&c.text[6..]) {
                Ok(a) => {
                    for (n, v) in a {
                        ptab.raw.insert(n, (v, c.line));
                    }
                }
                Err(e) => err(&mut errors, c.line, e),
            }
        }
    }
    for k in ptab.over.keys() {
        if !ptab.raw.contains_key(k) {
            warnings.push(format!("parameter override `{k}` does not match any .param in the deck"));
        }
    }
    let mut params = BTreeMap::new();
    let names: Vec<(String, usize)> = ptab.raw.iter().map(|(k, (_, l))| (k.clone(), *l)).collect();
    for (n, line) in names {
        match ptab.get(&n, &mut Vec::new()) {
            Ok(v) => {
                params.insert(n, v);
            }
            Err(e) => err(&mut errors, line, format!(".param {n}: {e}")),
        }
    }

    let mut models: BTreeMap<String, RawModel> = BTreeMap::new();
    for c in &cards {
        if !c.text.to_lowercase().starts_with(".model") {
            continue;
        }
        let toks = match tokenize(&c.text) {
            Ok(t) => t,
            Err(e) => {
                err(&mut errors, c.line, e);
                continue;
            }
        };
        if toks.len() < 3 {
            err(
                &mut errors,
                c.line,
                ".model needs a name and a type, e.g. `.model DX D(IS=1e-14)`".into(),
            );
            continue;
        }
        let name = toks[1].show().to_lowercase();
        let kind = toks[2].show().to_lowercase();
        let mut ps = Vec::new();
        let mut i = 3;
        while i < toks.len() {
            match &toks[i] {
                Tok::LP | Tok::RP => i += 1,
                Tok::Word(k) if i + 2 < toks.len() && toks.get(i + 1) == Some(&Tok::Eq) => {
                    match ptab.value(&toks[i + 2]) {
                        Ok(v) => ps.push((k.to_lowercase(), v)),
                        Err(e) => err(&mut errors, c.line, format!("model {name}, {k}: {e}")),
                    }
                    i += 3;
                }
                t => {
                    err(
                        &mut errors,
                        c.line,
                        format!("model {name}: expected `PARAM=value`, found `{}`", t.show()),
                    );
                    i += 1;
                }
            }
        }
        models.insert(
            name,
            RawModel {
                kind,
                params: ps,
                line: c.line,
            },
        );
    }

    // ---- 3. elements and directives ------------------------------------------
    let mut nodes: Vec<String> = vec!["0".into()];
    let node_of = |name: &str, nodes: &mut Vec<String>| -> usize {
        let k = canon_node(name);
        if let Some(i) = nodes.iter().position(|n| *n == k) {
            i
        } else {
            nodes.push(k);
            nodes.len() - 1
        }
    };
    let mut elements: Vec<Element> = Vec::new();
    let mut analyses: Vec<Analysis> = Vec::new();
    let mut ics_raw: Vec<(String, f64, usize)> = Vec::new();
    let mut nodesets_raw: Vec<(String, f64, usize)> = Vec::new();
    let mut options = Options::default();

    for c in &cards {
        let toks = match tokenize(&c.text) {
            Ok(t) => t,
            Err(e) => {
                err(&mut errors, c.line, e);
                continue;
            }
        };
        if toks.is_empty() {
            continue;
        }
        let head = toks[0].show();
        let lhead = head.to_lowercase();
        let line = c.line;

        if lhead.starts_with('.') {
            match lhead.as_str() {
                ".param" | ".model" => {}
                ".op" => analyses.push(Analysis::Op),
                ".dc" => {
                    if toks.len() < 5 {
                        err(&mut errors, line, ".dc needs `.dc <source> <start> <stop> <step>`".into());
                        continue;
                    }
                    let vals: Result<Vec<f64>, String> = toks[2..5].iter().map(|t| ptab.value(t)).collect();
                    match vals {
                        Ok(v) => {
                            if v[2] == 0.0 || ((v[1] - v[0]) / v[2]) < -1e-9 {
                                err(&mut errors, line, ".dc step must be non-zero and point from start to stop".into());
                                continue;
                            }
                            if (v[1] - v[0]) / v[2] > 20000.0 {
                                err(&mut errors, line, ".dc sweep has more than 20000 points".into());
                                continue;
                            }
                            analyses.push(Analysis::Dc {
                                src: toks[1].show(),
                                start: v[0],
                                stop: v[1],
                                step: v[2],
                            });
                            if toks.len() > 5 {
                                warnings.push(format!(
                                    "line {line}: nested .dc sweeps are not supported; only the first source is swept"
                                ));
                            }
                        }
                        Err(e) => err(&mut errors, line, format!(".dc: {e}")),
                    }
                }
                ".tran" => {
                    let mut uic = false;
                    let mut nums = Vec::new();
                    for t in &toks[1..] {
                        if t.is_kw("uic") {
                            uic = true;
                        } else {
                            match ptab.value(t) {
                                Ok(v) => nums.push(v),
                                Err(e) => err(&mut errors, line, format!(".tran: {e}")),
                            }
                        }
                    }
                    if nums.len() < 2 {
                        err(
                            &mut errors,
                            line,
                            ".tran needs `.tran <tstep> <tstop> [tstart [tmax]] [uic]`".into(),
                        );
                        continue;
                    }
                    let (tstep, tstop) = (nums[0], nums[1]);
                    let tstart = nums.get(2).copied().unwrap_or(0.0);
                    if tstep <= 0.0 || tstop <= 0.0 || tstart < 0.0 || tstart >= tstop {
                        err(&mut errors, line, ".tran: need tstep > 0, tstop > 0 and 0 ≤ tstart < tstop".into());
                        continue;
                    }
                    analyses.push(Analysis::Tran {
                        tstep,
                        tstop,
                        tstart,
                        tmax: nums.get(3).copied().filter(|v| *v > 0.0),
                        uic,
                    });
                }
                ".ac" => {
                    if toks.len() < 5 {
                        err(&mut errors, line, ".ac needs `.ac dec|oct|lin <n> <fstart> <fstop>`".into());
                        continue;
                    }
                    let sweep = match toks[1].show().to_lowercase().as_str() {
                        "dec" => AcSweep::Dec,
                        "oct" => AcSweep::Oct,
                        "lin" => AcSweep::Lin,
                        o => {
                            err(&mut errors, line, format!(".ac sweep type must be dec, oct or lin (found `{o}`)"));
                            continue;
                        }
                    };
                    let vals: Result<Vec<f64>, String> = toks[2..5].iter().map(|t| ptab.value(t)).collect();
                    match vals {
                        Ok(v) => {
                            let n = v[0].round().max(1.0) as usize;
                            if v[1] <= 0.0 || v[2] < v[1] {
                                err(&mut errors, line, ".ac: need 0 < fstart ≤ fstop".into());
                                continue;
                            }
                            analyses.push(Analysis::Ac {
                                sweep,
                                n,
                                fstart: v[1],
                                fstop: v[2],
                            });
                        }
                        Err(e) => err(&mut errors, line, format!(".ac: {e}")),
                    }
                }
                ".ic" | ".nodeset" => {
                    // v(node)=value ...
                    let mut i = 1;
                    while i < toks.len() {
                        let ok = toks[i].word().is_some_and(|w| w.eq_ignore_ascii_case("v"))
                            && toks.get(i + 1) == Some(&Tok::LP)
                            && toks.get(i + 3) == Some(&Tok::RP)
                            && toks.get(i + 4) == Some(&Tok::Eq)
                            && i + 5 < toks.len();
                        if !ok {
                            err(&mut errors, line, format!("{lhead}: expected `v(node)=value`"));
                            break;
                        }
                        let node = toks[i + 2].show();
                        match ptab.value(&toks[i + 5]) {
                            Ok(v) => {
                                if lhead == ".ic" {
                                    ics_raw.push((node, v, line));
                                } else {
                                    nodesets_raw.push((node, v, line));
                                }
                            }
                            Err(e) => err(&mut errors, line, format!("{lhead}: {e}")),
                        }
                        i += 6;
                    }
                }
                ".options" | ".option" | ".opt" => {
                    let mut i = 1;
                    while i < toks.len() {
                        if toks.get(i + 1) == Some(&Tok::Eq) && i + 2 < toks.len() {
                            let k = toks[i].show().to_lowercase();
                            match ptab.value(&toks[i + 2]) {
                                Ok(v) => match k.as_str() {
                                    "reltol" => options.reltol = v,
                                    "abstol" => options.abstol = v,
                                    "vntol" => options.vntol = v,
                                    "gmin" => options.gmin = v,
                                    "itl1" => options.itl1 = v.max(5.0) as usize,
                                    "itl4" => options.itl4 = v.max(5.0) as usize,
                                    "trtol" => options.trtol = v,
                                    _ => warnings.push(format!("line {line}: option `{k}` ignored")),
                                },
                                Err(e) => err(&mut errors, line, format!(".options {k}: {e}")),
                            }
                            i += 3;
                        } else {
                            let k = toks[i].show().to_lowercase();
                            if k != "method" && k != "trap" {
                                warnings.push(format!("line {line}: option `{k}` ignored"));
                            }
                            i += 1;
                        }
                    }
                }
                ".title" => title = c.text[6..].trim().to_string(),
                ".subckt" | ".ends" | ".include" | ".inc" | ".lib" => {
                    err(
                        &mut errors,
                        line,
                        format!("`{head}` is not supported here (flatten sub-circuits, or run the deck in ngspice)"),
                    );
                }
                ".save" | ".print" | ".plot" | ".probe" | ".meas" | ".measure" | ".four" | ".width" | ".global" | ".temp" | ".csparam" => {
                    warnings.push(format!(
                        "line {line}: `{head}` ignored (all node voltages and element currents are saved)"
                    ));
                }
                _ => err(&mut errors, line, format!("unknown directive `{head}`")),
            }
            continue;
        }

        // ---- element cards ----
        let letter = head.chars().next().unwrap().to_ascii_uppercase();
        let name = head.clone();
        if elements.iter().any(|e| e.name.eq_ignore_ascii_case(&name)) {
            err(&mut errors, line, format!("duplicate element name `{name}`"));
            continue;
        }
        let nnodes = match letter {
            'R' | 'C' | 'L' | 'V' | 'I' | 'D' => 2,
            'E' | 'G' | 'S' => 4,
            'M' => 4,
            _ => {
                err(
                    &mut errors,
                    line,
                    format!("element type `{letter}` ({name}) is not supported by this simulator — supported: R C L V I D M E G S"),
                );
                continue;
            }
        };
        // MOSFET: allow 3 nodes when the 4th token is a known model (bulk = source)
        let mut node_toks: Vec<String> = Vec::new();
        let mut i = 1;
        while node_toks.len() < nnodes && i < toks.len() {
            if letter == 'M' && node_toks.len() == 3 && models.contains_key(&toks[i].show().to_lowercase()) {
                let s = node_toks[2].clone();
                node_toks.push(s);
                warnings.push(format!("line {line}: {name} has no bulk node; bulk tied to source"));
                break;
            }
            match &toks[i] {
                Tok::Word(w) => node_toks.push(w.clone()),
                t => {
                    err(&mut errors, line, format!("{name}: expected a node name, found `{}`", t.show()));
                    break;
                }
            }
            i += 1;
        }
        if node_toks.len() < nnodes {
            err(&mut errors, line, format!("{name}: needs {nnodes} nodes"));
            continue;
        }
        let nidx: Vec<usize> = node_toks.iter().map(|n| node_of(n, &mut nodes)).collect();
        let rest = &toks[i..];

        let dev: Result<Device, String> = (|| -> Result<Device, String> {
            match letter {
                'R' | 'C' | 'L' => {
                    let mut value: Option<f64> = None;
                    let mut ic = None;
                    let mut j = 0;
                    while j < rest.len() {
                        if rest.get(j + 1) == Some(&Tok::Eq) && j + 2 < rest.len() {
                            let k = rest[j].show().to_lowercase();
                            let v = ptab.value(&rest[j + 2])?;
                            match k.as_str() {
                                "ic" => ic = Some(v),
                                "r" | "c" | "l" | "resistance" | "capacitance" | "inductance" => value = Some(v),
                                _ => warnings.push(format!("line {line}: {name}: parameter `{k}` ignored")),
                            }
                            j += 3;
                        } else if value.is_none() {
                            value = Some(ptab.value(&rest[j])?);
                            j += 1;
                        } else {
                            warnings.push(format!("line {line}: {name}: extra token `{}` ignored", rest[j].show()));
                            j += 1;
                        }
                    }
                    let v = value.ok_or_else(|| format!("{name}: missing value"))?;
                    match letter {
                        'R' => {
                            if v == 0.0 {
                                return Err(format!("{name}: zero resistance (use a voltage source of 0 V for a short)"));
                            }
                            if ic.is_some() {
                                warnings.push(format!("line {line}: {name}: IC ignored on a resistor"));
                            }
                            Ok(Device::Resistor { r: v })
                        }
                        'C' => Ok(Device::Capacitor { c: v, ic }),
                        _ => {
                            if v == 0.0 {
                                return Err(format!("{name}: zero inductance"));
                            }
                            Ok(Device::Inductor { l: v, ic })
                        }
                    }
                }
                'V' | 'I' => {
                    let src = parse_source(rest, &mut ptab, &name)?;
                    Ok(if letter == 'V' {
                        Device::Vsource { src }
                    } else {
                        Device::Isource { src }
                    })
                }
                'D' => {
                    let mname = rest.first().ok_or_else(|| format!("{name}: missing model name"))?.show();
                    let rm = models
                        .get(&mname.to_lowercase())
                        .ok_or_else(|| format!("{name}: model `{mname}` is not defined (.model {mname} D(...))"))?;
                    if rm.kind != "d" {
                        return Err(format!(
                            "{name}: model `{mname}` is type {}, a diode needs type D",
                            rm.kind.to_uppercase()
                        ));
                    }
                    let mut area = 1.0;
                    for t in &rest[1..] {
                        if t.is_kw("off") {
                            continue;
                        }
                        if let Ok(v) = ptab.value(t) {
                            area = v;
                        }
                    }
                    Ok(Device::Diode {
                        model: diode_model(rm, &mut warnings),
                        model_name: mname,
                        area,
                    })
                }
                'M' => {
                    let mname = rest.first().ok_or_else(|| format!("{name}: missing model name"))?.show();
                    let rm = models
                        .get(&mname.to_lowercase())
                        .ok_or_else(|| format!("{name}: model `{mname}` is not defined (.model {mname} NMOS(...))"))?;
                    if rm.kind != "nmos" && rm.kind != "pmos" {
                        return Err(format!(
                            "{name}: model `{mname}` is type {}, a MOSFET needs NMOS or PMOS",
                            rm.kind.to_uppercase()
                        ));
                    }
                    let (mut w, mut l, mut mult) = (100e-6, 100e-6, 1.0);
                    let mut j = 1;
                    while j < rest.len() {
                        if rest.get(j + 1) == Some(&Tok::Eq) && j + 2 < rest.len() {
                            let k = rest[j].show().to_lowercase();
                            let v = ptab.value(&rest[j + 2])?;
                            match k.as_str() {
                                "w" => w = v,
                                "l" => l = v,
                                "m" => mult = v,
                                "ad" | "as" | "pd" | "ps" | "nrd" | "nrs" => {}
                                _ => warnings.push(format!("line {line}: {name}: parameter `{k}` ignored")),
                            }
                            j += 3;
                        } else {
                            j += 1;
                        }
                    }
                    if w <= 0.0 || l <= 0.0 {
                        return Err(format!("{name}: W and L must be positive"));
                    }
                    Ok(Device::Mosfet {
                        model: mos_model(rm, &mut warnings),
                        model_name: mname,
                        w,
                        l,
                        mult,
                    })
                }
                'E' | 'G' => {
                    let t = rest.first().ok_or_else(|| format!("{name}: missing gain"))?;
                    if t.is_kw("value") || t.is_kw("poly") || t.is_kw("table") {
                        return Err(format!(
                            "{name}: `{}` forms are not supported; use the linear form `{name} n+ n- nc+ nc- gain`",
                            t.show()
                        ));
                    }
                    let g = ptab.value(t)?;
                    Ok(if letter == 'E' {
                        Device::Vcvs { gain: g }
                    } else {
                        Device::Vccs { gm: g }
                    })
                }
                'S' => {
                    let mname = rest.first().ok_or_else(|| format!("{name}: missing model name"))?.show();
                    let rm = models
                        .get(&mname.to_lowercase())
                        .ok_or_else(|| format!("{name}: model `{mname}` is not defined (.model {mname} SW(RON=1 ROFF=1meg VT=0.5))"))?;
                    if rm.kind != "sw" {
                        return Err(format!(
                            "{name}: model `{mname}` is type {}, a switch needs type SW",
                            rm.kind.to_uppercase()
                        ));
                    }
                    let on = rest.get(1).is_some_and(|t| t.is_kw("on"));
                    Ok(Device::Switch {
                        model: sw_model(rm, &mut warnings),
                        model_name: mname,
                        on,
                    })
                }
                _ => unreachable!(),
            }
        })();
        match dev {
            Ok(dev) => elements.push(Element {
                name,
                nodes: nidx,
                dev,
                line,
            }),
            Err(e) => err(&mut errors, line, e),
        }
    }

    let resolve = |list: Vec<(String, f64, usize)>, nodes: &Vec<String>, errors: &mut Vec<ParseError>| -> Vec<(usize, f64)> {
        let mut out = Vec::new();
        for (n, v, line) in list {
            match nodes.iter().position(|x| *x == canon_node(&n)) {
                Some(0) => errors.push(ParseError {
                    line,
                    message: "cannot set an initial condition on ground".into(),
                }),
                Some(i) => out.push((i, v)),
                None => errors.push(ParseError {
                    line,
                    message: format!("unknown node `{n}`"),
                }),
            }
        }
        out
    };
    let ics = resolve(ics_raw, &nodes, &mut errors);
    let nodesets = resolve(nodesets_raw, &nodes, &mut errors);

    // ---- 4. sanity checks ----------------------------------------------------
    for (mname, m) in &models {
        if !elements.iter().any(|e| match &e.dev {
            Device::Diode { model_name, .. } | Device::Mosfet { model_name, .. } | Device::Switch { model_name, .. } => {
                model_name.eq_ignore_ascii_case(mname)
            }
            _ => false,
        }) {
            warnings.push(format!("line {}: model `{mname}` is never used", m.line));
        }
    }
    for a in &analyses {
        if let Analysis::Dc { src, .. } = a {
            let ok = elements
                .iter()
                .any(|e| e.name.eq_ignore_ascii_case(src) && matches!(e.dev, Device::Vsource { .. } | Device::Isource { .. }));
            if !ok {
                errors.push(ParseError {
                    line: 0,
                    message: format!(".dc: `{src}` is not an independent V or I source"),
                });
            }
        }
    }
    if errors.is_empty() {
        if elements.is_empty() {
            errors.push(ParseError {
                line: 0,
                message: "the netlist has no elements (remember: line 1 is always the title)".into(),
            });
        } else if !elements.iter().any(|e| e.nodes.contains(&0)) {
            errors.push(ParseError {
                line: 0,
                message: "no element connects to ground (node 0 or gnd)".into(),
            });
        }
    }
    // dangling nodes
    if errors.is_empty() {
        let mut count = vec![0usize; nodes.len()];
        for e in &elements {
            let used: &[usize] = if matches!(e.dev, Device::Vcvs { .. } | Device::Vccs { .. } | Device::Switch { .. }) {
                &e.nodes[..2]
            } else {
                &e.nodes
            };
            for &n in used {
                count[n] += 1;
            }
            if matches!(e.dev, Device::Vcvs { .. } | Device::Vccs { .. } | Device::Switch { .. }) {
                for &n in &e.nodes[2..] {
                    count[n] += 1;
                }
            }
        }
        for (i, c) in count.iter().enumerate().skip(1) {
            if *c < 2 {
                warnings.push(format!("node `{}` has only one connection", nodes[i]));
            }
        }
    }
    if analyses.is_empty() && errors.is_empty() {
        warnings.push("no analysis directive (.op/.dc/.tran/.ac) — running .op".into());
        analyses.push(Analysis::Op);
    }

    if errors.is_empty() {
        Ok(Netlist {
            title,
            nodes,
            elements,
            analyses,
            ics,
            nodesets,
            params,
            options,
            warnings,
        })
    } else {
        errors.sort_by_key(|e| e.line);
        Err(errors)
    }
}

fn parse_source(rest: &[Tok], ptab: &mut ParamTable, name: &str) -> Result<Source, String> {
    let mut src = Source {
        dc: 0.0,
        ac_mag: 0.0,
        ac_phase_deg: 0.0,
        wave: Wave::None,
    };
    let mut dc_set = false;
    let mut j = 0;
    while j < rest.len() {
        let t = &rest[j];
        if t.is_kw("dc") {
            src.dc = ptab.value(rest.get(j + 1).ok_or_else(|| format!("{name}: DC needs a value"))?)?;
            dc_set = true;
            j += 2;
        } else if t.is_kw("ac") {
            src.ac_mag = 1.0;
            j += 1;
            if let Some(v) = rest.get(j).and_then(|t| ptab.value(t).ok()) {
                src.ac_mag = v;
                j += 1;
                if let Some(p) = rest.get(j).and_then(|t| ptab.value(t).ok()) {
                    src.ac_phase_deg = p;
                    j += 1;
                }
            }
        } else if let Some(w) = t
            .word()
            .map(|w| w.to_lowercase())
            .filter(|w| ["pulse", "sin", "pwl", "exp"].contains(&w.as_str()))
        {
            j += 1;
            let mut vals = Vec::new();
            let paren = rest.get(j) == Some(&Tok::LP);
            if paren {
                j += 1;
            }
            while j < rest.len() {
                if rest[j] == Tok::RP {
                    j += 1;
                    break;
                }
                if !paren && rest[j].word().is_some_and(|w| ["dc", "ac"].contains(&w.to_lowercase().as_str())) {
                    break;
                }
                vals.push(
                    ptab.value(&rest[j])
                        .map_err(|e| format!("{name}: {} argument: {e}", w.to_uppercase()))?,
                );
                j += 1;
            }
            let g = |i: usize| vals.get(i).copied();
            src.wave = match w.as_str() {
                "pulse" => {
                    if vals.len() < 2 {
                        return Err(format!("{name}: PULSE needs at least v1 and v2"));
                    }
                    Wave::Pulse {
                        v1: vals[0],
                        v2: vals[1],
                        td: g(2).unwrap_or(0.0),
                        tr: g(3).filter(|v| *v > 0.0),
                        tf: g(4).filter(|v| *v > 0.0),
                        pw: g(5),
                        per: g(6).filter(|v| *v > 0.0),
                    }
                }
                "sin" => {
                    if vals.len() < 2 {
                        return Err(format!("{name}: SIN needs at least vo and va"));
                    }
                    Wave::Sin {
                        vo: vals[0],
                        va: vals[1],
                        freq: g(2).filter(|v| *v > 0.0),
                        td: g(3).unwrap_or(0.0),
                        theta: g(4).unwrap_or(0.0),
                        phase_deg: g(5).unwrap_or(0.0),
                    }
                }
                "exp" => {
                    if vals.len() < 2 {
                        return Err(format!("{name}: EXP needs at least v1 and v2"));
                    }
                    Wave::Exp {
                        v1: vals[0],
                        v2: vals[1],
                        td1: g(2).unwrap_or(0.0),
                        tau1: g(3).filter(|v| *v > 0.0),
                        td2: g(4),
                        tau2: g(5).filter(|v| *v > 0.0),
                    }
                }
                _ => {
                    if vals.len() < 2 || vals.len() % 2 != 0 {
                        return Err(format!("{name}: PWL needs pairs of (time value)"));
                    }
                    let pts: Vec<(f64, f64)> = vals.chunks(2).map(|c| (c[0], c[1])).collect();
                    if pts.windows(2).any(|w| w[1].0 < w[0].0) {
                        return Err(format!("{name}: PWL times must be non-decreasing"));
                    }
                    Wave::Pwl { points: pts }
                }
            };
            if !dc_set {
                // like SPICE, the DC value defaults to the waveform's value at t = 0
                src.dc = wave_at(&src.wave, 0.0);
            }
        } else if !dc_set {
            src.dc = ptab.value(t).map_err(|e| format!("{name}: {e}"))?;
            dc_set = true;
            j += 1;
        } else {
            return Err(format!("{name}: unexpected `{}`", t.show()));
        }
    }
    Ok(src)
}

/// Waveform value used when the DC value is omitted (t = 0, defaults unresolved).
fn wave_at(w: &Wave, t: f64) -> f64 {
    match w {
        Wave::None => 0.0,
        Wave::Pulse { v1, .. } => *v1,
        Wave::Sin { vo, va, phase_deg, .. } => vo + va * phase_deg.to_radians().sin() * if t >= 0.0 { 1.0 } else { 0.0 },
        Wave::Exp { v1, .. } => *v1,
        Wave::Pwl { points } => points.first().map(|p| p.1).unwrap_or(0.0),
    }
}

fn diode_model(rm: &RawModel, warnings: &mut Vec<String>) -> DiodeModel {
    let mut m = DiodeModel::default();
    for (k, v) in &rm.params {
        match k.as_str() {
            "is" => m.is = *v,
            "n" => m.n = *v,
            "rs" => m.rs = *v,
            "cjo" | "cj0" | "cj" => m.cjo = *v,
            "vj" | "pb" => m.vj = *v,
            "m" | "mj" => m.m = *v,
            "fc" => m.fc = *v,
            "tt" => m.tt = *v,
            "bv" => m.bv = v.abs(),
            "level" => {}
            _ => warnings.push(format!("line {}: diode model parameter `{k}` ignored", rm.line)),
        }
    }
    m
}

fn mos_model(rm: &RawModel, warnings: &mut Vec<String>) -> MosModel {
    let mut m = MosModel {
        pmos: rm.kind == "pmos",
        ..Default::default()
    };
    let (mut uo, mut tox, mut kp_set) = (600.0, 0.0, false);
    for (k, v) in &rm.params {
        match k.as_str() {
            "vto" | "vt0" => m.vto = *v,
            "kp" => {
                m.kp = *v;
                kp_set = true;
            }
            "lambda" => m.lambda = *v,
            "gamma" => m.gamma = *v,
            "phi" => m.phi = *v,
            "cgso" => m.cgso = *v,
            "cgdo" => m.cgdo = *v,
            "cbd" => m.cbd = *v,
            "cbs" => m.cbs = *v,
            "uo" | "u0" => uo = *v,
            "tox" => tox = *v,
            "level" => {
                if (*v - 1.0).abs() > 1e-9 {
                    warnings.push(format!(
                        "line {}: only MOSFET LEVEL=1 is implemented; simulating as level 1",
                        rm.line
                    ));
                }
            }
            _ => warnings.push(format!("line {}: MOSFET model parameter `{k}` ignored", rm.line)),
        }
    }
    if !kp_set && tox > 0.0 {
        // KP = µ0·Cox, with µ0 in cm²/Vs and Cox = εox/tox
        m.kp = uo * 1e-4 * 3.9 * 8.854e-12 / tox;
    }
    if m.pmos && m.vto > 0.0 {
        warnings.push(format!("line {}: PMOS model has positive VTO (depletion device)", rm.line));
    }
    m
}

fn sw_model(rm: &RawModel, warnings: &mut Vec<String>) -> SwModel {
    let mut m = SwModel::default();
    for (k, v) in &rm.params {
        match k.as_str() {
            "ron" => m.ron = *v,
            "roff" => m.roff = *v,
            "vt" => m.vt = *v,
            "vh" => m.vh = v.abs(),
            _ => warnings.push(format!("line {}: switch model parameter `{k}` ignored", rm.line)),
        }
    }
    if m.ron <= 0.0 {
        m.ron = 1e-3;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * b.abs().max(1e-30)
    }

    #[test]
    fn engineering_suffixes() {
        assert!(close(parse_number("4.7k").unwrap(), 4700.0));
        assert!(close(parse_number("1meg").unwrap(), 1e6));
        assert!(close(parse_number("1MEG").unwrap(), 1e6));
        assert!(close(parse_number("1M").unwrap(), 1e-3));
        assert!(close(parse_number("10uF").unwrap(), 10e-6));
        assert!(close(parse_number("2.2e-6").unwrap(), 2.2e-6));
        assert!(close(parse_number("1e3k").unwrap(), 1e6));
        assert!(close(parse_number("5mil").unwrap(), 5.0 * 25.4e-6));
        assert!(close(parse_number("100p").unwrap(), 100e-12));
        assert!(close(parse_number("-3.3V").unwrap(), -3.3));
        assert!(close(parse_number("1F").unwrap(), 1e-15));
        assert!(parse_number("abc").is_none());
        assert!(parse_number("1.2.3").is_none());
    }

    #[test]
    fn expressions() {
        let mut none = |_: &str| -> Result<f64, String> { Err("x".into()) };
        assert!(close(eval_expr("1+2*3", &mut none).unwrap(), 7.0));
        assert!(close(eval_expr("2**3**2", &mut none).unwrap(), 512.0));
        assert!(close(eval_expr("-2^2", &mut none).unwrap(), -4.0));
        assert!(close(eval_expr("sqrt(16)/2k", &mut none).unwrap(), 2e-3));
        assert!((eval_expr("1/(2*pi*1k*1u)", &mut none).unwrap() - 159.154_943).abs() < 1e-5);
        assert!(close(eval_expr("max(1,3)-min(2,5)", &mut none).unwrap(), 1.0));
    }

    #[test]
    fn params_continuations_and_errors() {
        let deck = "test deck\n.param r=1k c = {1u/2}\n.param tau={r*c}\nR1 in out {r} ; comment\nC1 out 0\n+ {c}\nV1 in 0 PULSE(0 1 0 1n 1n 1m 2m)\n.tran {tau/100} {5*tau}\n.end\nR9 garbage";
        let n = parse(deck, &BTreeMap::new()).unwrap();
        assert_eq!(n.elements.len(), 3);
        assert!(close(n.params["tau"], 0.5e-3));
        match &n.analyses[0] {
            Analysis::Tran { tstep, tstop, .. } => {
                assert!(close(*tstep, 5e-6));
                assert!(close(*tstop, 2.5e-3));
            }
            _ => panic!(),
        }
        let mut over = BTreeMap::new();
        over.insert("r".to_string(), 2000.0);
        let n2 = parse(deck, &over).unwrap();
        assert!(close(n2.params["tau"], 1e-3));
        let e = parse("t\nR1 a 0\nQ1 a b c qmod\n.tran 1n\n", &BTreeMap::new()).unwrap_err();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].line, 2);
        assert_eq!(e[1].line, 3);
        assert_eq!(e[2].line, 4);
    }

    #[test]
    fn title_line_heuristic() {
        let a = parse("Iris 1.8 V rail - load step\nR1 a 0 1k\n.op\n", &BTreeMap::new()).unwrap();
        assert!(!a.warnings.iter().any(|w| w.contains("title")), "{:?}", a.warnings);
        let b = parse("R1 a 0 1k\nR2 a 0 1k\nV1 a 0 1\n.op\n", &BTreeMap::new()).unwrap();
        assert!(b.warnings.iter().any(|w| w.contains("title")));
    }

    #[test]
    fn rewrite_params() {
        let mut over = BTreeMap::new();
        over.insert("R".to_string(), 2200.0);
        let out = with_params("t\n.param r=1k c=1u ; note\nR1 a 0 {r}\n", &over);
        assert!(out.contains(".param r=2.2k c=1u ; note"));
    }

    #[test]
    fn models_and_sources() {
        let deck = "t\nVin in 0 DC 1 AC 1 90 SIN(0 1 1k)\nM1 d in 0 0 nm W=2u L=1u\nRd vdd d 10k\nVdd vdd 0 3.3\nD1 d 0 dx 2\nS1 d 0 in 0 sw1 OFF\n.model nm nmos (level=1 vto=0.7 kp=100u lambda=0.02)\n.model dx D(IS=1e-15 N=1.5)\n.model sw1 SW(RON=1 ROFF=1meg VT=0.5)\n.op\n";
        let n = parse(deck, &BTreeMap::new()).unwrap();
        match &n.elements[0].dev {
            Device::Vsource { src } => {
                assert_eq!(src.dc, 1.0);
                assert_eq!(src.ac_phase_deg, 90.0);
                assert!(matches!(src.wave, Wave::Sin { .. }));
            }
            _ => panic!(),
        }
        match &n.elements[1].dev {
            Device::Mosfet { model, w, .. } => {
                assert!(close(model.vto, 0.7));
                assert!(close(*w, 2e-6));
            }
            _ => panic!(),
        }
        match &n.elements[4].dev {
            Device::Diode { model, area, .. } => {
                assert!(close(model.n, 1.5));
                assert_eq!(*area, 2.0);
            }
            _ => panic!(),
        }
    }
}
