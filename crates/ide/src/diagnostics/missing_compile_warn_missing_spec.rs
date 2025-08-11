/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Lint/fix: missing_compile_warn_missing_spec
//!
//! Return a diagnostic if a the file does not have
//! `warn_missing_spec(_all)` in a compile attribute
//! Add this as a fix.

use elp_ide_assists::helpers::add_compile_option;
use elp_ide_assists::helpers::rename_atom_in_compile_attribute;
use elp_ide_db::elp_base_db::FileId;
use elp_ide_db::elp_base_db::FileKind;
use elp_ide_db::source_change::SourceChangeBuilder;
use elp_syntax::AstNode;
use elp_text_edit::TextRange;
use fxhash::FxHashSet;
use hir::AnyExpr;
use hir::CompileOptionId;
use hir::FoldCtx;
use hir::InFile;
use hir::Literal;
use hir::Name;
use hir::Semantic;
use hir::Strategy;
use hir::Term;
use hir::fold::MacroStrategy;
use hir::fold::ParenStrategy;
use hir::known;
use lazy_static::lazy_static;

use super::DIAGNOSTIC_WHOLE_FILE_RANGE;
use super::Diagnostic;
use super::DiagnosticConditions;
use super::DiagnosticDescriptor;
use crate::fix;

pub(crate) static DESCRIPTOR: DiagnosticDescriptor = DiagnosticDescriptor {
    conditions: DiagnosticConditions {
        experimental: false,
        include_generated: false,
        include_tests: false,
        default_disabled: true,
    },
    checker: &|diags, sema, file_id, file_kind| {
        missing_compile_warn_missing_spec(diags, sema, file_id, file_kind);
    },
};

