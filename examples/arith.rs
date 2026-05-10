//! Arithmetic simplification.

use tiny_egraph::{rewrite, AstSize, EGraph, Extractor, RecExpr, Rewrite, Runner};

fn arithmetic_rules() -> Vec<Rewrite> {
    vec![
        rewrite!("comm-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("comm-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("assoc-add-l"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
        rewrite!("assoc-add-r"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        rewrite!("assoc-mul-l"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
        rewrite!("assoc-mul-r"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        rewrite!("add-zero"; "(+ ?a 0)" => "?a"),
        rewrite!("mul-one";  "(* ?a 1)" => "?a"),
        rewrite!("mul-zero"; "(* ?a 0)" => "0"),
        rewrite!("distrib"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        rewrite!("add-self"; "(+ ?a ?a)" => "(* 2 ?a)"),
    ]
}

fn simplify(input: &str) -> String {
    let parsed: RecExpr = input.parse().unwrap();
    let mut egraph = EGraph::new();
    let root = egraph.add_expr(&parsed);

    let runner = Runner::default()
        .with_iter_limit(20)
        .with_node_limit(2_000)
        .with_egraph(egraph)
        .run(&arithmetic_rules());

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = extractor.find_best(root);
    format!("cost={} expr={} ({} iters)", cost, best, runner.iterations.len())
}

fn main() {
    let cases = [
        "(+ x 0)",
        "(+ 0 x)",
        "(* (+ a b) 2)",
        "(+ (+ a b) (+ c 0))",
        "(* x (+ y 0))",
        "(+ (* 2 x) (* 2 x))",
    ];
    for c in cases {
        println!("{:<30} => {}", c, simplify(c));
    }
}
