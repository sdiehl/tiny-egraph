//! Patterns and substitutions.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::language::{tokenize, ParseError, SymbolLang};
use crate::Id;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Var(pub String);

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?{}", self.0)
    }
}

impl Var {
    pub fn new(name: impl Into<String>) -> Self {
        Var(name.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PatternNode {
    Op { op: String, children: Vec<Id> },
    Var(Var),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pattern {
    pub nodes: Vec<PatternNode>,
}

impl Pattern {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: PatternNode) -> Id {
        let id = Id::from(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn get(&self, id: Id) -> &PatternNode {
        &self.nodes[id.index()]
    }

    pub fn root(&self) -> Id {
        assert!(!self.nodes.is_empty(), "Pattern is empty");
        Id::from(self.nodes.len() - 1)
    }

    pub fn vars(&self) -> Vec<Var> {
        let mut seen: HashMap<&Var, ()> = HashMap::new();
        let mut out = Vec::new();
        for node in &self.nodes {
            if let PatternNode::Var(v) = node {
                if seen.insert(v, ()).is_none() {
                    out.push(v.clone());
                }
            }
        }
        out
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.nodes.is_empty() {
            return Ok(());
        }
        fn go(p: &Pattern, id: Id, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match p.get(id) {
                PatternNode::Var(v) => write!(f, "{}", v),
                PatternNode::Op { op, children } => {
                    if children.is_empty() {
                        write!(f, "{}", op)
                    } else {
                        write!(f, "({}", op)?;
                        for &c in children {
                            write!(f, " ")?;
                            go(p, c, f)?;
                        }
                        write!(f, ")")
                    }
                }
            }
        }
        go(self, self.root(), f)
    }
}

impl FromStr for Pattern {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(s);
        let mut pat = Pattern::new();
        let mut idx = 0;
        parse_into(&tokens, &mut idx, &mut pat)?;
        if idx != tokens.len() {
            return Err(ParseError::Trailing(tokens[idx..].join(" ")));
        }
        Ok(pat)
    }
}

fn parse_into(tokens: &[String], idx: &mut usize, pat: &mut Pattern) -> Result<Id, ParseError> {
    if *idx >= tokens.len() {
        return Err(ParseError::UnexpectedEof);
    }
    let tok = &tokens[*idx];
    *idx += 1;
    if tok == ")" {
        return Err(ParseError::UnexpectedClose);
    }
    if tok != "(" {
        return Ok(if let Some(name) = tok.strip_prefix('?') {
            pat.push(PatternNode::Var(Var::new(name.to_string())))
        } else {
            pat.push(PatternNode::Op {
                op: tok.clone(),
                children: Vec::new(),
            })
        });
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
        let child = parse_into(tokens, idx, pat)?;
        children.push(child);
    }
    Ok(pat.push(PatternNode::Op { op, children }))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subst {
    bindings: Vec<(Var, Id)>,
}

impl Subst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, v: &Var) -> Option<Id> {
        self.bindings
            .iter()
            .find_map(|(k, id)| (k == v).then_some(*id))
    }

    pub fn insert(&mut self, v: Var, id: Id) {
        debug_assert!(self.get(&v).is_none(), "rebinding variable {}", v);
        self.bindings.push((v, id));
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Var, Id)> {
        self.bindings.iter()
    }
}

pub fn instantiate(pat: &Pattern, subst: &Subst, egraph: &mut crate::EGraph) -> Id {
    fn go(
        pat: &Pattern,
        id: Id,
        subst: &Subst,
        egraph: &mut crate::EGraph,
        cache: &mut Vec<Option<Id>>,
    ) -> Id {
        if let Some(cached) = cache[id.index()] {
            return cached;
        }
        let new_id = match pat.get(id) {
            PatternNode::Var(v) => subst
                .get(v)
                .unwrap_or_else(|| panic!("unbound variable {} during instantiate", v)),
            PatternNode::Op { op, children } => {
                let new_children = children
                    .iter()
                    .map(|&c| go(pat, c, subst, egraph, cache))
                    .collect();
                egraph.add(SymbolLang::new(op.clone(), new_children))
            }
        };
        cache[id.index()] = Some(new_id);
        new_id
    }
    let mut cache = vec![None; pat.len()];
    go(pat, pat.root(), subst, egraph, &mut cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_var() {
        let p: Pattern = "?x".parse().unwrap();
        assert!(matches!(p.get(p.root()), PatternNode::Var(v) if v.0 == "x"));
    }

    #[test]
    fn parse_compound() {
        let p: Pattern = "(+ ?a 0)".parse().unwrap();
        assert_eq!(p.to_string(), "(+ ?a 0)");
        assert_eq!(p.vars(), vec![Var::new("a")]);
    }

    #[test]
    fn vars_in_order() {
        let p: Pattern = "(* ?a (+ ?b ?a))".parse().unwrap();
        assert_eq!(p.vars(), vec![Var::new("a"), Var::new("b")]);
    }
}