fn missing_compile_warn_missing_spec(
    diags: &mut Vec<Diagnostic>,
    sema: &Semantic,
    file_id: FileId,
    file_kind: FileKind,
) {
    match file_kind {
        FileKind::Header | FileKind::Other | FileKind::OutsideProjectModel => {
            return;
        }
        _ => {}
    }

    let form_list = sema.form_list(file_id);
    if form_list.compile_attributes().next().is_none() {
        report_diagnostic(sema, None, file_id, (Found::No, None), diags);
    }
    let attributes = form_list
        .compile_attributes()
        .map(|(idx, compile_attribute)| {
            let co = sema.db.compile_body(InFile::new(file_id, idx));
            let is_present = FoldCtx::fold_term(
                Strategy {
                    macros: MacroStrategy::Expand,
                    parens: ParenStrategy::InvisibleParens,
                },
                &co.body,
                co.value,
                (Found::No, None),
                &mut |acc, ctx| match &ctx.item {
                    AnyExpr::Term(Term::Literal(Literal::Atom(atom))) => {
                        let name = sema.db.lookup_atom(*atom);
                        if MISSING_SPEC_ALL_OPTIONS.contains(&name) {
                            (Found::WarnMissingSpecAll, Some(idx))
                        } else if MISSING_SPEC_OPTIONS.contains(&name) {
                            (Found::WarnMissingSpec, Some(idx))
                        } else {
                            acc
                        }
                    }
                    _ => acc,
                },
            );
            (is_present, compile_attribute)
        })
        .collect::<Vec<_>>();

    let what = attributes
        .iter()
        .fold((Found::No, None), |acc, ((present, idx), _)| {
            if acc.0 == Found::No {
                (*present, *idx)
            } else {
                acc
            }
        });
    if what.0 != Found::WarnMissingSpecAll {
        // Report on first compile attribute only
        if let Some((_, compile_attribute)) = attributes.first() {
            let range = compile_attribute
                .form_id
                .get_ast(sema.db, file_id)
                .syntax()
                .text_range();
            report_diagnostic(sema, Some(range), file_id, what, diags)
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Found {
    No,
    WarnMissingSpec,
    WarnMissingSpecAll,
}

lazy_static! {
    static ref MISSING_SPEC_ALL_OPTIONS: FxHashSet<Name> = {
        let mut res = FxHashSet::default();
        for name in [known::warn_missing_spec_all, known::nowarn_missing_spec_all] {
            res.insert(name);
        }
        res
    };
    static ref MISSING_SPEC_OPTIONS: FxHashSet<Name> = {
        let mut res = FxHashSet::default();
        for name in [known::warn_missing_spec, known::nowarn_missing_spec] {
            res.insert(name);
        }
        res
    };
}

fn report_diagnostic(
    sema: &Semantic,
    range: Option<TextRange>,
    file_id: FileId,
    what: (Found, Option<CompileOptionId>),
    diags: &mut Vec<Diagnostic>,
) {
    let range = range.unwrap_or(DIAGNOSTIC_WHOLE_FILE_RANGE);

    let mut builder = SourceChangeBuilder::new(file_id);
    if what.0 == Found::No {
        add_compile_option(sema, file_id, "warn_missing_spec_all", None, &mut builder);
    } else {
        // We already have warn_missing_spec, upgrade it to warn_missing_spec_all
        if let Some(co_id) = what.1 {
            rename_atom_in_compile_attribute(
                sema,
                file_id,
                &co_id,
                "warn_missing_spec",
                "warn_missing_spec_all",
                &mut builder,
            );
        }
    }
    let edit = builder.finish();
    let d = Diagnostic::new(
        crate::diagnostics::DiagnosticCode::MissingCompileWarnMissingSpec,
            "Please add \"-compile(warn_missing_spec_all).\" to the module. If exported functions are not all specced, they need to be specced.".to_string(),
        range,
    ).with_fixes(Some(vec![fix("add_warn_missing_spec_all",
                               "Add compile option 'warn_missing_spec_all'",
                               edit, range)]))
    .with_ignore_fix(sema, file_id);
    diags.push(d);
}

#[cfg(test)]
mod tests {

    use elp_ide_db::DiagnosticCode;
    use expect_test::Expect;
    use expect_test::expect;

    use crate::diagnostics::DiagnosticsConfig;
    use crate::tests::check_diagnostics_with_config;
    use crate::tests::check_fix_with_config;
    use crate::tests::check_specific_fix_with_config;

    #[track_caller]
    pub(crate) fn check_fix(fixture_before: &str, fixture_after: Expect) {
        let config =
            DiagnosticsConfig::default().enable(DiagnosticCode::MissingCompileWarnMissingSpec);
        check_fix_with_config(config, fixture_before, fixture_after)
    }

    #[track_caller]
    pub(crate) fn check_specific_fix(
        assist_label: &str,
        fixture_before: &str,
        fixture_after: Expect,
    ) {
        let config =
            DiagnosticsConfig::default().enable(DiagnosticCode::MissingCompileWarnMissingSpec);
        check_specific_fix_with_config(Some(assist_label), fixture_before, fixture_after, config)
    }

    #[track_caller]
    pub(crate) fn check_diagnostics(fixture: &str) {
        let config = DiagnosticsConfig::default()
            .enable(DiagnosticCode::MissingCompileWarnMissingSpec)
            .disable(DiagnosticCode::NoNoWarnSuppressions)
            .disable(DiagnosticCode::UnspecificInclude);
        check_diagnostics_with_config(config, fixture)
    }

    #[track_caller]
    pub(crate) fn check_diagnostics_no_enable(fixture: &str) {
        let config = DiagnosticsConfig::default().disable(DiagnosticCode::NoNoWarnSuppressions);
        check_diagnostics_with_config(config, fixture)
    }

    #[test]
    fn no_compile_attribute() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            %% <<< 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.

            -module(main).

            "#,
        )
    }

    #[test]
    fn compile_attribute_missing_setting() {
        check_diagnostics(
            r#"
         //- /erl/my_app/src/main.erl
            -module(main).

            -compile([export_all, nowarn_export_all]).
         %% ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.

            "#,
        )
    }

    #[test]
    fn compile_attribute_missing_setting_no_explict_enable() {
        check_diagnostics_no_enable(
            r#"
         //- /erl/my_app/src/main.erl
            -module(main).

            -compile([export_all, nowarn_export_all]).
            "#,
        )
    }

    #[test]
    fn warn_missing_spec_not_ok() {
        check_diagnostics(
            r#"
         //- /erl/my_app/src/main.erl
            -module(main).

            -compile(warn_missing_spec).
         %% ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.

            "#,
        )
    }

    #[test]
    fn nowarn_missing_spec_not_ok() {
        check_diagnostics(
            r#"
         //- /erl/my_app/src/main.erl
            -module(main).

            -compile(nowarn_missing_spec).
         %% ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.

            "#,
        )
    }

    #[test]
    fn warn_missing_spec_all_ok() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -compile(warn_missing_spec_all).

            "#,
        )
    }

    #[test]
    fn nowarn_missing_spec_all_ok() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -compile(nowarn_missing_spec_all).

            "#,
        )
    }

    #[test]
    fn more_than_one_compile_attribute_1() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -compile(warn_missing_spec_all).
            -compile([export_all, nowarn_export_all]).
            "#,
        )
    }

    #[test]
    fn more_than_one_compile_attribute_2() {
        check_diagnostics(
            r#"
         //- /erl/my_app/src/main.erl
            -module(main).

            -compile(export_all).
         %% ^^^^^^^^^^^^^^^^^^^^^ 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.
            -compile(nowarn_export_all).
            "#,
        )
    }

    #[test]
    fn more_than_one_compile_attribute_3() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).
            -compile({nowarn_deprecated_function, {erlang,get_stacktrace,0}}).
            -compile([
                warn_missing_spec_all,
                export_all,
                nowarn_export_all
                ]).

            "#,
        )
    }

    #[test]
    fn not_in_generated_file() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/main.erl
            %% -*- coding: utf-8 -*-
            %% Automatically generated, do not edit
            %% @generated from blah
            %% To generate, see targets and instructions in local Makefile
            %% Version source: git
            -module(main).
            -eqwalizer(ignore).

            "#,
        )
    }

    #[test]
    fn not_in_header_file() {
        check_diagnostics(
            r#"
            //- /erl/my_app/src/header.hrl
              -define(OK, ok).

            //- /erl/my_app/src/main.erl
              -module(main).
              -eqwalizer(ignore).
              -compile([warn_missing_spec_all]).

              -include("header.hrl").

              foo () -> ?OK.

            "#,
        )
    }

    #[test]
    fn not_in_test_or_extra_file() {
        check_diagnostics(
            r#"
            //- /erl/my_app/test/my_SUITE.erl extra:test
               -module(my_SUITE).
               -export([all/0]).
               -export([a/1]).
               all() -> [a].
               a(_Config) ->
                 ok.
            "#,
        )
    }

    #[test]
    fn applies_fix_no_attribute() {
        check_fix(
            r#"
         //- /erl/my_app/src/main.erl
         ~-module(main).

         %% a comment"#,
            expect![[r#"
            -module(main).

            -compile([warn_missing_spec_all]).

            %% a comment"#]],
        );
    }

    #[test]
    fn applies_fix_existing_attribute_list() {
        check_fix(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -c~ompile([export_all, nowarn_export_all]).

            "#,
            expect![[r#"
            -module(main).

            -compile([export_all, nowarn_export_all, warn_missing_spec_all]).

            "#]],
        );
    }

    #[test]
    fn applies_fix_existing_attribute_atom() {
        check_fix(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -c~ompile(export_all).

            "#,
            expect![[r#"
            -module(main).

            -compile([export_all, warn_missing_spec_all]).

            "#]],
        );
    }

    #[test]
    fn applies_fix_existing_attribute_tuple() {
        check_fix(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -c~ompile({foo, bar}).

            "#,
            expect![[r#"
            -module(main).

            -compile([{foo, bar}, warn_missing_spec_all]).

            "#]],
        );
    }

    #[test]
    fn applies_fix_upgrade_to_all() {
        check_fix(
            r#"
            //- /erl/my_app/src/main.erl
            -module(main).

            -c~ompile(warn_missing_spec).

            "#,
            expect![[r#"
            -module(main).

            -compile(warn_missing_spec_all).

            "#]],
        );
    }

    #[test]
    fn applies_fix_elp_ignore_module_level() {
        check_specific_fix(
            "Ignore problem",
            r#"
            //- /erl/my_app/src/main.erl
            ~%% <<< 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.
            
            -module(main).

            "#,
            expect![[r#"
                % elp:ignore W0012 (compile-warn-missing-spec)
                -module(main).

            "#]],
        );
    }

    #[test]
    fn applies_fix_elp_ignore_module_level_header_comments() {
        check_specific_fix(
            "Ignore problem",
            r#"
            //- /erl/my_app/src/main.erl
            ~%% <<< 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.
            %% a comment at the
            %% top of the file

            -module(main).

            "#,
            expect![[r#"
                %% a comment at the
                %% top of the file

                % elp:ignore W0012 (compile-warn-missing-spec)
                -module(main).

            "#]],
        );
    }

    #[test]
    fn applies_fix_elp_ignore_module_level_header_comments_no_module_attribute() {
        check_specific_fix(
            "Ignore problem",
            r#"
            //- /erl/my_app/src/main.erl
            ~%% <<< 💡 error: Please add "-compile(warn_missing_spec_all)." to the module. If exported functions are not all specced, they need to be specced.
            %% a comment at the
            %% top of the file

            "#,
            expect![[r#"
                % elp:ignore W0012 (compile-warn-missing-spec)
                %% a comment at the
                %% top of the file

            "#]],
        );
    }

    #[test]
    fn ignore_is_honoured() {
        check_diagnostics(
            r#"
         //- /erl/my_app/src/main.erl
         % elp:ignore W0012 (compile-warn-missing-spec)
         %% a comment at the
         %% top of the file
            "#,
        )
    }
}
