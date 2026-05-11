//! E-node and `RecExpr` types plus an s-expression parser.

use std::fmt;
use std::mem;
use std::str::FromStr;

use crate::Id;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SymbolLang {
    pub op: String,
    pub children: Vec<Id>,
}

impl SymbolLang {
    pub fn new(op: impl Into<String>, children: Vec<Id>) -> Self {
        Self {
            op: op.into(),
            children,
        }
    }

    pub fn leaf(op: impl Into<String>) -> Self {
        Self::new(op, Vec::new())
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.op == other.op && self.children.len() == other.children.len()
    }
}

impl fmt::Display for SymbolLang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.children.is_empty() {
            write!(f, "{}", self.op)
        } else {
            write!(f, "({}", self.op)?;
            for c in &self.children {
                write!(f, " {c}")?;
            }
            write!(f, ")")
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecExpr {
    nodes: Vec<SymbolLang>,
}

impl RecExpr {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, node: SymbolLang) -> Id {
        let id = Id::from(self.nodes.len());
        self.nodes.push(node);
        id
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn nodes(&self) -> &[SymbolLang] {
        &self.nodes
    }

    #[must_use]
    pub fn get(&self, id: Id) -> &SymbolLang {
        &self.nodes[id.index()]
    }

    /// # Panics
    /// Panics if the expression has no nodes.
    #[must_use]
    pub fn root(&self) -> Id {
        assert!(!self.nodes.is_empty(), "RecExpr is empty");
        Id::from(self.nodes.len() - 1)
    }
}

impl fmt::Display for RecExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn go(rec: &RecExpr, id: Id, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let node = rec.get(id);
            if node.children.is_empty() {
                write!(f, "{}", node.op)
            } else {
                write!(f, "({}", node.op)?;
                for &c in &node.children {
                    write!(f, " ")?;
                    go(rec, c, f)?;
                }
                write!(f, ")")
            }
        }
        if self.nodes.is_empty() {
            return Ok(());
        }
        go(self, self.root(), f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedClose,
    EmptyList,
    Trailing(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::UnexpectedClose => write!(f, "unexpected ')'"),
            Self::EmptyList => write!(f, "empty list '()' has no operator"),
            Self::Trailing(s) => write!(f, "trailing input: {s:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for RecExpr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(s);
        let mut rec = Self::new();
        let mut idx = 0;
        parse_into(&tokens, &mut idx, &mut rec)?;
        if idx != tokens.len() {
            return Err(ParseError::Trailing(tokens[idx..].join(" ")));
        }
        Ok(rec)
    }
}

pub(crate) fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' => {
                if !cur.is_empty() {
                    out.push(mem::take(&mut cur));
                }
                out.push(c.to_string());
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_into(tokens: &[String], idx: &mut usize, rec: &mut RecExpr) -> Result<Id, ParseError> {
    if *idx >= tokens.len() {
        return Err(ParseError::UnexpectedEof);
    }
    let tok = &tokens[*idx];
    *idx += 1;
    if tok == ")" {
        return Err(ParseError::UnexpectedClose);
    }
    if tok != "(" {
        let id = rec.add(SymbolLang::leaf(tok));
        return Ok(id);
    }
    if *idx >= tokens.len() {
        return Err(ParseError::UnexpectedEof);
    }
    let op = tokens[*idx].clone();
    if op == "(" || op == ")" {
        return Err(ParseError::EmptyList);
    }
    *idx += 1;
    let mut children = Vec::new();
    loop {
        if *idx >= tokens.len() {
            return Err(ParseError::UnexpectedEof);
        }
        if tokens[*idx] == ")" {
            *idx += 1;
            break;
        }
        let child = parse_into(tokens, idx, rec)?;
        children.push(child);
    }
    Ok(rec.add(SymbolLang::new(op, children)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_atom() {
        let r: RecExpr = "x".parse().unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r.get(r.root()).op, "x");
    }

    #[test]
    fn parse_simple() {
        let r: RecExpr = "(+ 1 2)".parse().unwrap();
        assert_eq!(r.len(), 3);
        let root = r.get(r.root());
        assert_eq!(root.op, "+");
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn parse_nested() {
        let r: RecExpr = "(+ (* 2 x) (* 2 y))".parse().unwrap();
        assert_eq!(r.len(), 7);
        assert_eq!(r.to_string(), "(+ (* 2 x) (* 2 y))");
    }

    #[test]
    fn parse_errors() {
        assert!("(+ 1".parse::<RecExpr>().is_err());
        assert!(")".parse::<RecExpr>().is_err());
        assert!("()".parse::<RecExpr>().is_err());
        assert!("a b".parse::<RecExpr>().is_err());
    }

    #[test]
    fn roundtrip() {
        for s in ["x", "(+ 1 2)", "(if (< x 0) 0 x)", "(f a b c d)"] {
            let r: RecExpr = s.parse().unwrap();
            assert_eq!(r.to_string(), s);
        }
    }
}
