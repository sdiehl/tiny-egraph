//! Minimal pedagogical e-graph implementation.

pub mod egraph;
pub mod ematch;
pub mod extract;
pub mod id;
pub mod language;
pub mod pattern;
pub mod rewrite;
pub mod runner;
pub mod unionfind;

#[cfg(feature = "analysis")]
pub mod analysis;

pub use egraph::{EClass, EGraph};
pub use ematch::SearchMatches;
pub use extract::{AstDepth, AstSize, CostFunction, Extractor, GreedyExtractor};
pub use id::Id;
pub use language::{ParseError, RecExpr, SymbolLang};
pub use pattern::{Pattern, Subst, Var};
pub use rewrite::Rewrite;
pub use runner::{Iteration, Runner, StopReason};
pub use unionfind::UnionFind;
