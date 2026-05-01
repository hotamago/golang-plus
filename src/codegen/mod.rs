use std::collections::BTreeSet;

use regex::Regex;

use crate::{
    ast::*,
    sema::{SemanticModel, base_enum_name},
};

pub fn generate_go(program: &Program, model: &SemanticModel) -> String {
    let mut generator = GoGenerator::new(program, model);
    generator.emit_program()
}

struct GoGenerator<'a> {
    program: &'a Program,
    model: &'a SemanticModel,
    imports: BTreeSet<String>,
    sections: Vec<String>,
    tmp_counter: usize,
    needs_try_helper: bool,
    needs_main_wrapper: bool,
}

impl<'a> GoGenerator<'a> {
    fn new(program: &'a Program, model: &'a SemanticModel) -> Self {
        Self {
            program,
            model,
            imports: program.imports.iter().cloned().collect(),
            sections: Vec::new(),
            tmp_counter: 0,
            needs_try_helper: false,
            needs_main_wrapper: false,
        }
    }
}

mod decl;
mod decorators;
mod enums;
mod expr;
mod program;
mod stmt;
mod types;

use types::*;

#[cfg(test)]
mod tests;
