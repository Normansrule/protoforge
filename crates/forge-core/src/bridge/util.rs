//! Small shared helpers for the exporters: number formatting, quoting,
//! deterministic identifiers and a minimal S-expression reader used by the
//! tests to re-parse what the writers produce.

/// FNV-1a 64-bit hash (deterministic across platforms and WebAssembly).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Deterministic RFC 4122-shaped (version 4 / variant 1 bits set) identifier
/// derived from a seed string, so the same design always exports the same
/// file (diff-friendly) while every object still gets a unique id.
pub fn uuid(seed: &str) -> String {
    let a = fnv1a(seed.as_bytes());
    let b = fnv1a(format!("{seed}#protoforge").as_bytes()).rotate_left(17) ^ a.rotate_right(29);
    let hi = (a & 0xffff_ffff_ffff_0fff) | 0x0000_0000_0000_4000;
    let lo = (b & 0x3fff_ffff_ffff_ffff) | 0x8000_0000_0000_0000;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        hi >> 32,
        (hi >> 16) & 0xffff,
        hi & 0xffff,
        lo >> 48,
        lo & 0xffff_ffff_ffff
    )
}

/// Format a millimetre value with at most 6 decimals and no trailing zeros
/// (`1.5`, `-0.25`, `12`), never `-0`.
pub fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let r = (v * 1e6).round() / 1e6;
    let mut s = format!("{r:.6}");
    while s.contains('.') && (s.ends_with('0') || s.ends_with('.')) {
        s.pop();
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Quote a string for an S-expression (KiCad) file.
pub fn q(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            _ => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Parsed S-expression (used by tests and the self-check).
#[derive(Clone, Debug, PartialEq)]
pub enum Sx {
    Atom(String),
    Str(String),
    List(Vec<Sx>),
}

impl Sx {
    /// Head symbol of a list, e.g. `footprint` for `(footprint ...)`.
    pub fn head(&self) -> Option<&str> {
        match self {
            Sx::List(v) => match v.first() {
                Some(Sx::Atom(a)) => Some(a.as_str()),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn items(&self) -> &[Sx] {
        match self {
            Sx::List(v) => v,
            _ => &[],
        }
    }
    /// Child lists whose head is `name`.
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Sx> + 'a {
        self.items().iter().filter(move |x| x.head().is_some_and(|h| h == name))
    }
    pub fn child(&self, name: &str) -> Option<&Sx> {
        self.items().iter().find(|x| x.head() == Some(name))
    }
    /// The i-th element as text (atom or string).
    pub fn text(&self, i: usize) -> Option<&str> {
        match self.items().get(i) {
            Some(Sx::Atom(a)) | Some(Sx::Str(a)) => Some(a.as_str()),
            _ => None,
        }
    }
    pub fn f(&self, i: usize) -> Option<f64> {
        self.text(i).and_then(|t| t.parse().ok())
    }
    /// Recursive count of lists with head `name`.
    pub fn count(&self, name: &str) -> usize {
        let own = usize::from(self.head() == Some(name));
        own + self.items().iter().map(|x| x.count(name)).sum::<usize>()
    }
}

/// Parse one S-expression document (strings with `\"` escapes, atoms, lists).
pub fn parse_sexpr(src: &str) -> Result<Sx, String> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut stack: Vec<Vec<Sx>> = vec![Vec::new()];
    while i < b.len() {
        let c = b[i];
        match c {
            b'(' => {
                stack.push(Vec::new());
                i += 1;
            }
            b')' => {
                let top = stack.pop().ok_or("unbalanced ')'")?;
                let parent = stack.last_mut().ok_or_else(|| format!("unbalanced ')' at byte {i}"))?;
                parent.push(Sx::List(top));
                i += 1;
            }
            b'"' => {
                let mut s = String::new();
                i += 1;
                loop {
                    if i >= b.len() {
                        return Err("unterminated string".into());
                    }
                    match b[i] {
                        b'\\' if i + 1 < b.len() => {
                            s.push(match b[i + 1] {
                                b'n' => '\n',
                                x => x as char,
                            });
                            i += 2;
                        }
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => {
                            // copy one UTF-8 character
                            let ch = src[i..].chars().next().unwrap();
                            s.push(ch);
                            i += ch.len_utf8();
                        }
                    }
                }
                stack.last_mut().unwrap().push(Sx::Str(s));
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                let st = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'(' && b[i] != b')' {
                    i += 1;
                }
                stack.last_mut().unwrap().push(Sx::Atom(src[st..i].to_string()));
            }
        }
    }
    if stack.len() != 1 {
        return Err(format!("{} unclosed '('", stack.len() - 1));
    }
    let mut top = stack.pop().unwrap();
    if top.len() != 1 {
        return Err(format!("expected one top-level expression, found {}", top.len()));
    }
    Ok(top.remove(0))
}

/// Base64 (standard alphabet, padded) for text files inside a bundle.
pub fn base64(bytes: &[u8]) -> String {
    crate::cad::export::base64(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_compact() {
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(2.0), "2");
        assert_eq!(num(-0.0000001), "0");
        assert_eq!(num(0.1 + 0.2), "0.3");
        assert_eq!(num(-12.3456789), "-12.345679");
    }

    #[test]
    fn uuids_are_stable_and_unique() {
        let a = uuid("fp:U1");
        assert_eq!(a, uuid("fp:U1"));
        assert_ne!(a, uuid("fp:U2"));
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "4");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn sexpr_round_trip() {
        let s = format!("(a (b 1 2) (c {} x))", q("he said \"hi\" \\ ok"));
        let t = parse_sexpr(&s).unwrap();
        assert_eq!(t.head(), Some("a"));
        assert_eq!(t.child("b").unwrap().f(2), Some(2.0));
        assert_eq!(t.child("c").unwrap().text(1), Some("he said \"hi\" \\ ok"));
        assert!(parse_sexpr("(a (b)").is_err());
        assert!(parse_sexpr("(a))").is_err());
    }
}
