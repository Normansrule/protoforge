//! Circuit exchange formats.
//!
//! * **OpenQASM 2.0** reader and writer (A. W. Cross, L. S. Bishop, J. A.
//!   Smolin, J. M. Gambetta, "Open Quantum Assembly Language",
//!   arXiv:1707.03429, 2017). The reader understands `qreg`/`creg`
//!   (several registers, flattened in declaration order), the `qelib1.inc`
//!   gates listed in [`QELIB`], the built-ins `U` and `CX`, user `gate`
//!   definitions (expanded like macros, with parameter expressions using
//!   `pi + - * / ^`, `sin cos tan exp ln sqrt`), register broadcasting
//!   (`h q;`, `measure q -> c;`), `measure`, `reset`, `barrier` and
//!   `if (creg == n) op;`. `opaque` and unknown gates are errors with line
//!   numbers. Programs written here open in the IBM Quantum Composer and in
//!   Qiskit (`QuantumCircuit.from_qasm_str`).
//! * **OpenQASM 3.0** writer ([`emit_qasm3`], `stdgates.inc` plus `ctrl @`
//!   modifiers) and a **Qiskit** Python script writer ([`emit_qiskit`]).
//! * **Quirk** links (C. Gidney, <https://algassert.com/quirk>): the circuit
//!   is JSON in the URL fragment, `#circuit={"cols":[["H"],["•","X"]]}`. Gate
//!   ids were checked against the Quirk source (github.com/Strilanc/Quirk,
//!   `src/gates/*.js`): `H X Y Z • Swap Measure`, `Z^½ Z^-½ Z^¼ Z^-¼ X^½
//!   X^-½`, and the formula rotations `Rxft Ryft Rzft` with a constant
//!   angle argument in radians; everything else is exported as a custom
//!   matrix gate (`"gates":[{"id":"~g0","matrix":"{{…}}"}]`).

use std::collections::HashMap;

use serde::Serialize;

use super::circuit::{matrix, normalize, Circuit, Cond, Creg, Op};
use crate::math::PI;

/// Gates accepted from `qelib1.inc` (Qiskit's version of the file).
pub const QELIB: &[&str] = &[
    "u3", "u2", "u1", "u", "p", "cx", "id", "u0", "x", "y", "z", "h", "s", "sdg", "t", "tdg", "rx", "ry", "rz", "sx", "sxdg", "cz", "cy",
    "swap", "ch", "ccx", "cswap", "crx", "cry", "crz", "cu1", "cp", "cu3", "csx", "rxx", "rzz", "c3x", "c4x",
];

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Id(String),
    Num(f64),
    Int(u64),
    Str(String),
    Sym(&'static str),
}

#[derive(Clone, Debug)]
struct T {
    t: Tok,
    line: usize,
}

