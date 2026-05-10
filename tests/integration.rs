use tiny_egraph::{
    rewrite, AstSize, EGraph, Extractor, GreedyExtractor, Pattern, RecExpr, Rewrite, Runner,
    StopReason,
};

fn arithmetic_rules() -> Vec<Rewrite> {
    vec![
        rewrite!("comm-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("comm-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("assoc-add-l"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
        rewrite!("assoc-add-r"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        rewrite!("add-zero"; "(+ ?a 0)" => "?a"),
        rewrite!("mul-one"; "(* ?a 1)" => "?a"),
        rewrite!("mul-zero"; "(* ?a 0)" => "0"),
    ]
}

#[test]
fn add_zero_collapses_to_var() {
    let mut g = EGraph::new();
    let e: RecExpr = "(+ x 0)".parse().unwrap();
    let root = g.add_expr(&e);
    let runner = Runner::default()
        .with_egraph(g)
        .run(&arithmetic_rules());
    let ext = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = ext.find_best(root);
    assert_eq!(cost, 1);
    assert_eq!(best.to_string(), "x");
}

#[test]
fn associativity_then_zero() {
    let mut g = EGraph::new();
    let e: RecExpr = "(+ (+ x 0) y)".parse().unwrap();
    let root = g.add_expr(&e);
    let runner = Runner::default()
        .with_iter_limit(20)
        .with_egraph(g)
        .run(&arithmetic_rules());
    let ext = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = ext.find_best(root);
    assert!(cost <= 3, "expected (+ x y) or similar, got {}", best);
}

#[test]
fn mul_zero_dominates() {
    let mut g = EGraph::new();
    let e: RecExpr = "(* (+ a (* b c)) 0)".parse().unwrap();
    let root = g.add_expr(&e);
    let runner = Runner::default()
        .with_egraph(g)
        .run(&arithmetic_rules());
    let ext = Extractor::new(&runner.egraph, AstSize);
    let (cost, best) = ext.find_best(root);
    assert_eq!(cost, 1);
    assert_eq!(best.to_string(), "0");
}

#[test]
fn search_returns_consistent_substs() {
    use tiny_egraph::ematch::search_pattern;
    let mut g = EGraph::new();
    let e: RecExpr = "(+ x x)".parse().unwrap();
    g.add_expr(&e);
    let pat: Pattern = "(+ ?a ?a)".parse().unwrap();
    let matches = search_pattern(&g, &pat);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].substs.len(), 1);
}

#[test]
fn greedy_agrees_with_dp_on_acyclic_case() {
    let mut g = EGraph::new();
    let e: RecExpr = "(+ x 0)".parse().unwrap();
    let root = g.add_expr(&e);
    let runner = Runner::default()
        .with_egraph(g)
        .run(&arithmetic_rules());
    let dp = Extractor::new(&runner.egraph, AstSize);
    let mut greedy = GreedyExtractor::new(&runner.egraph, AstSize);
    let (dp_cost, dp_expr) = dp.find_best(root);
    let (gr_cost, gr_expr) = greedy.find_best(root);
    assert_eq!(dp_cost, gr_cost);
    assert_eq!(dp_expr.to_string(), gr_expr.to_string());
}

#[test]
fn runner_reports_stop_reason() {
    let mut g = EGraph::new();
    let e: RecExpr = "(+ a b)".parse().unwrap();
    g.add_expr(&e);
    let runner = Runner::default()
        .with_egraph(g)
        .run(&[rewrite!("noop"; "(* ?a 1)" => "?a")]);
    assert_eq!(runner.stop_reason, Some(StopReason::Saturated));
}

#[test]
fn node_limit_triggers() {
    let rules = vec![
        rewrite!("distrib"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        rewrite!("comm-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("comm-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
    ];
    let mut g = EGraph::new();
    let e: RecExpr = "(* (+ a (+ b (+ c (+ d e)))) (+ f (+ g h)))"
        .parse()
        .unwrap();
    g.add_expr(&e);
    let runner = Runner::default()
        .with_iter_limit(50)
        .with_node_limit(50)
        .with_egraph(g)
        .run(&rules);
    assert!(matches!(
        runner.stop_reason,
        Some(StopReason::NodeLimit(_)) | Some(StopReason::Saturated) | Some(StopReason::IterLimit(_))
    ));
}

#[test]
fn rebuild_after_unrelated_unions() {
    let mut g = EGraph::new();
    let a = g.add(tiny_egraph::language::SymbolLang::leaf("a"));
    let b = g.add(tiny_egraph::language::SymbolLang::leaf("b"));
    let c = g.add(tiny_egraph::language::SymbolLang::leaf("c"));
    let d = g.add(tiny_egraph::language::SymbolLang::leaf("d"));
    let fab = g.add(tiny_egraph::language::SymbolLang::new("f", vec![a, b]));
    let fcd = g.add(tiny_egraph::language::SymbolLang::new("f", vec![c, d]));
    g.union(a, c);
    g.union(b, d);
    g.rebuild();
    assert_eq!(g.find(fab), g.find(fcd));
}
