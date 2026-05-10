//! Boolean simplification.

use tiny_egraph::{rewrite, AstSize, EGraph, Extractor, RecExpr, Rewrite, Runner};

fn boolean_rules() -> Vec<Rewrite> {
    vec![
        rewrite!("comm-and"; "(and ?a ?b)" => "(and ?b ?a)"),
        rewrite!("comm-or";  "(or ?a ?b)"  => "(or ?b ?a)"),
        rewrite!("assoc-and-l"; "(and ?a (and ?b ?c))" => "(and (and ?a ?b) ?c)"),
        rewrite!("assoc-and-r"; "(and (and ?a ?b) ?c)" => "(and ?a (and ?b ?c))"),
        rewrite!("assoc-or-l";  "(or ?a (or ?b ?c))"   => "(or (or ?a ?b) ?c)"),
        rewrite!("assoc-or-r";  "(or (or ?a ?b) ?c)"   => "(or ?a (or ?b ?c))"),
        rewrite!("idemp-and"; "(and ?a ?a)" => "?a"),
        rewrite!("idemp-or";  "(or ?a ?a)"  => "?a"),
        rewrite!("and-true"; "(and ?a true)" => "?a"),
        rewrite!("or-false"; "(or ?a false)" => "?a"),
        rewrite!("and-false"; "(and ?a false)" => "false"),
        rewrite!("or-true";   "(or ?a true)"   => "true"),
        rewrite!("absorb-and"; "(and ?a (or ?a ?b))" => "?a"),
        rewrite!("absorb-or";  "(or ?a (and ?a ?b))" => "?a"),
        rewrite!("dm-and"; "(not (and ?a ?b))" => "(or (not ?a) (not ?b))"),
        rewrite!("dm-or";  "(not (or ?a ?b))"  => "(and (not ?a) (not ?b))"),
        rewrite!("dneg"; "(not (not ?a))" => "?a"),
    ]
}

fn simplify(input: &str) -> String {
    let parsed: RecExpr = input.parse().unwrap();
    let mut egraph = EGraph::new();
    let root = egraph.add_expr(&parsed);

    let runner = Runner::default()
        .with_iter_limit(30)
        .with_node_limit(5_000)
        .with_egraph(egraph)
        .run(&boolean_rules());

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = extractor.find_best(root);
    format!(
        "cost={} expr={} (stop={:?}, iters={})",
        cost,
        best,
        runner.stop_reason,
        runner.iterations.len()
    )
}

fn main() {
    let cases = [
        "(and x x)",
        "(or x false)",
        "(or x (and x y))",
        "(and x (or x y))",
        "(not (not p))",
        "(not (and a b))",
        "(or (and x y) (and x (not y)))",
    ];
    for c in cases {
        println!("input  : {}", c);
        println!("output : {}", simplify(c));
        println!();
    }
}