fn lex(src: &str) -> Result<Vec<T>, String> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1;
    let mut out = vec![];
    const SYMS: &[&str] = &["->", "==", ";", ",", "(", ")", "[", "]", "{", "}", "+", "-", "*", "/", "^"];
    while i < b.len() {
        let ch = b[i];
        if ch == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                if b[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if ch == '"' {
            let s = i + 1;
            i += 1;
            while i < b.len() && b[i] != '"' {
                i += 1;
            }
            out.push(T {
                t: Tok::Str(b[s..i.min(b.len())].iter().collect()),
                line,
            });
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < b.len() && b[i + 1].is_ascii_digit()) {
            let s = i;
            let mut real = false;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') {
                if b[i] == '.' {
                    real = true;
                }
                i += 1;
            }
            if i < b.len() && (b[i] == 'e' || b[i] == 'E') {
                real = true;
                i += 1;
                if i < b.len() && (b[i] == '+' || b[i] == '-') {
                    i += 1;
                }
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let txt: String = b[s..i].iter().collect();
            let t = if real {
                Tok::Num(txt.parse().map_err(|_| format!("line {line}: bad number `{txt}`"))?)
            } else {
                Tok::Int(txt.parse().map_err(|_| format!("line {line}: bad integer `{txt}`"))?)
            };
            out.push(T { t, line });
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            out.push(T {
                t: Tok::Id(b[s..i].iter().collect()),
                line,
            });
            continue;
        }
        let rest: String = b[i..(i + 2).min(b.len())].iter().collect();
        if let Some(s) = SYMS.iter().find(|s| rest.starts_with(**s)) {
            out.push(T { t: Tok::Sym(s), line });
            i += s.len();
            continue;
        }
        return Err(format!("line {line}: unexpected character `{ch}`"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Box<Expr>),
}

impl Expr {
    fn eval(&self, env: &HashMap<String, f64>) -> Result<f64, String> {
        Ok(match self {
            Expr::Num(v) => *v,
            Expr::Var(n) => {
                if n == "pi" {
                    PI
                } else {
                    *env.get(n).ok_or_else(|| format!("unknown parameter `{n}`"))?
                }
            }
            Expr::Neg(e) => -e.eval(env)?,
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(env)?, b.eval(env)?);
                match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    '/' => x / y,
                    _ => x.powf(y),
                }
            }
            Expr::Call(f, e) => {
                let x = e.eval(env)?;
                match f.as_str() {
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "exp" => x.exp(),
                    "ln" => x.ln(),
                    "sqrt" => x.sqrt(),
                    _ => return Err(format!("unknown function `{f}`")),
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Arg {
    Reg(String),
    Bit(String, usize),
}

#[derive(Clone, Debug)]
struct Call {
    name: String,
    params: Vec<Expr>,
    args: Vec<Arg>,
    line: usize,
}

#[derive(Clone, Debug)]
struct GateDef {
    params: Vec<String>,
    args: Vec<String>,
    body: Vec<Call>,
}

struct P {
    toks: Vec<T>,
    i: usize,
}

impl P {
    fn line(&self) -> usize {
        self.toks.get(self.i).or(self.toks.last()).map(|t| t.line).unwrap_or(0)
    }
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|t| &t.t)
    }
    fn next(&mut self) -> Result<Tok, String> {
        let t = self
            .toks
            .get(self.i)
            .map(|t| t.t.clone())
            .ok_or_else(|| "unexpected end of program".to_string())?;
        self.i += 1;
        Ok(t)
    }
    fn is(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(x)) if *x == s)
    }
    fn expect(&mut self, s: &str) -> Result<(), String> {
        let line = self.line();
        match self.next()? {
            Tok::Sym(x) if x == s => Ok(()),
            t => Err(format!("line {line}: expected `{s}`, found {}", show(&t))),
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        let line = self.line();
        match self.next()? {
            Tok::Id(s) => Ok(s),
            t => Err(format!("line {line}: expected a name, found {}", show(&t))),
        }
    }
    fn int(&mut self) -> Result<usize, String> {
        let line = self.line();
        match self.next()? {
            Tok::Int(v) => Ok(v as usize),
            t => Err(format!("line {line}: expected an integer, found {}", show(&t))),
        }
    }
    // expr := term (('+'|'-') term)*
    fn expr(&mut self) -> Result<Expr, String> {
        let mut a = self.term()?;
        while self.is("+") || self.is("-") {
            let op = if self.is("+") { '+' } else { '-' };
            self.i += 1;
            a = Expr::Bin(op, Box::new(a), Box::new(self.term()?));
        }
        Ok(a)
    }
    fn term(&mut self) -> Result<Expr, String> {
        let mut a = self.power()?;
        while self.is("*") || self.is("/") {
            let op = if self.is("*") { '*' } else { '/' };
            self.i += 1;
            a = Expr::Bin(op, Box::new(a), Box::new(self.power()?));
        }
        Ok(a)
    }
    fn power(&mut self) -> Result<Expr, String> {
        let a = self.unary()?;
        if self.is("^") {
            self.i += 1;
            return Ok(Expr::Bin('^', Box::new(a), Box::new(self.power()?)));
        }
        Ok(a)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.is("-") {
            self.i += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.is("+") {
            self.i += 1;
            return self.unary();
        }
        let line = self.line();
        match self.next()? {
            Tok::Num(v) => Ok(Expr::Num(v)),
            Tok::Int(v) => Ok(Expr::Num(v as f64)),
            Tok::Sym("(") => {
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Id(name) => {
                if self.is("(") {
                    self.i += 1;
                    let e = self.expr()?;
                    self.expect(")")?;
                    Ok(Expr::Call(name, Box::new(e)))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            t => Err(format!("line {line}: expected a number or expression, found {}", show(&t))),
        }
    }
    fn arg(&mut self) -> Result<Arg, String> {
        let name = self.ident()?;
        if self.is("[") {
            self.i += 1;
            let k = self.int()?;
            self.expect("]")?;
            Ok(Arg::Bit(name, k))
        } else {
            Ok(Arg::Reg(name))
        }
    }
    fn call(&mut self, name: String, line: usize) -> Result<Call, String> {
        let mut params = vec![];
        if self.is("(") {
            self.i += 1;
            if !self.is(")") {
                params.push(self.expr()?);
                while self.is(",") {
                    self.i += 1;
                    params.push(self.expr()?);
                }
            }
            self.expect(")")?;
        }
        let mut args = vec![self.arg()?];
        while self.is(",") {
            self.i += 1;
            args.push(self.arg()?);
        }
        self.expect(";")?;
        Ok(Call { name, params, args, line })
    }
}

fn show(t: &Tok) -> String {
    match t {
        Tok::Id(s) => format!("`{s}`"),
        Tok::Num(v) => format!("`{v}`"),
        Tok::Int(v) => format!("`{v}`"),
        Tok::Str(s) => format!("\"{s}\""),
        Tok::Sym(s) => format!("`{s}`"),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ParseOut {
    pub circuit: Circuit,
    pub qregs: Vec<Creg>,
    pub warnings: Vec<String>,
    pub ops: usize,
}

struct Ctx {
    qregs: Vec<(String, usize, usize)>, // name, offset, size
    cregs: Vec<(String, usize, usize)>,
    defs: HashMap<String, GateDef>,
    ops: Vec<Op>,
}

impl Ctx {
    fn qreg(&self, n: &str) -> Option<(usize, usize)> {
        self.qregs.iter().find(|r| r.0 == n).map(|r| (r.1, r.2))
    }
    fn creg(&self, n: &str) -> Option<(usize, usize)> {
        self.cregs.iter().find(|r| r.0 == n).map(|r| (r.1, r.2))
    }
    /// Expand register arguments into per-index qubit lists (broadcasting).
    fn qargs(&self, args: &[Arg], line: usize) -> Result<Vec<Vec<usize>>, String> {
        let mut size: Option<usize> = None;
        for a in args {
            if let Arg::Reg(n) = a {
                let (_, s) = self.qreg(n).ok_or_else(|| format!("line {line}: unknown quantum register `{n}`"))?;
                if size.is_some_and(|x| x != s) {
                    return Err(format!("line {line}: registers of different sizes cannot be broadcast together"));
                }
                size = Some(s);
            }
        }
        let reps = size.unwrap_or(1);
        let mut out = vec![];
        for k in 0..reps {
            let mut v = vec![];
            for a in args {
                match a {
                    Arg::Reg(n) => v.push(self.qreg(n).unwrap().0 + k),
                    Arg::Bit(n, i) => {
                        let (off, s) = self.qreg(n).ok_or_else(|| format!("line {line}: unknown quantum register `{n}`"))?;
                        if *i >= s {
                            return Err(format!("line {line}: index {i} is out of range for `{n}[{s}]`"));
                        }
                        v.push(off + i);
                    }
                }
            }
            out.push(v);
        }
        Ok(out)
    }
}

fn builtin(name: &str) -> Option<(usize, usize)> {
    // (params, qubits)
    Some(match name {
        "U" | "u3" | "u" => (3, 1),
        "u2" => (2, 1),
        "u1" | "p" | "rx" | "ry" | "rz" => (1, 1),
        "CX" | "cx" | "cz" | "cy" | "ch" | "swap" | "csx" => (0, 2),
        "crx" | "cry" | "crz" | "cu1" | "cp" | "rxx" | "rzz" => (1, 2),
        "cu3" => (3, 2),
        "ccx" | "cswap" => (0, 3),
        "c3x" => (0, 4),
        "c4x" => (0, 5),
        "id" | "u0" | "x" | "y" | "z" | "h" | "s" | "sdg" | "t" | "tdg" | "sx" | "sxdg" => (0, 1),
        _ => return None,
    })
}

fn emit_builtin(ctx: &mut Ctx, name: &str, p: &[f64], q: &[usize], cond: &Option<Cond>, line: usize) -> Result<(), String> {
    let push = |ctx: &mut Ctx, o: Op| -> Result<(), String> {
        let mut o = normalize(&o).map_err(|e| format!("line {line}: {e}"))?;
        o.cond = cond.clone();
        let mut seen = vec![];
        for &x in o.targets.iter().chain(&o.controls) {
            if seen.contains(&x) {
                return Err(format!("line {line}: the same qubit appears twice in `{name}`"));
            }
            seen.push(x);
        }
        ctx.ops.push(o);
        Ok(())
    };
    match name {
        "u0" | "id" => Ok(()),
        "U" => push(ctx, Op::new("u3", q).param(p)),
        "CX" => push(ctx, Op::new("cx", q)),
        "rzz" => {
            // rzz(θ) a,b = cx a,b; u1(θ) b; cx a,b
            push(ctx, Op::ctrl("x", &[q[0]], &[q[1]]))?;
            push(ctx, Op::new("p", &[q[1]]).param(p))?;
            push(ctx, Op::ctrl("x", &[q[0]], &[q[1]]))
        }
        "rxx" => {
            // rxx(θ) a,b = h a; h b; rzz(θ) a,b; h a; h b   (exp(−iθ XX/2) up to global phase)
            for &x in q {
                push(ctx, Op::new("h", &[x]))?;
            }
            push(ctx, Op::ctrl("x", &[q[0]], &[q[1]]))?;
            push(ctx, Op::new("rz", &[q[1]]).param(p))?;
            push(ctx, Op::ctrl("x", &[q[0]], &[q[1]]))?;
            for &x in q {
                push(ctx, Op::new("h", &[x]))?;
            }
            Ok(())
        }
        _ => push(ctx, Op::new(name, q).param(p)),
    }
}

fn expand(
    ctx: &mut Ctx,
    call: &Call,
    env: &HashMap<String, f64>,
    qmap: Option<&HashMap<String, usize>>,
    cond: &Option<Cond>,
    depth: usize,
) -> Result<(), String> {
    let line = call.line;
    if depth > 32 {
        return Err(format!("line {line}: gate definitions nest too deeply"));
    }
    let params: Vec<f64> = call
        .params
        .iter()
        .map(|e| e.eval(env))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("line {line}: {e}"))?;
    if call.name == "barrier" && qmap.is_none() {
        // a barrier spans every listed qubit (no broadcasting)
        let mut all = vec![];
        for a in &call.args {
            for g in ctx.qargs(std::slice::from_ref(a), line)? {
                all.extend(g);
            }
        }
        ctx.ops.push(Op::new("barrier", &all));
        return Ok(());
    }
    let groups: Vec<Vec<usize>> = match qmap {
        Some(m) => vec![call
            .args
            .iter()
            .map(|a| match a {
                Arg::Reg(n) => m.get(n).copied().ok_or_else(|| format!("line {line}: unknown gate argument `{n}`")),
                Arg::Bit(n, _) => Err(format!("line {line}: indexing `{n}[…]` is not allowed inside a gate body")),
            })
            .collect::<Result<_, _>>()?],
        None => ctx.qargs(&call.args, line)?,
    };
    if call.name == "barrier" {
        for g in groups {
            ctx.ops.push(Op::new("barrier", &g));
        }
        return Ok(());
    }
    if let Some(def) = ctx.defs.get(&call.name).cloned() {
        if def.params.len() != params.len() || def.args.len() != call.args.len() {
            return Err(format!(
                "line {line}: `{}` takes {} parameter(s) and {} qubit(s)",
                call.name,
                def.params.len(),
                def.args.len()
            ));
        }
        let env2: HashMap<String, f64> = def.params.iter().cloned().zip(params.iter().copied()).collect();
        for g in groups {
            let m: HashMap<String, usize> = def.args.iter().cloned().zip(g.iter().copied()).collect();
            for c in &def.body {
                expand(ctx, c, &env2, Some(&m), cond, depth + 1)?;
            }
        }
        return Ok(());
    }
    let Some((np, nq)) = builtin(&call.name) else {
        return Err(format!(
            "line {line}: gate `{}` is not supported (supported: {} and your own `gate` definitions)",
            call.name,
            QELIB.join(", ")
        ));
    };
    if params.len() != np || call.args.len() != nq {
        return Err(format!("line {line}: `{}` takes {np} parameter(s) and {nq} qubit(s)", call.name));
    }
    for g in groups {
        emit_builtin(ctx, &call.name, &params, &g, cond, line)?;
    }
    Ok(())
}

/// Parse an OpenQASM 2.0 program.
pub fn parse_qasm(src: &str) -> Result<ParseOut, String> {
    let toks = lex(src)?;
    let mut p = P { toks, i: 0 };
    let mut ctx = Ctx {
        qregs: vec![],
        cregs: vec![],
        defs: HashMap::new(),
        ops: vec![],
    };
    let mut warnings = vec![];
    let (mut nq, mut nc) = (0usize, 0usize);
    let mut saw_header = false;
    while p.peek().is_some() {
        let line = p.line();
        let word = match p.next()? {
            Tok::Id(w) => w,
            t => return Err(format!("line {line}: unexpected {}", show(&t))),
        };
        match word.as_str() {
            "OPENQASM" => {
                let v = match p.next()? {
                    Tok::Num(v) => v,
                    Tok::Int(v) => v as f64,
                    t => return Err(format!("line {line}: expected a version, found {}", show(&t))),
                };
                if (v - 2.0).abs() > 1e-9 {
                    return Err(format!(
                        "line {line}: only OpenQASM 2.0 is supported (found {v}); OpenQASM 3 adds classical types and loops"
                    ));
                }
                p.expect(";")?;
                saw_header = true;
            }
            "include" => {
                let f = match p.next()? {
                    Tok::Str(s) => s,
                    t => return Err(format!("line {line}: expected a file name, found {}", show(&t))),
                };
                if f != "qelib1.inc" {
                    return Err(format!("line {line}: only `include \"qelib1.inc\";` is supported"));
                }
                p.expect(";")?;
            }
            "qreg" | "creg" => {
                let name = p.ident()?;
                p.expect("[")?;
                let n = p.int()?;
                p.expect("]")?;
                p.expect(";")?;
                if word == "qreg" {
                    ctx.qregs.push((name, nq, n));
                    nq += n;
                } else {
                    ctx.cregs.push((name, nc, n));
                    nc += n;
                }
            }
            "gate" => {
                let name = p.ident()?;
                let mut params = vec![];
                if p.is("(") {
                    p.i += 1;
                    if !p.is(")") {
                        params.push(p.ident()?);
                        while p.is(",") {
                            p.i += 1;
                            params.push(p.ident()?);
                        }
                    }
                    p.expect(")")?;
                }
                let mut args = vec![p.ident()?];
                while p.is(",") {
                    p.i += 1;
                    args.push(p.ident()?);
                }
                p.expect("{")?;
                let mut body = vec![];
                while !p.is("}") {
                    let l = p.line();
                    let n = p.ident()?;
                    body.push(p.call(n, l)?);
                }
                p.expect("}")?;
                ctx.defs.insert(name, GateDef { params, args, body });
            }
            "opaque" => return Err(format!("line {line}: `opaque` gates have no definition and cannot be simulated")),
            "measure" => {
                let q = p.arg()?;
                p.expect("->")?;
                let cb = p.arg()?;
                p.expect(";")?;
                measure(&mut ctx, &q, &cb, &None, line)?;
            }
            "reset" => {
                let q = p.arg()?;
                p.expect(";")?;
                for g in ctx.qargs(&[q], line)? {
                    ctx.ops.push(Op::new("reset", &g));
                }
            }
            "if" => {
                p.expect("(")?;
                let reg = p.ident()?;
                p.expect("==")?;
                let v = p.int()? as u64;
                p.expect(")")?;
                let (off, size) = ctx
                    .creg(&reg)
                    .ok_or_else(|| format!("line {line}: unknown classical register `{reg}`"))?;
                let cond = Some(Cond {
                    bits: (off..off + size).collect(),
                    value: v,
                });
                let l2 = p.line();
                let name = p.ident()?;
                if name == "measure" {
                    let q = p.arg()?;
                    p.expect("->")?;
                    let cb = p.arg()?;
                    p.expect(";")?;
                    measure(&mut ctx, &q, &cb, &cond, l2)?;
                } else if name == "reset" {
                    return Err(format!("line {l2}: conditional reset is not supported"));
                } else {
                    let call = p.call(name, l2)?;
                    expand(&mut ctx, &call, &HashMap::new(), None, &cond, 0)?;
                }
            }
            _ => {
                let call = p.call(word, line)?;
                expand(&mut ctx, &call, &HashMap::new(), None, &None, 0)?;
            }
        }
    }
    if !saw_header {
        warnings.push("no `OPENQASM 2.0;` header — assumed version 2.0".into());
    }
    if nq == 0 {
        return Err("no `qreg` declared".into());
    }
    if nq > super::statevec::MAX_QUBITS {
        warnings.push(format!(
            "{nq} qubits: the simulator runs at most {} qubits",
            super::statevec::MAX_QUBITS
        ));
    }
    let count = ctx.ops.len();
    let mut circ = Circuit::from_ops(nq, nc, ctx.ops);
    circ.cregs = ctx.cregs.iter().map(|(n, _, s)| Creg { name: n.clone(), size: *s }).collect();
    Ok(ParseOut {
        circuit: circ,
        qregs: ctx.qregs.iter().map(|(n, _, s)| Creg { name: n.clone(), size: *s }).collect(),
        warnings,
        ops: count,
    })
}

fn measure(ctx: &mut Ctx, q: &Arg, cb: &Arg, cond: &Option<Cond>, line: usize) -> Result<(), String> {
    let qs = ctx.qargs(std::slice::from_ref(q), line)?;
    let cs: Vec<usize> = match cb {
        Arg::Reg(n) => {
            let (off, s) = ctx
                .creg(n)
                .ok_or_else(|| format!("line {line}: unknown classical register `{n}`"))?;
            (off..off + s).collect()
        }
        Arg::Bit(n, i) => {
            let (off, s) = ctx
                .creg(n)
                .ok_or_else(|| format!("line {line}: unknown classical register `{n}`"))?;
            if *i >= s {
                return Err(format!("line {line}: index {i} is out of range for `{n}[{s}]`"));
            }
            vec![off + i]
        }
    };
    if qs.len() != cs.len() {
        return Err(format!("line {line}: `measure` needs registers of the same size"));
    }
    for (g, c) in qs.iter().zip(cs) {
        let mut o = Op::measure(g[0], c);
        o.cond = cond.clone();
        ctx.ops.push(o);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Format an angle, preferring exact multiples of π.
pub fn angle(x: f64) -> String {
    if x.abs() < 1e-12 {
        return "0".into();
    }
    for den in [1i64, 2, 3, 4, 6, 8, 16, 32, 64, 128] {
        let k = x / PI * den as f64;
        if (k - k.round()).abs() < 1e-9 && k.round().abs() <= 4.0 * den as f64 {
            let k = k.round() as i64;
            let sign = if k < 0 { "-" } else { "" };
            let num = if k.abs() == 1 {
                "pi".to_string()
            } else {
                format!("{}*pi", k.abs())
            };
            return if den == 1 {
                format!("{sign}{num}")
            } else {
                format!("{sign}{num}/{den}")
            };
        }
    }
    let s = format!("{x:.12}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[derive(Clone, Debug, Serialize)]
pub struct EmitOut {
    pub qasm: String,
    pub warnings: Vec<String>,
}

pub fn emit_qasm(circ: &Circuit) -> Result<EmitOut, String> {
    let c = circ.validated()?;
    let mut s = String::from("OPENQASM 2.0;\ninclude \"qelib1.inc\";\n");
    let mut warnings = vec![];
    let uses_iswap = c.ops().any(|o| o.gate == "iswap");
    if uses_iswap {
        // iSWAP = (S⊗S)·(H⊗I)·CX(a→b)·CX(b→a)·(I⊗H), checked in the tests.
        s.push_str("gate iswap a,b { s a; s b; h a; cx a,b; cx b,a; h b; }\n");
    }
    s.push_str(&format!("qreg q[{}];\n", c.qubits));
    let regs = c.registers();
    let mut offs = vec![];
    let mut off = 0;
    for r in &regs {
        s.push_str(&format!("creg {}[{}];\n", r.name, r.size));
        offs.push((r.name.clone(), off, r.size));
        off += r.size;
    }
    let cname = |b: usize| -> String {
        for (n, o, sz) in &offs {
            if b >= *o && b < o + sz {
                return format!("{n}[{}]", b - o);
            }
        }
        format!("c[{b}]")
    };
    for (ci, col) in c.cols.iter().enumerate() {
        for op in col {
            let mut line = String::new();
            if let Some(cd) = &op.cond {
                let reg = offs.iter().find(|(_, o, sz)| cd.bits == (*o..o + sz).collect::<Vec<_>>());
                match reg {
                    Some((n, _, _)) => line.push_str(&format!("if({n}=={}) ", cd.value)),
                    None => {
                        return Err(format!(
                            "column {}: OpenQASM 2.0 can only condition on a whole classical register",
                            ci + 1
                        ))
                    }
                }
            }
            let q = |i: usize| format!("q[{i}]");
            let ps = if op.params.is_empty() {
                String::new()
            } else {
                format!("({})", op.params.iter().map(|x| angle(*x)).collect::<Vec<_>>().join(","))
            };
            let nctl = op.controls.len();
            let name: Option<String> = match (op.gate.as_str(), nctl) {
                ("measure", 0) => {
                    let b = op
                        .clbit
                        .ok_or_else(|| format!("column {}: measurement without a classical bit", ci + 1))?;
                    line.push_str(&format!("measure {} -> {};", q(op.targets[0]), cname(b)));
                    s.push_str(&line);
                    s.push('\n');
                    continue;
                }
                ("barrier", _) => {
                    line.push_str(&format!(
                        "barrier {};",
                        op.targets.iter().map(|&t| q(t)).collect::<Vec<_>>().join(",")
                    ));
                    s.push_str(&line);
                    s.push('\n');
                    continue;
                }
                ("reset", 0) => Some("reset".into()),
                ("p", 0) => Some("u1".into()),
                (g, 0) => Some(g.into()),
                ("x", 1) => Some("cx".into()),
                ("x", 2) => Some("ccx".into()),
                ("x", 3) => Some("c3x".into()),
                ("x", 4) => Some("c4x".into()),
                ("y", 1) => Some("cy".into()),
                ("z", 1) => Some("cz".into()),
                ("h", 1) => Some("ch".into()),
                ("sx", 1) => Some("csx".into()),
                ("rx", 1) => Some("crx".into()),
                ("ry", 1) => Some("cry".into()),
                ("rz", 1) => Some("crz".into()),
                ("p", 1) => Some("cu1".into()),
                ("u3", 1) => Some("cu3".into()),
                ("swap", 1) => Some("cswap".into()),
                ("s", 1) | ("sdg", 1) | ("t", 1) | ("tdg", 1) => {
                    let lam = match op.gate.as_str() {
                        "s" => PI / 2.0,
                        "sdg" => -PI / 2.0,
                        "t" => PI / 4.0,
                        _ => -PI / 4.0,
                    };
                    line.push_str(&format!("cu1({}) {},{};", angle(lam), q(op.controls[0]), q(op.targets[0])));
                    s.push_str(&line);
                    s.push('\n');
                    continue;
                }
                ("z", 2) => {
                    // CCZ = H·CCX·H on the target
                    let t = op.targets[0];
                    let pre = line.clone();
                    s.push_str(&format!(
                        "{pre}h {};\n{pre}ccx {},{},{};\n{pre}h {};\n",
                        q(t),
                        q(op.controls[0]),
                        q(op.controls[1]),
                        q(t),
                        q(t)
                    ));
                    continue;
                }
                _ => None,
            };
            let Some(name) = name else {
                return Err(format!(
                    "column {}: `{}` with {nctl} control(s) has no OpenQASM 2.0 qelib1 equivalent — decompose it first",
                    ci + 1,
                    op.gate
                ));
            };
            if name == "id" {
                continue;
            }
            let qs: Vec<String> = op.controls.iter().chain(&op.targets).map(|&i| q(i)).collect();
            line.push_str(&format!("{name}{ps} {};", qs.join(",")));
            s.push_str(&line);
            s.push('\n');
        }
    }
    if c.ops().any(|o| o.gate == "sx" && !o.controls.is_empty()) {
        warnings.push("`csx` is in Qiskit's qelib1.inc but not in the original 2017 file".into());
    }
    Ok(EmitOut { qasm: s, warnings })
}

// ---------------------------------------------------------------------------
// OpenQASM 3.0 and Qiskit
// ---------------------------------------------------------------------------

/// Classical register layout: `(name, first flat bit, size)`.
fn creg_offsets(c: &Circuit) -> Vec<(String, usize, usize)> {
    let mut offs = vec![];
    let mut off = 0;
    for r in c.registers() {
        offs.push((r.name.clone(), off, r.size));
        off += r.size;
    }
    offs
}

/// `(register name, index)` of flat classical bit `b`.
fn creg_bit(offs: &[(String, usize, usize)], b: usize) -> (String, usize) {
    for (n, o, sz) in offs {
        if b >= *o && b < o + sz {
            return (n.clone(), b - o);
        }
    }
    ("c".into(), b)
}

/// Whole register matching a condition's bits, if any.
fn cond_register<'a>(offs: &'a [(String, usize, usize)], cd: &Cond) -> Option<&'a str> {
    offs.iter()
        .find(|(_, o, sz)| cd.bits == (*o..o + sz).collect::<Vec<_>>())
        .map(|(n, _, _)| n.as_str())
}

/// Write the circuit as an OpenQASM 3.0 program (`stdgates.inc`).
///
/// Follows the OpenQASM 3 specification (A. W. Cross, A. Javadi-Abhari,
/// T. Alexander et al., "OpenQASM 3: A broader and deeper quantum assembly
/// language", ACM Trans. Quantum Comput. 3 (3), 2022): `qubit[n] q;`,
/// `bit[m] c;`, `c[0] = measure q[0];`, `if (c == 5) …`, and the `ctrl(k) @`
/// / `inv @` gate modifiers for controlled gates without a `stdgates.inc`
/// name. The IBM Quantum Composer and `qiskit.qasm3.loads` (which needs the
/// `qiskit-qasm3-import` package) read these programs.
pub fn emit_qasm3(circ: &Circuit) -> Result<EmitOut, String> {
    let c = circ.validated()?;
    let mut s = String::from("OPENQASM 3.0;\ninclude \"stdgates.inc\";\n");
    let warnings = vec![];
    if c.ops().any(|o| o.gate == "iswap") {
        s.push_str("gate iswap a, b { s a; s b; h a; cx a, b; cx b, a; h b; }\n");
    }
    s.push_str(&format!("qubit[{}] q;\n", c.qubits));
    let offs = creg_offsets(&c);
    for (n, _, sz) in &offs {
        s.push_str(&format!("bit[{sz}] {n};\n"));
    }
    let q = |i: usize| format!("q[{i}]");
    for col in &c.cols {
        for op in col {
            let mut pre = String::new();
            if let Some(cd) = &op.cond {
                match cond_register(&offs, cd) {
                    Some(n) => pre = format!("if ({n} == {}) ", cd.value),
                    None => {
                        let terms: Vec<String> = cd
                            .bits
                            .iter()
                            .enumerate()
                            .map(|(k, &b)| {
                                let (n, i) = creg_bit(&offs, b);
                                format!("{n}[{i}] == {}", (cd.value >> k) & 1)
                            })
                            .collect();
                        pre = format!("if ({}) ", terms.join(" && "));
                    }
                }
            }
            let line = match op.gate.as_str() {
                "measure" => {
                    let b = op.clbit.ok_or("measurement without a classical bit")?;
                    let (n, i) = creg_bit(&offs, b);
                    format!("{n}[{i}] = measure {};", q(op.targets[0]))
                }
                "barrier" => format!("barrier {};", op.targets.iter().map(|&t| q(t)).collect::<Vec<_>>().join(", ")),
                "reset" => format!("reset {};", q(op.targets[0])),
                "id" if op.controls.is_empty() => continue,
                g => {
                    let ps = if op.params.is_empty() {
                        String::new()
                    } else {
                        format!("({})", op.params.iter().map(|x| angle(*x)).collect::<Vec<_>>().join(", "))
                    };
                    let nc = op.controls.len();
                    // names in stdgates.inc, else modifiers
                    let (name, mods) = match (g, nc) {
                        ("sxdg", 0) => ("sx".to_string(), "inv @ ".to_string()),
                        (g, 0) => (g.to_string(), String::new()),
                        ("x", 1) => ("cx".into(), String::new()),
                        ("x", 2) => ("ccx".into(), String::new()),
                        ("y", 1) => ("cy".into(), String::new()),
                        ("z", 1) => ("cz".into(), String::new()),
                        ("h", 1) => ("ch".into(), String::new()),
                        ("p", 1) => ("cp".into(), String::new()),
                        ("rx", 1) => ("crx".into(), String::new()),
                        ("ry", 1) => ("cry".into(), String::new()),
                        ("rz", 1) => ("crz".into(), String::new()),
                        ("swap", 1) => ("cswap".into(), String::new()),
                        ("sxdg", k) => (
                            "sx".into(),
                            format!("{} @ inv @ ", if k == 1 { "ctrl".into() } else { format!("ctrl({k})") }),
                        ),
                        (g, k) => (g.to_string(), if k == 1 { "ctrl @ ".into() } else { format!("ctrl({k}) @ ") }),
                    };
                    let qs: Vec<String> = op.controls.iter().chain(&op.targets).map(|&i| q(i)).collect();
                    format!("{mods}{name}{ps} {};", qs.join(", "))
                }
            };
            s.push_str(&pre);
            s.push_str(&line);
            s.push('\n');
        }
    }
    Ok(EmitOut { qasm: s, warnings })
}

/// Write the circuit as a Qiskit (≥ 1.0) Python script that builds the same
/// `QuantumCircuit`, draws it and samples it on the Aer simulator.
///
/// Classical conditions use `QuantumCircuit.if_test` (the dynamic-circuit
/// form that replaced `c_if`); gates with more controls than Qiskit has a
/// method for use `Gate.control(k)`.
pub fn emit_qiskit(circ: &Circuit) -> Result<EmitOut, String> {
    let c = circ.validated()?;
    let offs = creg_offsets(&c);
    let mut s = String::new();
    s.push_str(&format!("q = QuantumRegister({}, \"q\")\n", c.qubits));
    let mut regs = vec!["q".to_string()];
    for (n, _, sz) in &offs {
        s.push_str(&format!("{n} = ClassicalRegister({sz}, \"{n}\")\n"));
        regs.push(n.clone());
    }
    s.push_str(&format!("qc = QuantumCircuit({})\n\n", regs.join(", ")));
    let cls = |g: &str| -> &'static str {
        match g {
            "h" => "HGate",
            "x" => "XGate",
            "y" => "YGate",
            "z" => "ZGate",
            "s" => "SGate",
            "sdg" => "SdgGate",
            "t" => "TGate",
            "tdg" => "TdgGate",
            "sx" => "SXGate",
            "sxdg" => "SXdgGate",
            "rx" => "RXGate",
            "ry" => "RYGate",
            "rz" => "RZGate",
            "p" => "PhaseGate",
            "u3" => "UGate",
            "swap" => "SwapGate",
            _ => "iSwapGate",
        }
    };
    let list = |v: &[usize]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
    let mut has_measure = false;
    let mut used: Vec<&'static str> = vec![];
    let mut has_pi = false;
    for col in &c.cols {
        for op in col {
            let ps: Vec<String> = op.params.iter().map(|x| angle(*x)).collect();
            let pj = ps.join(", ");
            has_pi |= pj.contains("pi");
            let nc = op.controls.len();
            let tl = list(&op.targets);
            let call = match (op.gate.as_str(), nc) {
                ("measure", _) => {
                    has_measure = true;
                    let (n, i) = creg_bit(&offs, op.clbit.ok_or("measurement without a classical bit")?);
                    format!("qc.measure(q[{}], {n}[{i}])", op.targets[0])
                }
                ("barrier", _) => format!("qc.barrier({tl})"),
                ("reset", _) => format!("qc.reset({tl})"),
                ("id", 0) => format!("qc.id({tl})"),
                ("u3", 0) => format!("qc.u({pj}, {tl})"),
                (g @ ("h" | "x" | "y" | "z" | "s" | "sdg" | "t" | "tdg" | "sx" | "sxdg" | "swap" | "iswap"), 0) => format!("qc.{g}({tl})"),
                (g @ ("rx" | "ry" | "rz" | "p"), 0) => format!("qc.{g}({pj}, {tl})"),
                (g @ ("x" | "y" | "z" | "h" | "sx" | "swap"), 1) => {
                    let m = if g == "swap" { "cswap".to_string() } else { format!("c{g}") };
                    format!("qc.{m}({}, {tl})", op.controls[0])
                }
                (g @ ("rx" | "ry" | "rz" | "p"), 1) => format!("qc.c{g}({pj}, {}, {tl})", op.controls[0]),
                ("u3", 1) => format!("qc.cu({pj}, 0, {}, {tl})", op.controls[0]),
                ("x", 2) => format!("qc.ccx({}, {tl})", list(&op.controls)),
                ("x", _) => format!("qc.mcx([{}], {tl})", list(&op.controls)),
                (g, k) => {
                    let args = if ps.is_empty() { String::new() } else { pj.clone() };
                    if !used.contains(&cls(g)) {
                        used.push(cls(g));
                    }
                    format!("qc.append({}({args}).control({k}), [{}, {tl}])", cls(g), list(&op.controls))
                }
            };
            match &op.cond {
                None => s.push_str(&call),
                Some(cd) => {
                    let mut indent = String::new();
                    if let Some(n) = cond_register(&offs, cd) {
                        s.push_str(&format!("with qc.if_test(({n}, {})):\n    ", cd.value));
                    } else {
                        for (k, &b) in cd.bits.iter().enumerate() {
                            let (n, i) = creg_bit(&offs, b);
                            s.push_str(&format!("{indent}with qc.if_test(({n}[{i}], {})):\n", (cd.value >> k) & 1));
                            indent.push_str("    ");
                        }
                        s.push_str(&indent);
                    }
                    s.push_str(&call);
                }
            }
            s.push('\n');
        }
    }
    s.push_str("\nprint(qc.draw())\n");
    if has_measure {
        s.push_str(
            "\nfrom qiskit_aer import AerSimulator\n\nsim = AerSimulator()\ncounts = sim.run(transpile(qc, sim), shots=1024).result().get_counts()\nprint(counts)  # keys read c[n-1] … c[0], like this lab\n",
        );
    } else {
        s.push_str("\nfrom qiskit.quantum_info import Statevector\n\nprint(Statevector.from_instruction(qc).probabilities_dict())\n");
    }
    let mut head = String::from("# Generated by the Protoforge Quantum Lab. Needs: pip install qiskit qiskit-aer\n");
    if has_pi {
        head.push_str("from math import pi\n\n");
    }
    head.push_str(&format!(
        "from qiskit import ClassicalRegister, QuantumCircuit, QuantumRegister{}\n",
        if has_measure { ", transpile" } else { "" }
    ));
    if !used.is_empty() {
        head.push_str(&format!("from qiskit.circuit.library import {}\n", used.join(", ")));
    }
    head.push('\n');
    Ok(EmitOut {
        qasm: head + &s,
        warnings: vec![],
    })
}

// ---------------------------------------------------------------------------
// Quirk
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct QuirkOut {
    pub url: String,
    pub json: String,
    pub warnings: Vec<String>,
}

fn cplx(z: crate::math::C64) -> String {
    let f = |x: f64| {
        let s = format!("{:.6}", x);
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s == "-0" {
            "0".to_string()
        } else {
            s
        }
    };
    let (re, im) = (f(z.re), f(z.im));
    match (re.as_str(), im.as_str()) {
        (_, "0") => re,
        ("0", _) => format!("{im}i"),
        _ => {
            if im.starts_with('-') {
                format!("{re}{im}i")
            } else {
                format!("{re}+{im}i")
            }
        }
    }
}

fn quirk_id(op: &Op, customs: &mut Vec<(String, String, String)>) -> String {
    let fixed = match op.gate.as_str() {
        "h" => Some("H"),
        "x" => Some("X"),
        "y" => Some("Y"),
        "z" => Some("Z"),
        "s" => Some("Z^½"),
        "sdg" => Some("Z^-½"),
        "t" => Some("Z^¼"),
        "tdg" => Some("Z^-¼"),
        "sx" => Some("X^½"),
        "sxdg" => Some("X^-½"),
        "measure" => Some("Measure"),
        "swap" => Some("Swap"),
        _ => None,
    };
    if let Some(f) = fixed {
        return format!("\"{f}\"");
    }
    match op.gate.as_str() {
        "rx" | "ry" | "rz" => {
            let id = match op.gate.as_str() {
                "rx" => "Rxft",
                "ry" => "Ryft",
                _ => "Rzft",
            };
            return format!("{{\"id\":\"{id}\",\"arg\":\"{}\"}}", fmt_arg(op.params[0]));
        }
        _ => {}
    }
    let m = matrix(&op.gate, &op.params).unwrap_or(matrix("id", &[]).unwrap());
    let mat = format!(
        "{{{{{},{}}},{{{},{}}}}}",
        cplx(m[0][0]),
        cplx(m[0][1]),
        cplx(m[1][0]),
        cplx(m[1][1])
    );
    let name = match op.gate.as_str() {
        "p" => format!("P({})", angle(op.params[0])),
        "u3" => "U".to_string(),
        g => g.to_uppercase(),
    };
    if let Some(k) = customs.iter().position(|(_, _, mm)| *mm == mat) {
        return format!("\"~g{k}\"");
    }
    customs.push((format!("~g{}", customs.len()), name, mat));
    format!("\"~g{}\"", customs.len() - 1)
}

fn fmt_arg(x: f64) -> String {
    let s = format!("{x:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn quirk(circ: &Circuit) -> Result<QuirkOut, String> {
    let c = circ.validated()?;
    let n = c.qubits;
    let mut warnings = vec![];
    let mut cols: Vec<Vec<String>> = vec![];
    let mut customs: Vec<(String, String, String)> = vec![];
    // clbit → the qubit last measured into it (Quirk uses measured wires as classical controls)
    let mut writer: HashMap<usize, usize> = HashMap::new();
    let mut warned_reset = false;
    for col in &c.cols {
        // split so that controls in one Quirk column apply to one op
        let mut groups: Vec<Vec<&Op>> = vec![];
        let mut plain: Vec<&Op> = vec![];
        for op in col {
            let solo = !op.controls.is_empty() || op.cond.is_some() || op.gate == "iswap";
            if solo {
                groups.push(vec![op]);
            } else {
                plain.push(op);
            }
        }
        if !plain.is_empty() {
            groups.insert(0, plain);
        }
        for g in groups {
            let mut cells = vec!["1".to_string(); n];
            let mut any = false;
            for op in g {
                match op.gate.as_str() {
                    "barrier" | "id" => {}
                    "reset" => {
                        if !warned_reset {
                            warnings.push("Quirk has no reset gate; resets were left out".into());
                            warned_reset = true;
                        }
                    }
                    "iswap" => {
                        // iSWAP = (S⊗S)·(H⊗I)·CX(a→b)·CX(b→a)·(I⊗H) as separate Quirk columns
                        let (a, b) = (op.targets[0], op.targets[1]);
                        let mut push = |pairs: &[(usize, &str)]| {
                            let mut cc = vec!["1".to_string(); n];
                            for (q, g) in pairs {
                                cc[*q] = format!("\"{g}\"");
                            }
                            cols.push(cc);
                        };
                        push(&[(b, "H")]);
                        push(&[(b, "•"), (a, "X")]);
                        push(&[(a, "•"), (b, "X")]);
                        push(&[(a, "H")]);
                        push(&[(a, "Z^½"), (b, "Z^½")]);
                        if !op.controls.is_empty() || op.cond.is_some() {
                            warnings.push("controlled iSWAP was exported without its controls".into());
                        }
                    }
                    _ => {
                        let id = quirk_id(op, &mut customs);
                        for &t in &op.targets {
                            cells[t] = id.clone();
                        }
                        for &ctl in &op.controls {
                            cells[ctl] = "\"•\"".into();
                        }
                        if let Some(cd) = &op.cond {
                            // only single-bit conditions translate to Quirk's classical controls
                            if cd.bits.len() == 1 && cd.value == 1 {
                                match writer.get(&cd.bits[0]) {
                                    Some(&mq) if cells[mq] == "1" => cells[mq] = "\"•\"".into(),
                                    _ => warnings.push("a classical condition could not be mapped to a measured wire".into()),
                                }
                            } else {
                                warnings.push("multi-bit classical conditions are not exported to Quirk".into());
                            }
                        }
                        if op.gate == "measure" {
                            if let Some(b) = op.clbit {
                                writer.insert(b, op.targets[0]);
                            }
                        }
                        any = true;
                    }
                }
            }
            if any {
                cols.push(cells);
            }
        }
    }
    let cols_json: Vec<String> = cols
        .iter()
        .map(|cc| {
            let mut v = cc.clone();
            while v.last().is_some_and(|x| x == "1") {
                v.pop();
            }
            format!("[{}]", v.join(","))
        })
        .collect();
    let mut json = format!("{{\"cols\":[{}]", cols_json.join(","));
    if !customs.is_empty() {
        let gs: Vec<String> = customs
            .iter()
            .map(|(id, name, m)| format!("{{\"id\":\"{id}\",\"name\":\"{name}\",\"matrix\":\"{m}\"}}"))
            .collect();
        json.push_str(&format!(",\"gates\":[{}]", gs.join(",")));
    }
    json.push('}');
    Ok(QuirkOut {
        url: format!("https://algassert.com/quirk#circuit={}", encode_uri_component(&json)),
        json,
        warnings,
    })
}

/// Percent-encode like JavaScript's `encodeURIComponent`.
pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        let keep = b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b);
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::C64;
    use crate::quantum::statevec::final_state;

    const BELL: &str = r#"OPENQASM 2.0;
include "qelib1.inc";
// Bell pair
qreg q[2];
creg c[2];
h q[0];
cx q[0],q[1];
measure q -> c;
"#;

    #[test]
    fn parses_bell_and_round_trips() {
        let p = parse_qasm(BELL).unwrap();
        assert_eq!(p.circuit.qubits, 2);
        assert_eq!(p.circuit.clbits, 2);
        assert_eq!(p.ops, 4);
        let e = emit_qasm(&p.circuit).unwrap();
        assert!(e.qasm.contains("cx q[0],q[1];"));
        assert!(e.qasm.contains("measure q[1] -> c[1];"));
        let again = parse_qasm(&e.qasm).unwrap();
        assert_eq!(again.circuit, p.circuit);
    }

    #[test]
    fn gate_definitions_expressions_and_errors() {
        let src = "OPENQASM 2.0; include \"qelib1.inc\"; qreg a[1]; qreg b[2];\n\
                   gate mine(theta) x, y { ry(theta/2) x; cx x, y; u2(0, pi) y; }\n\
                   mine(pi) a[0], b[1];\nrzz(-pi/4) b[0], b[1];\nbarrier a, b;";
        let p = parse_qasm(src).unwrap();
        assert_eq!(p.circuit.qubits, 3);
        let first = p.circuit.ops().next().unwrap();
        assert_eq!(first.gate, "ry");
        assert!((first.params[0] - PI / 2.0).abs() < 1e-12);
        let e = parse_qasm("OPENQASM 2.0;\nqreg q[1];\nfoo q[0];").unwrap_err();
        assert!(e.contains("line 3") && e.contains("foo"), "{e}");
        let e = parse_qasm("OPENQASM 2.0;\nqreg q[1];\nh q[3];").unwrap_err();
        assert!(e.contains("out of range"), "{e}");
        assert!(parse_qasm("OPENQASM 3.0; qubit q;").is_err());
    }

    #[test]
    fn qasm3_and_qiskit_writers() {
        // teleportation-style feed-forward, a Toffoli, a controlled S, a 3-control X and √X†
        let mut c = Circuit::new(4, 2);
        c.cols = vec![
            vec![Op::new("h", &[0]), Op::new("sxdg", &[3])],
            vec![Op::ctrl("x", &[0], &[1])],
            vec![Op::measure(0, 0), Op::measure(1, 1)],
            vec![Op::new("x", &[2]).when(&[1], 1)],
            vec![Op::new("z", &[2]).when(&[0, 1], 3)],
            vec![Op::ctrl("x", &[0, 1], &[2])],
            vec![Op::ctrl("s", &[0], &[3])],
            vec![Op::ctrl("x", &[0, 1, 2], &[3])],
            vec![Op::ctrl("rz", &[1], &[2]).param(&[PI / 4.0])],
        ];
        let q3 = emit_qasm3(&c).unwrap().qasm;
        for want in [
            "OPENQASM 3.0;",
            "include \"stdgates.inc\";",
            "qubit[4] q;",
            "bit[2] c;",
            "inv @ sx q[3];",
            "cx q[0], q[1];",
            "c[0] = measure q[0];",
            "if (c[1] == 1) x q[2];",
            "if (c == 3) z q[2];",
            "ccx q[0], q[1], q[2];",
            "ctrl @ s q[0], q[3];",
            "ctrl(3) @ x q[0], q[1], q[2], q[3];",
            "crz(pi/4) q[1], q[2];",
        ] {
            assert!(q3.contains(want), "missing `{want}` in\n{q3}");
        }
        let py = emit_qiskit(&c).unwrap().qasm;
        for want in [
            "q = QuantumRegister(4, \"q\")",
            "c = ClassicalRegister(2, \"c\")",
            "qc = QuantumCircuit(q, c)",
            "qc.sxdg(3)",
            "qc.cx(0, 1)",
            "qc.measure(q[0], c[0])",
            "with qc.if_test((c[1], 1)):\n    qc.x(2)",
            "with qc.if_test((c, 3)):\n    qc.z(2)",
            "qc.ccx(0, 1, 2)",
            "qc.append(SGate().control(1), [0, 3])",
            "qc.mcx([0, 1, 2], 3)",
            "qc.crz(pi/4, 1, 2)",
            "AerSimulator",
            "from math import pi",
            "from qiskit.circuit.library import SGate\n",
        ] {
            assert!(py.contains(want), "missing `{want}` in\n{py}");
        }
        // a partial-register condition becomes nested if_tests / a conjunction
        let mut c2 = Circuit::new(2, 3);
        c2.cols = vec![vec![Op::new("x", &[1]).when(&[0, 2], 2)]];
        assert!(emit_qasm3(&c2).unwrap().qasm.contains("if (c[0] == 0 && c[2] == 1) x q[1];"));
        assert!(emit_qiskit(&c2)
            .unwrap()
            .qasm
            .contains("with qc.if_test((c[0], 0)):\n    with qc.if_test((c[2], 1)):\n        qc.x(1)"));
        // no measurement: the script prints the exact probabilities instead
        assert!(emit_qiskit(&Circuit::from_ops(1, 0, vec![Op::new("h", &[0])]))
            .unwrap()
            .qasm
            .contains("Statevector"));
    }

    #[test]
    fn angles_print_as_pi_fractions() {
        assert_eq!(angle(PI / 4.0), "pi/4");
        assert_eq!(angle(-3.0 * PI / 4.0), "-3*pi/4");
        assert_eq!(angle(PI), "pi");
        assert_eq!(angle(0.3), "0.3");
    }

    #[test]
    fn iswap_decomposition_is_exact() {
        let src = "OPENQASM 2.0; include \"qelib1.inc\"; gate iswap a,b { s a; s b; h a; cx a,b; cx b,a; h b; } qreg q[2];";
        for input in 0..4usize {
            let mut prep = String::from(src);
            if input & 1 == 1 {
                prep.push_str("x q[0];");
            }
            if input & 2 == 2 {
                prep.push_str("x q[1];");
            }
            prep.push_str("iswap q[0],q[1];");
            let sv = final_state(&parse_qasm(&prep).unwrap().circuit, 1).unwrap();
            let mut direct = Circuit::new(2, 0);
            let mut ops = vec![];
            if input & 1 == 1 {
                ops.push(Op::new("x", &[0]));
            }
            if input & 2 == 2 {
                ops.push(Op::new("x", &[1]));
            }
            ops.push(Op::new("iswap", &[0, 1]));
            direct.cols = vec![ops];
            let want = final_state(&Circuit::from_ops(2, 0, direct.cols[0].clone()), 1).unwrap();
            // equal up to a global phase
            let k = (0..4)
                .max_by(|&a, &b| want.amps[a].abs().partial_cmp(&want.amps[b].abs()).unwrap())
                .unwrap();
            let ph = sv.amps[k] / want.amps[k];
            assert!((ph.abs() - 1.0).abs() < 1e-9);
            for i in 0..4 {
                assert!((sv.amps[i] - want.amps[i] * ph).abs() < 1e-9, "input {input} amp {i}");
            }
        }
        let _ = C64::ZERO;
    }

    #[test]
    fn quirk_link_uses_known_ids() {
        let p = parse_qasm(BELL).unwrap();
        let q = quirk(&p.circuit).unwrap();
        assert_eq!(q.json, r#"{"cols":[["H"],["•","X"],["Measure","Measure"]]}"#);
        assert!(q.url.starts_with("https://algassert.com/quirk#circuit=%7B%22cols%22"));
        let c = Circuit::from_ops(1, 0, vec![Op::new("rx", &[0]).param(&[0.5]), Op::new("p", &[0]).param(&[PI / 3.0])]);
        let q = quirk(&c).unwrap();
        assert!(q.json.contains(r#"{"id":"Rxft","arg":"0.5"}"#), "{}", q.json);
        assert!(q.json.contains(r#""gates":[{"id":"~g0""#), "{}", q.json);
    }
}
