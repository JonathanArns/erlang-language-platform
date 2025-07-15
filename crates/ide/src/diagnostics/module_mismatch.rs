/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

// Diagnostic: module-mismatch
//
// Diagnostic for mismatches between the module attribute name and the path of the given file

use elp_ide_assists::Assist;
use elp_ide_db::elp_base_db::FileId;
use elp_ide_db::source_change::SourceChange;
use elp_syntax::AstNode;
use elp_syntax::SyntaxNode;
use elp_syntax::TextRange;
use elp_syntax::ast;
use elp_text_edit::TextEdit;
use hir::Semantic;

use crate::Diagnostic;
use crate::fix;

pub(crate) fn module_mismatch(
    sema: &Semantic,
    acc: &mut Vec<Diagnostic>,
    file_id: FileId,
    node: &SyntaxNode,
) -> Option<()> {
    let module_name = ast::ModuleAttribute::cast(node.clone())?.name()?;
    // Context for T171541590
    let _ = stdx::panic_context::enter(format!("\nmodule_mismatch: {file_id:?}"));
    let root_id = sema.db.file_source_root(file_id).source_root_id(sema.db);
    let root = sema.db.source_root(root_id).source_root(sema.db);
    let path = root.path_for_file(&file_id).unwrap();
    let filename = path.name_and_extension().unwrap_or_default().0;
    let loc = module_name.syntax().text_range();
    if module_name.text()? != filename {
        let d = Diagnostic::new(
            crate::diagnostics::DiagnosticCode::ModuleMismatch,
            format!("Module name ({module_name}) does not match file name ({filename})"),
            loc,
        )
        .with_fixes(Some(vec![rename_module_to_match_filename(
            file_id, loc, filename,
        )]));
        acc.push(d);
    };
    Some(())
}

fn rename_module_to_match_filename(file_id: FileId, loc: TextRange, filename: &str) -> Assist {
    let mut builder = TextEdit::builder();
    builder.replace(loc, filename.to_string());
    let edit = builder.finish();
    fix(
        "rename_module_to_match_filename",
        &format!("Rename module to: {filename}"),
        SourceChange::from_text_edit(file_id, edit),
        loc,
    )
}

#[cfg(test)]
mod tests {

    use expect_test::expect;

    use crate::tests::check_diagnostics;
    use crate::tests::check_fix;

    #[test]
    fn test_module_mismatch() {
        check_diagnostics(
            r#"
//- /src/foo.erl
-module(bar).
%%      ^^^ 💡 error: Module name (bar) does not match file name (foo)
"#,
        );
        check_fix(
            r#"
//- /src/foo.erl
-module(b~ar).
"#,
            expect![[r#"
-module(foo).
"#]],
        )
    }

    #[test]
    fn test_module_mismatch_correct() {
        check_diagnostics(
            r#"
//- /src/foo.erl
-module(foo).
            "#,
        );
    }
}
