//! Lattice-shaped analyses over an e-graph.

use std::collections::HashMap;

use crate::language::SymbolLang;
use crate::{EGraph, Id};

pub trait Analysis {
    type Data: Clone + PartialEq;

    fn make(&self, node: &SymbolLang, child_data: &[&Self::Data]) -> Self::Data;
    fn merge(&self, a: Self::Data, b: Self::Data) -> Self::Data;

    fn run(&self, egraph: &EGraph) -> HashMap<Id, Self::Data> {
        let mut data: HashMap<Id, Self::Data> = HashMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for class in egraph.classes() {
                for node in &class.nodes {
                    let mut child_data: Vec<&Self::Data> = Vec::with_capacity(node.children.len());
                    let mut all_known = true;
                    for &c in &node.children {
                        let c = egraph.find(c);
                        if let Some(d) = data.get(&c) {
                            child_data.push(d);
                        } else {
                            all_known = false;
                            break;
                        }
                    }
                    if !all_known {
                        continue;
                    }
                    let new = self.make(node, &child_data);
                    let id = class.id;
                    if let Some(existing) = data.get(&id) {
                        let merged = self.merge(existing.clone(), new);
                        if merged != *existing {
                            data.insert(id, merged);
                            changed = true;
                        }
                    } else {
                        data.insert(id, new);
                        changed = true;
                    }
                }
            }
        }
        data
    }
}

#[derive(Debug)]
pub struct ConstFold;

impl Analysis for ConstFold {
    type Data = Option<i64>;

    fn make(&self, node: &SymbolLang, child_data: &[&Self::Data]) -> Self::Data {
        if node.children.is_empty() {
            return node.op.parse().ok();
        }
        let args: Option<Vec<i64>> = child_data.iter().map(|d| **d).collect();
        let args = args?;
        match (node.op.as_str(), args.as_slice()) {
            ("+", &[a, b]) => Some(a + b),
            ("-", &[a, b]) => Some(a - b),
            ("*", &[a, b]) => Some(a * b),
            ("neg", &[a]) => Some(-a),
            _ => None,
        }
    }

    fn merge(&self, a: Self::Data, b: Self::Data) -> Self::Data {
        match (a, b) {
            (Some(x), None | Some(_)) | (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecExpr;

    #[test]
    fn folds_two_plus_three() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ 2 3)".parse().unwrap();
        let root = g.add_expr(&e);
        g.rebuild();
        let data = ConstFold.run(&g);
        assert_eq!(data.get(&g.find(root)), Some(&Some(5)));
    }

    #[test]
    fn nested_folding() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ (* 2 3) (- 10 4))".parse().unwrap();
        let root = g.add_expr(&e);
        g.rebuild();
        let data = ConstFold.run(&g);
        assert_eq!(data.get(&g.find(root)), Some(&Some(12)));
    }

    #[test]
    fn unknown_stays_unknown() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x 3)".parse().unwrap();
        let root = g.add_expr(&e);
        g.rebuild();
        let data = ConstFold.run(&g);
        assert_eq!(data.get(&g.find(root)), Some(&None));
    }
}
