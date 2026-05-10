//! Array/loop algebra.

use tiny_egraph::{rewrite, AstSize, EGraph, Extractor, RecExpr, Rewrite, Runner};

fn array_rules() -> Vec<Rewrite> {
    vec![
        rewrite!("map-fuse";
            "(map ?f (map ?g ?xs))" => "(map (compose ?f ?g) ?xs)"),
        rewrite!("map-id"; "(map id ?xs)" => "?xs"),
        rewrite!("length-map"; "(length (map ?f ?xs))" => "(length ?xs)"),
        rewrite!("length-range"; "(length (range ?n))" => "?n"),
        rewrite!("sum-ones"; "(fold + 0 (map (const 1) ?xs))" => "(length ?xs)"),
        rewrite!("compose-id-l"; "(compose id ?f)" => "?f"),
        rewrite!("compose-id-r"; "(compose ?f id)" => "?f"),
    ]
}

fn simplify(input: &str) -> String {
    let parsed: RecExpr = input.parse().unwrap();
    let mut egraph = EGraph::new();
    let root = egraph.add_expr(&parsed);

    let runner = Runner::default()
        .with_iter_limit(15)
        .with_egraph(egraph)
        .run(&array_rules());

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = extractor.find_best(root);
    format!("cost={} expr={}", cost, best)
}

fn main() {
    let cases = [
        "(map id xs)",
        "(map f (map g xs))",
        "(length (range 100))",
        "(fold + 0 (map (const 1) xs))",
        "(length (map h xs))",
        "(compose id (compose f id))",
    ];
    for c in cases {
        println!("input  : {}", c);
        println!("output : {}", simplify(c));
        println!();
    }
}
