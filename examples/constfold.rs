//! Constant folding via the Analysis API.

use tiny_egraph::analysis::{Analysis, ConstFold};
use tiny_egraph::{AstSize, EGraph, Extractor, RecExpr, SymbolLang};

fn fold(input: &str) -> String {
    let expr: RecExpr = input.parse().unwrap();
    let mut egraph = EGraph::new();
    let root = egraph.add_expr(&expr);
    egraph.rebuild();

    let data = ConstFold.run(&egraph);
    for (id, value) in data {
        if let Some(v) = value {
            let leaf = egraph.add(SymbolLang::leaf(v.to_string()));
            egraph.union(id, leaf);
        }
    }
    egraph.rebuild();

    let extractor = Extractor::new(&egraph, AstSize);
    let (_cost, best) = extractor.find_best(root);
    best.to_string()
}

fn main() {
    let cases = [
        "(+ 2 3)",
        "(* (+ 1 2) (- 10 4))",
        "(+ x (* 2 3))",
        "(neg (+ 1 2))",
    ];
    for c in cases {
        println!("input  : {c}");
        println!("folded : {}", fold(c));
        println!();
    }
}
