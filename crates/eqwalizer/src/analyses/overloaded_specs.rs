/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use elp_types_db::eqwalizer::AST;
use elp_types_db::eqwalizer::EqwalizerDiagnostic;
use elp_types_db::eqwalizer::Pos;
use elp_types_db::eqwalizer::form::ExternalForm;
use elp_types_db::eqwalizer::visitor::Visitor;

struct OverloadedSpecVisitor<'a> {
    diagnostics: &'a mut Vec<EqwalizerDiagnostic>,
}

impl<'a> Visitor<'a, ()> for OverloadedSpecVisitor<'a> {
    fn visit_form(&mut self, form: &'a ExternalForm) -> Result<(), ()> {
        match form {
            ExternalForm::ExternalFunSpec(spec) if spec.types.len() > 1 => {
                if let Some(d) = overloaded_spec_diagnostic(&spec.pos) {
                    self.diagnostics.push(d);
                }
            }
            _ => (),
        }
        Ok(())
    }
}

fn overloaded_spec_diagnostic(pos: &Pos) -> Option<EqwalizerDiagnostic> {
    if let Pos::TextRange(range) = pos {
        Some(EqwalizerDiagnostic {
            range: range.clone().into(),
            message: "overloaded spec".into(),
            uri: "https://fb.me/eqwalizer_stats#overloaded_spec".into(),
            code: "eqwalizer_overloaded_spec".into(),
            expression: None,
            explanation: None,
            diagnostic: None,
        })
    } else {
        None
    }
}

pub(crate) fn overloaded_specs(diagnostics: &mut Vec<EqwalizerDiagnostic>, ast: &AST) {
    let mut visitor = OverloadedSpecVisitor { diagnostics };
    let _ = visitor.visit_ast(ast);
}
