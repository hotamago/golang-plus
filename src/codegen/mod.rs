use std::collections::BTreeMap;

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
    imports: BTreeMap<String, Option<String>>,
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
            imports: program
                .imports
                .iter()
                .map(|import| (import.path.clone(), import.alias.clone()))
                .collect(),
            sections: Vec::new(),
            tmp_counter: 0,
            needs_try_helper: false,
            needs_main_wrapper: false,
        }
    }

    fn import_binding(&mut self, path: &str, default_name: &str) -> String {
        match self.imports.get(path) {
            Some(Some(alias)) if alias != "_" && alias != "." => alias.clone(),
            Some(None) => default_name.to_string(),
            _ => {
                self.imports.insert(path.to_string(), None);
                default_name.to_string()
            }
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
