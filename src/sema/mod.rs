use std::collections::{BTreeSet, HashMap};

use crate::{ast::*, diag::Diagnostic};

mod decorators;
pub mod lint;
mod matches;
mod model;
mod types;

use decorators::validate_decorators;
use matches::{analyze_match_stmt, infer_enum_type_from_expr};
use model::analyze_block;
use types::validate_try_usage;

pub use model::{SemanticModel, analyze, analyze_with_model, base_enum_name, build_model};

#[cfg(test)]
mod tests;
