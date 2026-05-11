//! Rewrite rules.

use std::collections::HashSet;

use crate::ematch::{search_pattern, SearchMatches};
use crate::pattern::{instantiate, Pattern};
use crate::EGraph;

#[derive(Clone, Debug)]
pub struct Rewrite {
    pub name: String,
    pub searcher: Pattern,
    pub applier: Pattern,
}

impl Rewrite {
    /// # Errors
    /// Returns an error if the applier references a variable that the searcher does not bind.
    pub fn new(
        name: impl Into<String>,
        searcher: Pattern,
        applier: Pattern,
    ) -> Result<Self, String> {
        let searcher_vars: HashSet<_> = searcher.vars().into_iter().collect();
        for v in applier.vars() {
            if !searcher_vars.contains(&v) {
                return Err(format!("applier variable {v} is not bound by the searcher"));
            }
        }
        Ok(Self {
            name: name.into(),
            searcher,
            applier,
        })
    }

    #[must_use]
    pub fn search(&self, egraph: &EGraph) -> Vec<SearchMatches> {
        search_pattern(egraph, &self.searcher)
    }

    pub fn apply(&self, egraph: &mut EGraph, matches: &[SearchMatches]) -> usize {
        let mut changes = 0;
        for m in matches {
            for subst in &m.substs {
                let new_id = instantiate(&self.applier, subst, egraph);
                if egraph.union(m.eclass, new_id) {
                    changes += 1;
                }
            }
        }
        changes
    }
}

#[macro_export]
macro_rules! rewrite {
    ($name:expr; $lhs:expr => $rhs:expr $(,)?) => {{
        let lhs: $crate::Pattern = $lhs.parse().expect("invalid LHS pattern");
        let rhs: $crate::Pattern = $rhs.parse().expect("invalid RHS pattern");
        $crate::Rewrite::new($name, lhs, rhs).expect("ill-formed rewrite")
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecExpr, Runner, SymbolLang};

    #[test]
    fn applier_var_must_be_bound() {
        let lhs: Pattern = "(+ ?a 0)".parse().unwrap();
        let rhs: Pattern = "?b".parse().unwrap();
        assert!(Rewrite::new("bad", lhs, rhs).is_err());
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn one_step_apply() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x 0)".parse().unwrap();
        let root = g.add_expr(&e);
        let r = rewrite!("zero"; "(+ ?a 0)" => "?a");
        let matches = r.search(&g);
        let n = r.apply(&mut g, &matches);
        g.rebuild();
        assert!(n >= 1);
        let x = g.add(SymbolLang::leaf("x"));
        assert_eq!(g.find(root), g.find(x));
    }

    #[test]
    fn saturate_associativity() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ a (+ b c))".parse().unwrap();
        let root = g.add_expr(&e);
        let rules = vec![
            rewrite!("assoc-l"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
            rewrite!("assoc-r"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        ];
        let runner = Runner::default()
            .with_iter_limit(10)
            .with_egraph(g)
            .run(&rules);
        let other: RecExpr = "(+ (+ a b) c)".parse().unwrap();
        let mut g = runner.egraph;
        let id = g.add_expr(&other);
        g.rebuild();
        assert_eq!(g.find(root), g.find(id));
    }
}
