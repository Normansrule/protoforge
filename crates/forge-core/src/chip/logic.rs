//! Two-level logic minimisation.
//!
//! 1. Parse a Boolean expression (`&` `*` AND, `|` `+` OR, `^` XOR, `!` `~`
//!    prefix NOT, `'` postfix NOT, parentheses, implicit AND between adjacent
//!    factors, constants 0/1) — or take minterms directly.
//! 2. Enumerate the truth table (≤ 8 variables; variable A is the MSB).
//! 3. **Quine–McCluskey**: repeatedly merge implicants that differ in one
//!    bit to find every prime implicant (W. V. Quine, "The problem of
//!    simplifying truth functions", Amer. Math. Monthly 1952; E. J.
//!    McCluskey, "Minimization of Boolean functions", Bell Syst. Tech. J.
//!    1956).
//! 4. **Minimum cover**: essential primes first, then **Petrick's method**
//!    (S. R. Petrick, 1956) — the cover condition as a product of sums,
//!    multiplied out with absorption — choosing the product with the fewest
//!    implicants, then fewest literals. A greedy cover is the fallback when
//!    the expansion would explode.
//! 5. POS = De Morgan of the minimum SOP of the complement.
//! 6. For ≤ 4 variables, Karnaugh-map layout in Gray-code order (M.
//!    Karnaugh, "The map method for synthesis of combinational logic
//!    circuits", Trans. AIEE 1953) with each chosen implicant's rectangles.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// ------------------------------------------------------------------ parser

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Var(usize),
    Const(bool),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Xor(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn eval(&self, v: &[bool]) -> bool {
        match self {
            Expr::Var(i) => v[*i],
            Expr::Const(c) => *c,
            Expr::Not(e) => !e.eval(v),
            Expr::And(a, b) => a.eval(v) && b.eval(v),
            Expr::Or(a, b) => a.eval(v) || b.eval(v),
            Expr::Xor(a, b) => a.eval(v) ^ b.eval(v),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Id(String),
    Const(bool),
    And,
    Or,
    Xor,
    Not,
    Prime,
    LParen,
    RParen,
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        match ch {
            ' ' | '\t' | '\n' | '\r' => {}
            '&' | '*' | '·' | '∧' => out.push(Tok::And),
            '|' | '+' | '∨' => out.push(Tok::Or),
            '^' | '⊕' => out.push(Tok::Xor),
            '!' | '~' | '¬' => out.push(Tok::Not),
            '\'' | '’' => out.push(Tok::Prime),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '0' => out.push(Tok::Const(false)),
            '1' => out.push(Tok::Const(true)),
            _ if ch.is_ascii_alphabetic() || ch == '_' => {
                let st = i;
                while i + 1 < c.len() && (c[i + 1].is_ascii_alphanumeric() || c[i + 1] == '_') {
                    i += 1;
                }
                out.push(Tok::Id(c[st..=i].iter().collect()));
            }
            _ => return Err(format!("unexpected character `{ch}` at position {}", i + 1)),
        }
        // collapse doubled operators like && and ||
        if out.len() >= 2 {
            let n = out.len();
            if (out[n - 1] == Tok::And && out[n - 2] == Tok::And) || (out[n - 1] == Tok::Or && out[n - 2] == Tok::Or) {
                out.pop();
            }
        }
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    vars: Vec<String>,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn var(&mut self, name: &str) -> usize {
        if let Some(i) = self.vars.iter().position(|v| v == name) {
            i
        } else {
            self.vars.push(name.to_string());
            self.vars.len() - 1
        }
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut e = self.xor()?;
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            e = Expr::Or(Box::new(e), Box::new(self.xor()?));
        }
        Ok(e)
    }
    fn xor(&mut self) -> Result<Expr, String> {
        let mut e = self.and()?;
        while self.peek() == Some(&Tok::Xor) {
            self.pos += 1;
            e = Expr::Xor(Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        loop {
            match self.peek() {
                Some(Tok::And) => {
                    self.pos += 1;
                    e = Expr::And(Box::new(e), Box::new(self.unary()?));
                }
                // implicit AND: a b, a(b+c), (a)(b), a !b
                Some(Tok::Id(_)) | Some(Tok::LParen) | Some(Tok::Not) | Some(Tok::Const(_)) => {
                    e = Expr::And(Box::new(e), Box::new(self.unary()?));
                }
                _ => break,
            }
        }
        Ok(e)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        let mut e = match self.peek().cloned() {
            Some(Tok::Not) => {
                self.pos += 1;
                Expr::Not(Box::new(self.unary()?))
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let e = self.or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err("missing `)`".into());
                }
                self.pos += 1;
                e
            }
            Some(Tok::Id(n)) => {
                self.pos += 1;
                Expr::Var(self.var(&n))
            }
            Some(Tok::Const(c)) => {
                self.pos += 1;
                Expr::Const(c)
            }
            Some(t) => return Err(format!("unexpected {t:?}")),
            None => return Err("unexpected end of expression".into()),
        };
        while self.peek() == Some(&Tok::Prime) {
            self.pos += 1;
            e = Expr::Not(Box::new(e));
        }
        Ok(e)
    }
}

/// Parse an expression. Variables are renumbered in alphabetical order.
pub fn parse(s: &str) -> Result<(Expr, Vec<String>), String> {
    let toks = lex(s)?;
    if toks.is_empty() {
        return Err("empty expression".into());
    }
    let mut p = Parser { toks, pos: 0, vars: vec![] };
    let e = p.or()?;
    if p.pos != p.toks.len() {
        return Err(format!("unexpected token {:?}", p.toks[p.pos]));
    }
    if p.vars.len() > 8 {
        return Err(format!("{} variables — the minimiser supports at most 8", p.vars.len()));
    }
    // alphabetical renumbering
    let mut sorted = p.vars.clone();
    sorted.sort();
    let map: Vec<usize> = p.vars.iter().map(|v| sorted.iter().position(|s| s == v).unwrap()).collect();
    fn remap(e: Expr, m: &[usize]) -> Expr {
        match e {
            Expr::Var(i) => Expr::Var(m[i]),
            Expr::Const(c) => Expr::Const(c),
            Expr::Not(a) => Expr::Not(Box::new(remap(*a, m))),
            Expr::And(a, b) => Expr::And(Box::new(remap(*a, m)), Box::new(remap(*b, m))),
            Expr::Or(a, b) => Expr::Or(Box::new(remap(*a, m)), Box::new(remap(*b, m))),
            Expr::Xor(a, b) => Expr::Xor(Box::new(remap(*a, m)), Box::new(remap(*b, m))),
        }
    }
    Ok((remap(e, &map), sorted))
}

// ------------------------------------------------------------------ QM

/// An implicant: `mask` bits are don't-care positions; `value` holds the
/// fixed bits (bit n−1−i ↔ variable i, so variable A is the MSB).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Implicant {
    pub value: u32,
    pub mask: u32,
}

impl Implicant {
    pub fn covers(&self, m: u32) -> bool {
        (m & !self.mask) == (self.value & !self.mask)
    }
    pub fn literals(&self, n: usize) -> usize {
        n - (self.mask.count_ones() as usize)
    }
    pub fn minterms(&self, n: usize) -> Vec<u32> {
        (0..(1u32 << n)).filter(|&m| self.covers(m)).collect()
    }
    /// Pattern like "1-0" (MSB first).
    pub fn pattern(&self, n: usize) -> String {
        (0..n)
            .map(|i| {
                let bit = 1 << (n - 1 - i);
                if self.mask & bit != 0 {
                    '-'
                } else if self.value & bit != 0 {
                    '1'
                } else {
                    '0'
                }
            })
            .collect()
    }
    /// Product term, e.g. "A B'" ("1" for the all-don't-care implicant).
    pub fn product(&self, vars: &[String], style: Style) -> String {
        let n = vars.len();
        let mut lits = Vec::new();
        for (i, v) in vars.iter().enumerate() {
            let bit = 1 << (n - 1 - i);
            if self.mask & bit == 0 {
                let pos = self.value & bit != 0;
                lits.push(match (style, pos) {
                    (Style::Math, true) => v.clone(),
                    (Style::Math, false) => format!("{v}'"),
                    (Style::Verilog, true) => v.clone(),
                    (Style::Verilog, false) => format!("~{v}"),
                });
            }
        }
        if lits.is_empty() {
            return match style {
                Style::Math => "1".into(),
                Style::Verilog => "1'b1".into(),
            };
        }
        match style {
            Style::Math => lits.join(""),
            Style::Verilog => lits.join(" & "),
        }
    }
    /// Sum term for the POS form (complemented literals of an implicant of F').
    pub fn sum_of_complement(&self, vars: &[String], style: Style) -> String {
        let n = vars.len();
        let mut lits = Vec::new();
        for (i, v) in vars.iter().enumerate() {
            let bit = 1 << (n - 1 - i);
            if self.mask & bit == 0 {
                let pos = self.value & bit == 0; // complemented
                lits.push(match (style, pos) {
                    (Style::Math, true) => v.clone(),
                    (Style::Math, false) => format!("{v}'"),
                    (Style::Verilog, true) => v.clone(),
                    (Style::Verilog, false) => format!("~{v}"),
                });
            }
        }
        if lits.is_empty() {
            return match style {
                Style::Math => "0".into(),
                Style::Verilog => "1'b0".into(),
            };
        }
        let sep = match style {
            Style::Math => " + ",
            Style::Verilog => " | ",
        };
        format!("({})", lits.join(sep))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Math,
    Verilog,
}

/// Quine–McCluskey prime implicant generation. Returns primes plus the merge
/// "tables" (one column per round) for teaching.
pub fn prime_implicants(n: usize, on: &[u32], dc: &[u32]) -> (Vec<Implicant>, Vec<Vec<Implicant>>) {
    let mut current: BTreeSet<Implicant> = on.iter().chain(dc.iter()).map(|&m| Implicant { value: m, mask: 0 }).collect();
    let mut primes: BTreeSet<Implicant> = BTreeSet::new();
    let mut columns = Vec::new();
    let full = if n >= 32 { u32::MAX } else { (1u32 << n) - 1 };
    while !current.is_empty() {
        columns.push(current.iter().cloned().collect::<Vec<_>>());
        let list: Vec<Implicant> = current.iter().cloned().collect();
        let mut used = vec![false; list.len()];
        let mut next = BTreeSet::new();
        // group by mask and popcount for the pairwise merge
        let mut groups: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        for (i, imp) in list.iter().enumerate() {
            groups.entry((imp.mask, (imp.value & !imp.mask).count_ones())).or_default().push(i);
        }
        for ((mask, ones), idx) in &groups {
            if let Some(nidx) = groups.get(&(*mask, ones + 1)) {
                for &i in idx {
                    for &j in nidx {
                        let (a, b) = (list[i], list[j]);
                        let diff = (a.value ^ b.value) & !mask & full;
                        if diff.count_ones() == 1 {
                            used[i] = true;
                            used[j] = true;
                            next.insert(Implicant { value: a.value & !diff, mask: mask | diff });
                        }
                    }
                }
            }
        }
        for (i, imp) in list.iter().enumerate() {
            if !used[i] {
                primes.insert(*imp);
            }
        }
        current = next;
    }
    (primes.into_iter().collect(), columns)
}

/// Bitset inclusion a ⊆ b.
fn is_subset(a: u64, b: u64) -> bool {
    a & !b == 0
}

/// Minimum cover of `on` by `primes`: essentials, then Petrick, greedy fallback.
/// Returns (chosen primes, essential flags, method).
pub fn min_cover(n: usize, on: &[u32], primes: &[Implicant]) -> (Vec<Implicant>, Vec<bool>, String) {
    if on.is_empty() {
        return (vec![], vec![false; primes.len()], "constant 0".into());
    }
    let mut essential = vec![false; primes.len()];
    let mut chosen: BTreeSet<usize> = BTreeSet::new();
    for &m in on {
        let cov: Vec<usize> = (0..primes.len()).filter(|&i| primes[i].covers(m)).collect();
        if cov.len() == 1 {
            essential[cov[0]] = true;
            chosen.insert(cov[0]);
        }
    }
    let remaining: Vec<u32> = on.iter().cloned().filter(|&m| !chosen.iter().any(|&i| primes[i].covers(m))).collect();
    let mut method = "essential prime implicants".to_string();
    if !remaining.is_empty() {
        // Petrick: product over remaining minterms of (sum of primes covering it)
        let cands: Vec<usize> = (0..primes.len()).filter(|i| !chosen.contains(i)).collect();
        let mut products: Vec<u64> = vec![0]; // bitsets over cands
        let mut exploded = cands.len() > 64;
        if !exploded {
            for &m in &remaining {
                let clause: Vec<usize> = cands.iter().enumerate().filter(|(_, &p)| primes[p].covers(m)).map(|(k, _)| k).collect();
                let mut next: Vec<u64> = Vec::new();
                for &p in &products {
                    for &k in &clause {
                        next.push(p | (1u64 << k));
                    }
                }
                // absorption: drop any product that is a superset of another
                next.sort_by_key(|x| x.count_ones());
                next.dedup();
                let mut kept: Vec<u64> = Vec::new();
                for x in next {
                    if !kept.iter().any(|&k| is_subset(k, x)) {
                        kept.push(x);
                    }
                }
                products = kept;
                if products.len() > 4000 {
                    exploded = true;
                    break;
                }
            }
        }
        if !exploded {
            let cost = |s: u64| {
                let lits: usize = (0..cands.len()).filter(|k| s & (1u64 << k) != 0).map(|k| primes[cands[k]].literals(n)).sum();
                (s.count_ones() as usize, lits)
            };
            let best = products.iter().cloned().min_by_key(|&s| cost(s)).unwrap_or(0);
            for (k, &c) in cands.iter().enumerate() {
                if best & (1u64 << k) != 0 {
                    chosen.insert(c);
                }
            }
            method = "essential primes + Petrick's method (exact)".into();
        } else {
            // greedy: repeatedly take the prime covering most uncovered minterms (fewest literals on ties)
            let mut left: BTreeSet<u32> = remaining.into_iter().collect();
            while !left.is_empty() {
                let best = cands
                    .iter()
                    .cloned()
                    .max_by_key(|&p| {
                        let c = left.iter().filter(|&&m| primes[p].covers(m)).count();
                        (c, usize::MAX - primes[p].literals(n))
                    })
                    .unwrap();
                chosen.insert(best);
                left.retain(|&m| !primes[best].covers(m));
            }
            method = "essential primes + greedy cover (Petrick expansion too large)".into();
        }
    }
    let mut cover: Vec<Implicant> = chosen.into_iter().map(|i| primes[i]).collect();
    cover.sort_by_key(|imp| literal_order(imp, n));
    (cover, essential, method)
}

/// Display order: terms whose first literal is an earlier, uncomplemented variable come first.
fn literal_order(imp: &Implicant, n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| {
            let bit = 1 << (n - 1 - i);
            if imp.mask & bit != 0 {
                2
            } else if imp.value & bit != 0 {
                0
            } else {
                1
            }
        })
        .collect()
}

// ------------------------------------------------------------------ K-map

#[derive(Clone, Debug, Serialize)]
pub struct KCell {
    pub row: usize,
    pub col: usize,
    pub minterm: u32,
    /// "0", "1" or "x".
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct KRect {
    pub r0: usize,
    pub r1: usize,
    pub c0: usize,
    pub c1: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct KGroup {
    pub term: String,
    pub pattern: String,
    pub rects: Vec<KRect>,
    pub essential: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct KMap {
    pub rows: usize,
    pub cols: usize,
    pub row_vars: String,
    pub col_vars: String,
    pub row_labels: Vec<String>,
    pub col_labels: Vec<String>,
    pub cells: Vec<KCell>,
    pub groups: Vec<KGroup>,
}

fn gray(bits: usize) -> Vec<u32> {
    (0..(1u32 << bits)).map(|i| i ^ (i >> 1)).collect()
}

/// Split a cyclic set of indices (0..len) into contiguous runs.
fn runs(set: &[usize], len: usize) -> Vec<(usize, usize)> {
    if set.len() == len {
        return vec![(0, len - 1)];
    }
    let inset = |i: usize| set.contains(&i);
    let mut out = Vec::new();
    let mut i = 0;
    while i < len {
        if inset(i) {
            let s = i;
            while i + 1 < len && inset(i + 1) {
                i += 1;
            }
            out.push((s, i));
        }
        i += 1;
    }
    out
}

pub fn kmap(n: usize, vars: &[String], table: &[Option<bool>], cover: &[Implicant], essential: &[Implicant]) -> Option<KMap> {
    if n == 0 || n > 4 {
        return None;
    }
    let rbits = n / 2;
    let cbits = n - rbits;
    let rg = gray(rbits);
    let cg = gray(cbits);
    let fmt = |v: u32, b: usize| if b == 0 { String::new() } else { format!("{:0width$b}", v, width = b) };
    let mut cells = Vec::new();
    for (r, &rv) in rg.iter().enumerate() {
        for (c, &cv) in cg.iter().enumerate() {
            let m = (rv << cbits) | cv;
            let value = match table[m as usize] {
                Some(true) => "1",
                Some(false) => "0",
                None => "x",
            };
            cells.push(KCell { row: r, col: c, minterm: m, value: value.into() });
        }
    }
    let groups = cover
        .iter()
        .map(|imp| {
            let rset: Vec<usize> = rg.iter().enumerate().filter(|(_, &rv)| rows_ok(imp, rv, cbits)).map(|(i, _)| i).collect();
            let cset: Vec<usize> = cg.iter().enumerate().filter(|(_, &cv)| col_ok(imp, cv, cbits)).map(|(i, _)| i).collect();
            let mut rects = Vec::new();
            for (r0, r1) in runs(&rset, rg.len()) {
                for (c0, c1) in runs(&cset, cg.len()) {
                    rects.push(KRect { r0, r1, c0, c1 });
                }
            }
            KGroup { term: imp.product(vars, Style::Math), pattern: imp.pattern(n), rects, essential: essential.contains(imp) }
        })
        .collect();
    Some(KMap {
        rows: rg.len(),
        cols: cg.len(),
        row_vars: vars[..rbits].join(""),
        col_vars: vars[rbits..].join(""),
        row_labels: rg.iter().map(|&v| fmt(v, rbits)).collect(),
        col_labels: cg.iter().map(|&v| fmt(v, cbits)).collect(),
        cells,
        groups,
    })
}

/// Does the implicant's row part (upper bits) match row value `rv`?
fn rows_ok(imp: &Implicant, rv: u32, cbits: usize) -> bool {
    let m = imp.mask >> cbits;
    let v = imp.value >> cbits;
    (rv & !m) == (v & !m)
}

/// Does the implicant's column part (lower bits) match column value `cv`?
fn col_ok(imp: &Implicant, cv: u32, cbits: usize) -> bool {
    let low = (1u32 << cbits) - 1;
    let m = imp.mask & low;
    let v = imp.value & low;
    (cv & !m) == (v & !m)
}

// ------------------------------------------------------------------ netlist

/// A gate in the SOP realisation (for drawing and simulation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gate {
    pub id: String,
    pub kind: String,
    pub inputs: Vec<String>,
    pub output: String,
    /// Drawing column (0 = inverters, 1 = AND plane, 2 = OR).
    pub col: usize,
    pub row: usize,
}

pub fn sop_netlist(vars: &[String], cover: &[Implicant], out: &str) -> Vec<Gate> {
    let n = vars.len();
    let mut gates = Vec::new();
    let mut needed_not = BTreeSet::new();
    for imp in cover {
        for i in 0..n {
            let bit = 1 << (n - 1 - i);
            if imp.mask & bit == 0 && imp.value & bit == 0 {
                needed_not.insert(i);
            }
        }
    }
    for (row, &i) in needed_not.iter().enumerate() {
        gates.push(Gate { id: format!("inv_{}", vars[i]), kind: "NOT".into(), inputs: vec![vars[i].clone()], output: format!("{}_n", vars[i]), col: 0, row });
    }
    let mut terms = Vec::new();
    for (k, imp) in cover.iter().enumerate() {
        let mut ins = Vec::new();
        for (i, v) in vars.iter().enumerate() {
            let bit = 1 << (n - 1 - i);
            if imp.mask & bit == 0 {
                ins.push(if imp.value & bit != 0 { v.clone() } else { format!("{v}_n") });
            }
        }
        if ins.len() == 1 {
            terms.push(ins[0].clone());
        } else if ins.is_empty() {
            terms.push("1'b1".into());
        } else {
            let o = format!("p{k}");
            gates.push(Gate { id: format!("and{k}"), kind: "AND".into(), inputs: ins, output: o.clone(), col: 1, row: k });
            terms.push(o);
        }
    }
    if terms.len() > 1 {
        gates.push(Gate { id: "or0".into(), kind: "OR".into(), inputs: terms, output: out.into(), col: 2, row: 0 });
    } else if let Some(t) = terms.first() {
        gates.push(Gate { id: "buf0".into(), kind: "BUF".into(), inputs: vec![t.clone()], output: out.into(), col: 2, row: 0 });
    } else {
        gates.push(Gate { id: "tie0".into(), kind: "TIE0".into(), inputs: vec![], output: out.into(), col: 2, row: 0 });
    }
    gates
}

// ------------------------------------------------------------------ top level

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MinimizeInput {
    /// Boolean expression (ignored if `minterms` is non-empty).
    pub expr: String,
    pub minterms: Vec<u32>,
    pub dont_cares: Vec<u32>,
    /// Variable names when minterms are given (default A, B, C…).
    pub vars: Vec<String>,
    pub num_vars: usize,
}

impl Default for MinimizeInput {
    fn default() -> Self {
        MinimizeInput { expr: "A'B'C' + A'B'C + A'BC' + AB'C + ABC' + ABC".into(), minterms: vec![], dont_cares: vec![], vars: vec![], num_vars: 0 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PrimeInfo {
    pub pattern: String,
    pub term: String,
    pub minterms: Vec<u32>,
    pub essential: bool,
    pub chosen: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct MinimizeResult {
    pub vars: Vec<String>,
    /// Truth table: "0" | "1" | "x" per minterm index (A = MSB).
    pub table: Vec<String>,
    pub minterms: Vec<u32>,
    pub dont_cares: Vec<u32>,
    pub qm_columns: Vec<Vec<String>>,
    pub primes: Vec<PrimeInfo>,
    pub method: String,
    pub sop: String,
    pub pos: String,
    pub sop_verilog: String,
    pub pos_verilog: String,
    pub sop_literals: usize,
    pub pos_literals: usize,
    pub kmap: Option<KMap>,
    pub netlist: Vec<Gate>,
    pub verilog: String,
    /// Canonical forms for reference.
    pub sigma: String,
    pub pi: String,
}

pub fn minimize(inp: &MinimizeInput) -> Result<MinimizeResult, String> {
    let (vars, table): (Vec<String>, Vec<Option<bool>>) = if !inp.minterms.is_empty() || (inp.expr.trim().is_empty() && inp.num_vars > 0) {
        let maxm = inp.minterms.iter().chain(inp.dont_cares.iter()).cloned().max().unwrap_or(0);
        let mut n = inp.num_vars.max(inp.vars.len());
        while (1u32 << n) <= maxm {
            n += 1;
        }
        let n = n.clamp(1, 8);
        let vars: Vec<String> = if inp.vars.len() == n { inp.vars.clone() } else { (0..n).map(|i| ((b'A' + i as u8) as char).to_string()).collect() };
        let mut t = vec![Some(false); 1 << n];
        for &m in &inp.dont_cares {
            if (m as usize) < t.len() {
                t[m as usize] = None;
            }
        }
        for &m in &inp.minterms {
            if (m as usize) < t.len() {
                t[m as usize] = Some(true);
            }
        }
        (vars, t)
    } else {
        let (e, vars) = parse(&inp.expr)?;
        let n = vars.len();
        let mut t = Vec::with_capacity(1 << n);
        for m in 0..(1u32 << n) {
            let v: Vec<bool> = (0..n).map(|i| m & (1 << (n - 1 - i)) != 0).collect();
            t.push(Some(e.eval(&v)));
        }
        for &m in &inp.dont_cares {
            if (m as usize) < t.len() {
                t[m as usize] = None;
            }
        }
        (vars, t)
    };
    let n = vars.len();
    let on: Vec<u32> = (0..table.len() as u32).filter(|&m| table[m as usize] == Some(true)).collect();
    let dc: Vec<u32> = (0..table.len() as u32).filter(|&m| table[m as usize].is_none()).collect();
    let off: Vec<u32> = (0..table.len() as u32).filter(|&m| table[m as usize] == Some(false)).collect();
    let (primes, cols) = prime_implicants(n, &on, &dc);
    let (cover, ess, method) = min_cover(n, &on, &primes);
    // POS via complement
    let (primes_c, _) = prime_implicants(n, &off, &dc);
    let (cover_c, _, _) = min_cover(n, &off, &primes_c);
    let sop = if on.is_empty() {
        "0".to_string()
    } else {
        cover.iter().map(|i| i.product(&vars, Style::Math)).collect::<Vec<_>>().join(" + ")
    };
    let sop_v = if on.is_empty() {
        "1'b0".to_string()
    } else {
        cover.iter().map(|i| format!("({})", i.product(&vars, Style::Verilog))).collect::<Vec<_>>().join(" | ")
    };
    let pos = if off.is_empty() {
        "1".to_string()
    } else {
        cover_c.iter().map(|i| i.sum_of_complement(&vars, Style::Math)).collect::<Vec<_>>().join("")
    };
    let pos_v = if off.is_empty() {
        "1'b1".to_string()
    } else {
        cover_c.iter().map(|i| i.sum_of_complement(&vars, Style::Verilog)).collect::<Vec<_>>().join(" & ")
    };
    let essential: Vec<Implicant> = primes.iter().zip(&ess).filter(|(_, e)| **e).map(|(p, _)| *p).collect();
    let prime_info = primes
        .iter()
        .zip(&ess)
        .map(|(p, e)| PrimeInfo { pattern: p.pattern(n), term: p.product(&vars, Style::Math), minterms: p.minterms(n).into_iter().filter(|m| on.contains(m)).collect(), essential: *e, chosen: cover.contains(p) })
        .collect();
    let netlist = sop_netlist(&vars, &cover, "F");
    let verilog = format!(
        "// Minimised by Protoforge Chip Lab (Quine–McCluskey + Petrick)\nmodule logic_fn (\n{}    output wire F\n);\n    assign F = {};\nendmodule\n",
        vars.iter().map(|v| format!("    input  wire {v},\n")).collect::<String>(),
        sop_v
    );
    Ok(MinimizeResult {
        table: table.iter().map(|v| match v { Some(true) => "1", Some(false) => "0", None => "x" }.to_string()).collect(),
        sigma: format!("Σm({}){}", on.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(","), if dc.is_empty() { String::new() } else { format!(" + d({})", dc.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(",")) }),
        pi: format!("ΠM({})", off.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(",")),
        minterms: on,
        dont_cares: dc,
        qm_columns: cols.iter().map(|c| c.iter().map(|i| i.pattern(n)).collect()).collect(),
        primes: prime_info,
        method,
        sop_literals: cover.iter().map(|i| i.literals(n)).sum(),
        pos_literals: cover_c.iter().map(|i| i.literals(n)).sum(),
        sop,
        pos,
        sop_verilog: sop_v,
        pos_verilog: pos_v,
        kmap: kmap(n, &vars, &table, &cover, &essential),
        netlist,
        verilog,
        vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_cover(cover_sop: &str, vars: &[String], m: u32) -> bool {
        let (e, v2) = parse(cover_sop).unwrap();
        let n = vars.len();
        let vals: Vec<bool> = v2.iter().map(|name| {
            let i = vars.iter().position(|x| x == name).unwrap();
            m & (1 << (n - 1 - i)) != 0
        }).collect();
        e.eval(&vals)
    }

    #[test]
    fn classic_cyclic_example_needs_petrick() {
        // Σm(0,1,2,5,6,7): six primes, no essentials, two minimum covers of 3 terms
        let r = minimize(&MinimizeInput { expr: String::new(), minterms: vec![0, 1, 2, 5, 6, 7], ..Default::default() }).unwrap();
        assert_eq!(r.primes.len(), 6);
        assert!(r.primes.iter().all(|p| !p.essential));
        assert_eq!(r.sop.split(" + ").count(), 3);
        assert_eq!(r.sop_literals, 6);
        assert!(r.method.contains("Petrick"));
        for m in 0..8 {
            assert_eq!(eval_cover(&r.sop_verilog, &r.vars, m), [0, 1, 2, 5, 6, 7].contains(&m), "m{m}");
        }
    }

    #[test]
    fn majority_function() {
        let r = minimize(&MinimizeInput { expr: "a&b | a&c | b&c | a&b&c".into(), ..Default::default() }).unwrap();
        assert_eq!(r.minterms, vec![3, 5, 6, 7]);
        assert_eq!(r.sop, "ab + ac + bc");
        assert!(r.primes.iter().all(|p| p.essential));
        // POS of majority is (a+b)(a+c)(b+c)
        assert_eq!(r.pos.matches('(').count(), 3);
        let k = r.kmap.unwrap();
        assert_eq!((k.rows, k.cols), (2, 4));
        assert_eq!(k.col_labels, vec!["00", "01", "11", "10"]);
    }

    #[test]
    fn parser_handles_all_syntaxes() {
        let (e, v) = parse("A'B + !C & (D ^ ~A) + 1*0").unwrap();
        assert_eq!(v, vec!["A", "B", "C", "D"]);
        // A=0,B=1,C=0,D=0 → A'B = 1
        assert!(e.eval(&[false, true, false, false]));
        assert!(parse("A +").is_err());
        assert!(parse("(A").is_err());
        let (e, _) = parse("x y'").unwrap();
        assert!(e.eval(&[true, false]));
        assert!(!e.eval(&[true, true]));
    }

    #[test]
    fn dont_cares_and_kmap_wraparound() {
        // Σm(0,2,8,10) = B'D' — the four corners of a 4-variable map
        let r = minimize(&MinimizeInput { expr: String::new(), minterms: vec![0, 2, 8, 10], num_vars: 4, ..Default::default() }).unwrap();
        assert_eq!(r.sop, "B'D'");
        let k = r.kmap.unwrap();
        assert_eq!(k.groups[0].rects.len(), 4, "corners split into four rectangles");
        // with don't cares: Σm(1,3,7,11,15) + d(0,2,5) = CD + A'B' (classic textbook)
        let r = minimize(&MinimizeInput { expr: String::new(), minterms: vec![1, 3, 7, 11, 15], dont_cares: vec![0, 2, 5], num_vars: 4, ..Default::default() }).unwrap();
        assert_eq!(r.sop_literals, 4, "{}", r.sop);
    }

    #[test]
    fn eight_variables_parity_is_handled() {
        let r = minimize(&MinimizeInput { expr: "a^b^c^d^e^f^g^h".into(), ..Default::default() }).unwrap();
        assert_eq!(r.minterms.len(), 128);
        assert_eq!(r.primes.len(), 128); // parity has no adjacencies
        assert!(r.kmap.is_none());
    }
}
