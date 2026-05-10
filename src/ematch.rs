//! E-matching: recursively search the graph for a pattern.

use crate::egraph::EGraph;
use crate::language::SymbolLang;
use crate::pattern::{Pattern, PatternNode, Subst};
use crate::Id;

#[derive(Clone, Debug)]
pub struct SearchMatches {
    pub eclass: Id,
    pub substs: Vec<Subst>,
}

pub fn search_pattern(egraph: &EGraph, pat: &Pattern) -> Vec<SearchMatches> {
    let mut out = Vec::new();
    for class in egraph.classes() {
        let substs = search_eclass(egraph, pat, class.id);
        if !substs.is_empty() {
            out.push(SearchMatches {
                eclass: class.id,
                substs,
            });
        }
    }
    out
}

pub fn search_eclass(egraph: &EGraph, pat: &Pattern, eclass: Id) -> Vec<Subst> {
    let root = pat.root();
    let mut out = Vec::new();
    match_pattern(pat, root, egraph, eclass, &Subst::new(), &mut out);
    out.sort_by_key(|s: &Subst| {
        s.iter()
            .map(|(v, i)| (v.0.clone(), i.raw()))
            .collect::<Vec<_>>()
    });
    out.dedup();
    out
}

fn match_pattern(
    pat: &Pattern,
    node_id: Id,
    egraph: &EGraph,
    eclass: Id,
    subst: &Subst,
    out: &mut Vec<Subst>,
) {
    let canon = egraph.find(eclass);
    match pat.get(node_id) {
        PatternNode::Var(v) => match subst.get(v) {
            Some(bound) => {
                if egraph.find(bound) == canon {
                    out.push(subst.clone());
                }
            }
            None => {
                let mut s = subst.clone();
                s.insert(v.clone(), canon);
                out.push(s);
            }
        },
        PatternNode::Op { op, children } => {
            let class = match egraph.get_class(canon) {
                Some(c) => c,
                None => return,
            };
            for enode in &class.nodes {
                if enode.op != *op || enode.children.len() != children.len() {
                    continue;
                }
                match_children(pat, children, enode, 0, egraph, subst, out);
            }
        }
    }
}

fn match_children(
    pat: &Pattern,
    child_pats: &[Id],
    enode: &SymbolLang,
    i: usize,
    egraph: &EGraph,
    subst: &Subst,
    out: &mut Vec<Subst>,
) {
    if i == child_pats.len() {
        out.push(subst.clone());
        return;
    }
    let child_pat = child_pats[i];
    let child_class = enode.children[i];
    if let PatternNode::Var(v) = pat.get(child_pat) {
        let canon = egraph.find(child_class);
        match subst.get(v) {
            Some(bound) if egraph.find(bound) != canon => return,
            Some(_) => {
                match_children(pat, child_pats, enode, i + 1, egraph, subst, out);
            }
            None => {
                let mut s = subst.clone();
                s.insert(v.clone(), canon);
                match_children(pat, child_pats, enode, i + 1, egraph, &s, out);
            }
        }
        return;
    }
    let mut child_substs = Vec::new();
    match_pattern(
        pat,
        child_pat,
        egraph,
        child_class,
        subst,
        &mut child_substs,
    );
    for s in child_substs {
        match_children(pat, child_pats, enode, i + 1, egraph, &s, out);
    }
}

#[allow(dead_code)]
pub(crate) fn flatten_matches(matches: Vec<SearchMatches>) -> Vec<(Id, Subst)> {
    matches
        .into_iter()
        .flat_map(|m| {
            let id = m.eclass;
            m.substs.into_iter().map(move |s| (id, s))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::SymbolLang;
    use crate::pattern::Var;

    fn leaf(op: &str) -> SymbolLang {
        SymbolLang::leaf(op)
    }

    #[test]
    fn match_leaf_var() {
        let mut g = EGraph::new();
        let _x = g.add(leaf("x"));
        let pat: Pattern = "?a".parse().unwrap();
        let matches = search_pattern(&g, &pat);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn match_compound_with_var() {
        let mut g = EGraph::new();
        let one = g.add(leaf("1"));
        let x = g.add(leaf("x"));
        let _add = g.add(SymbolLang::new("+", vec![x, one]));
        let pat: Pattern = "(+ ?a 1)".parse().unwrap();
        let matches = search_pattern(&g, &pat);
        assert_eq!(matches.len(), 1);
        let subst = &matches[0].substs[0];
        assert_eq!(subst.get(&Var::new("a")), Some(g.find(x)));
    }

    #[test]
    fn match_requires_consistent_var() {
        let mut g = EGraph::new();
        let x = g.add(leaf("x"));
        let y = g.add(leaf("y"));
        let _xx = g.add(SymbolLang::new("+", vec![x, x]));
        let _xy = g.add(SymbolLang::new("+", vec![x, y]));
        let pat: Pattern = "(+ ?a ?a)".parse().unwrap();
        let flat = flatten_matches(search_pattern(&g, &pat));
        assert_eq!(flat.len(), 1);
    }
}
