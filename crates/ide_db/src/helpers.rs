/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! A module with ide helpers for high-level ide features.

use elp_syntax::AstNode;
use elp_syntax::SourceFile;
use elp_syntax::SyntaxKind;
use elp_syntax::SyntaxNode;
use elp_syntax::SyntaxToken;
use elp_syntax::TokenAtOffset;
use elp_syntax::ast;
use elp_text_edit::TextSize;
use hir::FormList;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnippetCap {
    _private: (),
}

impl SnippetCap {
    pub const fn new(allow_snippets: bool) -> Option<SnippetCap> {
        if allow_snippets {
            Some(SnippetCap { _private: () })
        } else {
            None
        }
    }
}

/// Picks the token with the highest rank returned by the passed in function.
pub fn pick_best_token(
    tokens: TokenAtOffset<SyntaxToken>,
    f: impl Fn(SyntaxKind) -> usize,
) -> Option<SyntaxToken> {
    tokens.max_by_key(move |t| f(t.kind()))
}

pub fn get_call(syntax: &SyntaxNode) -> Option<ast::Call> {
    if let Some(call) = ast::Call::cast(syntax.parent()?) {
        Some(call)
    } else {
        ast::Call::cast(syntax.parent()?.parent()?)
    }
}

/// Find the first position at the top of the file to add a new
/// form. It will be just after the module attribute, if there is one.
pub fn top_insert_position(form_list: &FormList, source: &SourceFile) -> TextSize {
    if let Some(module_attr) = form_list.module_attribute() {
        let module_attr_range = module_attr.form_id.get(source).syntax().text_range();
        module_attr_range.end() + TextSize::from(1)
    } else {
        TextSize::from(0)
    }
}
