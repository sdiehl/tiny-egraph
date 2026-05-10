//! Equality saturation loop.

use std::time::{Duration, Instant};

use crate::ematch::SearchMatches;
use crate::rewrite::Rewrite;
use crate::EGraph;

#[derive(Clone, Debug)]
pub struct Iteration {
    pub n_classes: usize,
    pub n_nodes: usize,
    pub applied: usize,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    Saturated,
    IterLimit(usize),
    NodeLimit(usize),
    TimeLimit(Duration),
}

pub struct Runner {
    pub egraph: EGraph,
    pub iterations: Vec<Iteration>,
    pub stop_reason: Option<StopReason>,
    iter_limit: usize,
    node_limit: usize,
    time_limit: Duration,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            egraph: EGraph::new(),
            iterations: Vec::new(),
            stop_reason: None,
            iter_limit: 30,
            node_limit: 10_000,
            time_limit: Duration::from_secs(10),
        }
    }
}

impl Runner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_iter_limit(mut self, n: usize) -> Self {
        self.iter_limit = n;
        self
    }

    pub fn with_node_limit(mut self, n: usize) -> Self {
        self.node_limit = n;
        self
    }

    pub fn with_time_limit(mut self, d: Duration) -> Self {
        self.time_limit = d;
        self
    }

    pub fn with_egraph(mut self, egraph: EGraph) -> Self {
        self.egraph = egraph;
        self
    }

    pub fn run(mut self, rules: &[Rewrite]) -> Self {
        let start = Instant::now();
        for iter in 0..self.iter_limit {
            let iter_start = Instant::now();
            let n_classes = self.egraph.number_of_classes();
            let n_nodes = self.egraph.total_size();

            let matches: Vec<(usize, Vec<SearchMatches>)> = rules
                .iter()
                .enumerate()
                .map(|(i, r)| (i, r.search(&self.egraph)))
                .collect();

            let mut applied = 0;
            for (i, m) in matches {
                applied += rules[i].apply(&mut self.egraph, &m);
            }

            self.egraph.rebuild();

            let elapsed = iter_start.elapsed();
            self.iterations.push(Iteration {
                n_classes,
                n_nodes,
                applied,
                elapsed,
            });

            if self.egraph.total_size() > self.node_limit {
                self.stop_reason = Some(StopReason::NodeLimit(self.egraph.total_size()));
                return self;
            }
            if start.elapsed() > self.time_limit {
                self.stop_reason = Some(StopReason::TimeLimit(start.elapsed()));
                return self;
            }
            if applied == 0 {
                self.stop_reason = Some(StopReason::Saturated);
                return self;
            }
            if iter + 1 == self.iter_limit {
                self.stop_reason = Some(StopReason::IterLimit(iter + 1));
                return self;
            }
        }
        self.stop_reason
            .get_or_insert(StopReason::IterLimit(self.iter_limit));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rewrite, RecExpr};

    #[test]
    fn saturates_quickly_when_nothing_to_do() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ x 1)".parse().unwrap();
        g.add_expr(&e);
        let runner = Runner::default()
            .with_egraph(g)
            .run(&[rewrite!("nope"; "(* ?a 1)" => "?a")]);
        assert_eq!(runner.stop_reason, Some(StopReason::Saturated));
    }

    #[test]
    fn associativity_saturates() {
        let mut g = EGraph::new();
        let e: RecExpr = "(+ a (+ b c))".parse().unwrap();
        g.add_expr(&e);
        let rules = vec![
            rewrite!("assoc-l"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
            rewrite!("assoc-r"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        ];
        let runner = Runner::default()
            .with_iter_limit(20)
            .with_egraph(g)
            .run(&rules);
        assert!(matches!(
            runner.stop_reason,
            Some(StopReason::Saturated) | Some(StopReason::IterLimit(_))
        ));
    }
}
