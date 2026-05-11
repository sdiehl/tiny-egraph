//! Extract a single concrete term from an e-class.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::language::{RecExpr, SymbolLang};
use crate::{EGraph, Id};

pub trait CostFunction {
    type Cost: Ord + Clone;
    fn cost(&mut self, op: &str, children: &[Self::Cost]) -> Self::Cost;
}

#[derive(Debug)]
pub struct AstSize;

impl CostFunction for AstSize {
    type Cost = usize;
    fn cost(&mut self, _op: &str, children: &[usize]) -> usize {
        1 + children.iter().sum::<usize>()
    }
}

#[derive(Debug)]
pub struct AstDepth;

impl CostFunction for AstDepth {
    type Cost = usize;
    fn cost(&mut self, _op: &str, children: &[usize]) -> usize {
        1 + children.iter().copied().max().unwrap_or(0)
    }
}

const UNREACHED: usize = usize::MAX / 2;

pub struct Extractor<'a, CF: CostFunction> {
    egraph: &'a EGraph,
    costs: HashMap<Id, (CF::Cost, SymbolLang)>,
    cost_fn: CF,
}

impl<CF: CostFunction> fmt::Debug for Extractor<'_, CF> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extractor")
            .field("classes", &self.costs.len())
            .finish_non_exhaustive()
    }
}

impl<'a, CF: CostFunction<Cost = usize>> Extractor<'a, CF> {
    pub fn new(egraph: &'a EGraph, cost_fn: CF) -> Self {
        let mut ex = Self {
            egraph,
            costs: HashMap::new(),
            cost_fn,
        };
        ex.compute_costs();
        ex
    }

    /// # Panics
    /// Panics if `id` refers to an e-class with no extracted node (e.g. the graph was not rebuilt).
    pub fn find_best(&self, id: Id) -> (usize, RecExpr) {
        let canon = self.egraph.find(id);
        let &(cost, _) = self
            .costs
            .get(&canon)
            .expect("e-class has no extracted node; was the graph rebuilt?");
        let mut rec = RecExpr::new();
        let mut memo = HashMap::new();
        self.build(canon, &mut rec, &mut memo);
        (cost, rec)
    }

    fn build(&self, class: Id, rec: &mut RecExpr, memo: &mut HashMap<Id, Id>) -> Id {
        if let Some(&id) = memo.get(&class) {
            return id;
        }
        let (_, node) = self.costs.get(&class).expect("missing class");
        let new_children: Vec<Id> = node
            .children
            .iter()
            .map(|&c| self.build(self.egraph.find(c), rec, memo))
            .collect();
        let id = rec.add(SymbolLang::new(node.op.clone(), new_children));
        memo.insert(class, id);
        id
    }

    fn compute_costs(&mut self) {
        for class in self.egraph.classes() {
            self.costs
                .insert(class.id, (UNREACHED, class.nodes[0].clone()));
        }
        let mut changed = true;
        while changed {
            changed = false;
            for class in self.egraph.classes() {
                let canon = class.id;
                let mut best: Option<(usize, &SymbolLang)> = None;
                for node in &class.nodes {
                    let mut all_known = true;
                    let mut child_costs = Vec::with_capacity(node.children.len());
                    for &c in &node.children {
                        let c_canon = self.egraph.find(c);
                        match self.costs.get(&c_canon) {
                            Some((cost, _)) if *cost < UNREACHED => {
                                child_costs.push(*cost);
                            }
                            _ => {
                                all_known = false;
                                break;
                            }
                        }
                    }
                    if !all_known {
                        continue;
                    }
                    let c = self.cost_fn.cost(&node.op, &child_costs);
                    match best {
                        Some((b, _)) if b <= c => {}
                        _ => best = Some((c, node)),
                    }
                }
                if let Some((c, node)) = best {
                    let entry = self
                        .costs
                        .entry(canon)
                        .or_insert_with(|| (UNREACHED, node.clone()));
                    if c < entry.0 {
                        *entry = (c, node.clone());
                        changed = true;
                    }
                }
            }
        }
    }
}

pub struct GreedyExtractor<'a, CF: CostFunction> {
    egraph: &'a EGraph,
    cost_fn: CF,
}

impl<CF: CostFunction> fmt::Debug for GreedyExtractor<'_, CF> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GreedyExtractor").finish_non_exhaustive()
    }
}

impl<'a, CF: CostFunction<Cost = usize>> GreedyExtractor<'a, CF> {
    pub const fn new(egraph: &'a EGraph, cost_fn: CF) -> Self {
        Self { egraph, cost_fn }
    }

    pub fn find_best(&mut self, id: Id) -> (usize, RecExpr) {
        let mut rec = RecExpr::new();
        let mut visiting = HashSet::new();
        let (cost, _) = self.go(self.egraph.find(id), &mut rec, &mut visiting);
        (cost, rec)
    }

    fn go(&mut self, class: Id, rec: &mut RecExpr, visiting: &mut HashSet<Id>) -> (usize, Id) {
        if !visiting.insert(class) {
            let id = rec.add(SymbolLang::leaf(format!("<cycle:{class}>")));
            return (UNREACHED, id);
        }
        let class_data = self.egraph.get_class(class).expect("missing class");
        let mut best: Option<(usize, &SymbolLang, Vec<usize>)> = None;
        for node in &class_data.nodes {
            let est_children: Vec<usize> = vec![1; node.children.len()];
            let est = self.cost_fn.cost(&node.op, &est_children);
            match best {
                Some((b, _, _)) if b <= est => {}
                _ => best = Some((est, node, est_children)),
            }
        }
        let (_est, node, _) = best.expect("empty class");
        let node = node.clone();
        let mut child_costs = Vec::with_capacity(node.children.len());
        let mut new_children = Vec::with_capacity(node.children.len());
        for &c in &node.children {
            let (cc, cid) = self.go(self.egraph.find(c), rec, visiting);
            child_costs.push(cc);
            new_children.push(cid);
        }
        let real_cost = self.cost_fn.cost(&node.op, &child_costs);
        let id = rec.add(SymbolLang::new(node.op.clone(), new_children));
        visiting.remove(&class);
        (real_cost, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rewrite, RecExpr, Runner};

    #[test]
    fn extract_unrewritten_tree() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x 1)".parse().unwrap();
        let root = g.add_expr(&e);
        g.rebuild();
        let ext = Extractor::new(&g, AstSize);
        let (cost, expr) = ext.find_best(root);
        assert_eq!(cost, 3);
        assert_eq!(expr.to_string(), "(+ x 1)");
    }

    #[test]
    fn extract_after_rewrite() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x 0)".parse().unwrap();
        let root = g.add_expr(&e);
        let rules = vec![rewrite!("zero"; "(+ ?a 0)" => "?a")];
        let runner = Runner::default().with_egraph(g).run(&rules);
        let ext = Extractor::new(&runner.egraph, AstSize);
        let (cost, expr) = ext.find_best(root);
        assert_eq!(cost, 1);
        assert_eq!(expr.to_string(), "x");
    }
}
