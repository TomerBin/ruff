use ruff_python_ast::{self as ast, Alias, Identifier, Stmt};
use ruff_text_size::{Ranged, TextRange};

use ruff_macros::{ViolationMetadata, derive_message_formats};

use crate::checkers::ast::Checker;
use crate::{Edit, Fix, FixAvailability, Violation};

/// ## What it does
/// Checks for submodule imports that are aliased to the submodule name.
///
/// ## Why is this bad?
/// Using the `from` keyword to import the submodule is more concise and
/// readable.
///
/// ## Example
/// ```python
/// import concurrent.futures as futures
/// ```
///
/// Use instead:
/// ```python
/// from concurrent import futures
/// ```
///
/// ## References
/// - [Python documentation: Submodules](https://docs.python.org/3/reference/import.html#submodules)
#[derive(ViolationMetadata)]
pub(crate) struct ManualFromImport {
    module: String,
    name: String,
}

impl Violation for ManualFromImport {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        let ManualFromImport { module, name } = self;
        format!("Use `from {module} import {name}` in lieu of alias")
    }

    fn fix_title(&self) -> Option<String> {
        let ManualFromImport { module, name } = self;
        Some(format!("Replace with `from {module} import {name}`"))
    }
}

/// PLR0402
pub(crate) fn manual_from_import(checker: &Checker, stmt: &Stmt, alias: &Alias, names: &[Alias]) {
    let Some(asname) = &alias.asname else {
        return;
    };
    let Some((module, name)) = alias.name.rsplit_once('.') else {
        return;
    };
    if asname != name {
        return;
    }

    let mut diagnostic = checker.report_diagnostic(
        ManualFromImport {
            module: module.to_string(),
            name: name.to_string(),
        },
        alias.range(),
    );
    if names.len() == 1 {
        let node = ast::StmtImportFrom {
            module: Some(Identifier::new(module.to_string(), TextRange::default())),
            names: vec![Alias {
                name: asname.clone(),
                asname: None,
                range: TextRange::default(),
                node_index: ruff_python_ast::AtomicNodeIndex::dummy(),
            }],
            level: 0,
            range: TextRange::default(),
            node_index: ruff_python_ast::AtomicNodeIndex::dummy(),
        };
        diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
            checker.generator().stmt(&node.into()),
            stmt.range(),
        )));
    }
}
