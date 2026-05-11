//! E-graph: hashcons + union-find + congruence closure via deferred rebuild.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::mem;

use crate::language::SymbolLang;
use crate::{Id, RecExpr, UnionFind};

#[derive(Clone, Debug)]
pub struct EClass {
    pub id: Id,
    pub nodes: Vec<SymbolLang>,
    pub parents: Vec<(SymbolLang, Id)>,
}

impl EClass {
    fn new(id: Id, node: SymbolLang) -> Self {
        Self {
            id,
            nodes: vec![node],
            parents: Vec::new(),
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[derive(Clone, Default)]
pub struct EGraph {
    union_find: UnionFind,
    memo: HashMap<SymbolLang, Id>,
    classes: HashMap<Id, EClass>,
    pending: Vec<Id>,
}

impl fmt::Debug for EGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EGraph")
            .field("classes", &self.classes.len())
            .field("memo_size", &self.memo.len())
            .field("pending", &self.pending.len())
            .finish_non_exhaustive()
    }
}

impl EGraph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn number_of_classes(&self) -> usize {
        self.classes.len()
    }

    pub fn total_size(&self) -> usize {
        self.classes.values().map(EClass::len).sum()
    }

    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.pending.is_empty()
    }

    #[must_use]
    pub fn find(&self, id: Id) -> Id {
        self.union_find.find(id)
    }

    pub fn find_mut(&mut self, id: Id) -> Id {
        self.union_find.find_mut(id)
    }

    pub fn classes(&self) -> impl Iterator<Item = &EClass> {
        self.classes.values()
    }

    #[must_use]
    pub fn get_class(&self, id: Id) -> Option<&EClass> {
        self.classes.get(&self.find(id))
    }

    #[must_use]
    pub fn lookup(&self, node: &SymbolLang) -> Option<Id> {
        self.memo.get(node).map(|&id| self.find(id))
    }

    pub fn canonicalize(&self, node: &mut SymbolLang) {
        for child in &mut node.children {
            *child = self.find(*child);
        }
    }

    pub fn add(&mut self, mut node: SymbolLang) -> Id {
        self.canonicalize(&mut node);
        if let Some(&id) = self.memo.get(&node) {
            return self.find(id);
        }
        let id = self.union_find.make_set();
        for &child in &node.children {
            let child = self.find(child);
            if let Some(class) = self.classes.get_mut(&child) {
                class.parents.push((node.clone(), id));
            }
        }
        let class = EClass::new(id, node.clone());
        self.classes.insert(id, class);
        self.memo.insert(node, id);
        id
    }

    /// # Panics
    /// Panics if `expr` has no nodes.
    pub fn add_expr(&mut self, expr: &RecExpr) -> Id {
        let mut ids: Vec<Id> = Vec::with_capacity(expr.len());
        for node in expr.nodes() {
            let translated = SymbolLang::new(
                node.op.clone(),
                node.children.iter().map(|&c| ids[c.index()]).collect(),
            );
            ids.push(self.add(translated));
        }
        *ids.last().expect("RecExpr must be non-empty")
    }

    /// # Panics
    /// Panics if internal class bookkeeping is missing for either id (an invariant violation).
    pub fn union(&mut self, a: Id, b: Id) -> bool {
        let a = self.find_mut(a);
        let b = self.find_mut(b);
        if a == b {
            return false;
        }
        let new_root = self.union_find.union(a, b);
        let (winner, loser) = if new_root == a { (a, b) } else { (b, a) };
        let loser_class = self
            .classes
            .remove(&loser)
            .expect("loser class missing during union");
        let winner_class = self
            .classes
            .get_mut(&winner)
            .expect("winner class missing during union");
        winner_class.nodes.extend(loser_class.nodes);
        winner_class.parents.extend(loser_class.parents);
        self.pending.push(winner);
        true
    }

    pub fn rebuild(&mut self) -> usize {
        let mut merged = 0;
        while !self.pending.is_empty() {
            let todo = mem::take(&mut self.pending);
            let mut seen: HashSet<Id> = HashSet::with_capacity(todo.len());
            for id in todo {
                let c = self.find_mut(id);
                seen.insert(c);
            }
            for &id in &seen {
                merged += self.repair(id);
            }
        }
        merged
    }

    fn repair(&mut self, id: Id) -> usize {
        let id = self.find_mut(id);
        let parents = match self.classes.get_mut(&id) {
            Some(c) => mem::take(&mut c.parents),
            None => return 0,
        };

        let mut new_parents: Vec<(SymbolLang, Id)> = Vec::with_capacity(parents.len());
        let mut merged = 0;
        for (mut node, parent_id) in parents {
            self.memo.remove(&node);
            self.canonicalize(&mut node);
            let parent_id = self.find_mut(parent_id);
            match self.memo.get(&node).copied() {
                Some(existing) if self.find(existing) != parent_id => {
                    self.union(parent_id, existing);
                    merged += 1;
                }
                _ => {}
            }
            self.memo.insert(node.clone(), parent_id);
            new_parents.push((node, parent_id));
        }

        if let Some(class) = self.classes.get_mut(&id) {
            for node in &mut class.nodes {
                for child in &mut node.children {
                    *child = self.union_find.find(*child);
                }
            }
            class
                .nodes
                .sort_by(|a, b| a.op.cmp(&b.op).then(a.children.cmp(&b.children)));
            class.nodes.dedup();
            class.parents = new_parents;
        }
        merged
    }

    #[must_use]
    pub fn equiv(&self, a: Id, b: Id) -> bool {
        self.find(a) == self.find(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(op: &str) -> SymbolLang {
        SymbolLang::leaf(op)
    }

    fn op(op: &str, children: Vec<Id>) -> SymbolLang {
        SymbolLang::new(op, children)
    }

    #[test]
    fn add_hashconses() {
        let mut g = EGraph::new();
        let a1 = g.add(leaf("a"));
        let a2 = g.add(leaf("a"));
        assert_eq!(a1, a2);
        assert_eq!(g.number_of_classes(), 1);
    }

    #[test]
    fn add_distinguishes() {
        let mut g = EGraph::new();
        let a = g.add(leaf("a"));
        let b = g.add(leaf("b"));
        assert_ne!(a, b);
        assert_eq!(g.number_of_classes(), 2);
    }

    #[test]
    fn add_compound() {
        let mut g = EGraph::new();
        let a = g.add(leaf("a"));
        let b = g.add(leaf("b"));
        let f1 = g.add(op("f", vec![a, b]));
        let f2 = g.add(op("f", vec![a, b]));
        assert_eq!(f1, f2);
    }

    #[test]
    fn union_propagates_via_rebuild() {
        let mut g = EGraph::new();
        let a = g.add(leaf("a"));
        let b = g.add(leaf("b"));
        let fa = g.add(op("f", vec![a]));
        let fb = g.add(op("f", vec![b]));
        assert_ne!(g.find(fa), g.find(fb));
        g.union(a, b);
        g.rebuild();
        assert_eq!(g.find(fa), g.find(fb));
    }

    #[test]
    fn union_propagates_two_levels() {
        let mut g = EGraph::new();
        let a = g.add(leaf("a"));
        let b = g.add(leaf("b"));
        let fa = g.add(op("f", vec![a]));
        let fb = g.add(op("f", vec![b]));
        let gfa = g.add(op("g", vec![fa]));
        let gfb = g.add(op("g", vec![fb]));
        g.union(a, b);
        g.rebuild();
        assert_eq!(g.find(gfa), g.find(gfb));
    }

    #[test]
    fn add_expr_shares_subterms() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x x)".parse().unwrap();
        g.add_expr(&e);
        let x_classes = g
            .classes()
            .filter(|c| c.nodes.iter().any(|n| n.op == "x"))
            .count();
        assert_eq!(x_classes, 1);
    }

    #[test]
    fn diamond_congruence() {
        let mut g = EGraph::new();
        let x = g.add(leaf("x"));
        let y = g.add(leaf("y"));
        let fx = g.add(op("f", vec![x]));
        let fy = g.add(op("f", vec![y]));
        let gfx = g.add(op("g", vec![fx]));
        let gfy = g.add(op("g", vec![fy]));
        g.union(x, y);
        g.rebuild();
        assert_eq!(g.find(fx), g.find(fy));
        assert_eq!(g.find(gfx), g.find(gfy));
    }
}
