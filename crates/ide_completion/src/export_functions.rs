/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use elp_syntax::AstNode;
use elp_syntax::algo;
use elp_syntax::ast;

use crate::Completion;
use crate::Ctx;
use crate::Kind;
use crate::helpers;

pub(crate) fn add_completions(
    acc: &mut Vec<Completion>,
    Ctx {
        file_position,
        parsed,
        sema,
        ..
    }: &Ctx,
) {
    let node = parsed.value.syntax();
    let prefix = &match algo::find_node_at_offset::<ast::Fa>(node, file_position.offset) {
        Some(fa) => {
            let completion_needed = match fa.arity() {
                Some(arity) => arity.value().is_none(),
                None => true,
            };

            if !completion_needed {
                return;
            }
            fa.fun().and_then(|name| name.text()).unwrap_or_default()
        }
        None => {
            // T126163640 / T125984246
            // When we have better error recovery, delete this branch
            node.token_at_offset(file_position.offset)
                .peekable()
                .peek()
                .map(|token| token.text().to_string())
                .unwrap_or_default()
        }
    };

    let def_map = sema.def_map(file_position.file_id);
    let exported_functions = def_map.get_exported_functions();
    let completions = def_map.get_functions().filter_map(|(na, _)| {
        if exported_functions.contains(na) {
            None
        } else {
            helpers::name_slash_arity_completion(na, prefix, Kind::Function)
        }
    });

    acc.extend(completions);
}

#[cfg(test)]
mod test {
    use expect_test::Expect;
    use expect_test::expect;

    use crate::tests::get_completions;
    use crate::tests::render_completions;

    fn check(code: &str, trigger_character: Option<char>, expect: Expect) {
        let completions = get_completions(code, trigger_character);
        let actual = &render_completions(completions);
        expect.assert_eq(actual);
    }

    #[test]
    fn test_exports() {
        assert!(serde_json::to_string(&lsp_types::CompletionItemKind::FUNCTION).unwrap() == "3");

        check(
            r#"
        -module(sample).
        -export([
            foo~
        ]).
        foo() -> ok.
        foo(X) -> X.
        foon() -> ok.
        bar() -> ok.
        "#,
            None,
            expect![[r#"
                {label:foo/0, kind:Function, contents:SameAsLabel, position:None}
                {label:foo/1, kind:Function, contents:SameAsLabel, position:None}
                {label:foon/0, kind:Function, contents:SameAsLabel, position:None}"#]],
        );
    }

    #[test]
    fn test_exclude_already_exported_functions() {
        assert!(serde_json::to_string(&lsp_types::CompletionItemKind::FUNCTION).unwrap() == "3");

        check(
            r#"
        -module(sample).
        -export([
            function_a/0,
            fun~
        ]).
        function_a() -> ok.
        function_b(X) -> X.
        my_function() -> ok.
        "#,
            None,
            expect![[r#"
                {label:function_b/1, kind:Function, contents:SameAsLabel, position:None}"#]],
        );
    }
}
