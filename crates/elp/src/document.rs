/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use core::str;
use std::ops::Range;

use elp_ide::elp_ide_db::LineIndex;
use lsp_types::TextDocumentContentChangeEvent;

use crate::from_proto::safe_text_range;
use crate::line_endings::LineEndings;

pub struct Document {
    pub content: String,
}

impl Document {
    pub fn vfs_to_salsa(self) -> (String, LineEndings) {
        LineEndings::normalize(self.content)
    }

    pub fn from_bytes(bytes: &[u8]) -> Document {
        let content = match str::from_utf8(bytes) {
            Ok(text) => text.to_string(),
            Err(_err) => {
                // Fall back to lossy latin1 loading of files.
                // This should only affect files from yaws, and
                // possibly OTP that are latin1 encoded.
                bytes.iter().map(|byte| (*byte) as char).collect()
            }
        };
        Document { content }
    }

    // From https://github.com/rust-lang/rust-analyzer/blob/607b9ea160149bacca41c0638f16d372c3b235cd/crates/rust-analyzer/src/lsp_utils.rs#L90
    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        let mut line_index = LineIndex::new(&self.content);

        // The changes we got must be applied sequentially, but can cross lines so we
        // have to keep our line index updated.
        // Some clients (e.g. Code) sort the ranges in reverse. As an optimization, we
        // remember the last valid line in the index and only rebuild it if needed.
        // The VFS will normalize the end of lines to `\n`.
        enum IndexValid {
            All,
            UpToLineExclusive(u32),
        }

        impl IndexValid {
            fn covers(&self, line: u32) -> bool {
                match *self {
                    IndexValid::UpToLineExclusive(to) => to > line,
                    _ => true,
                }
            }
        }

        let mut index_valid = IndexValid::All;
        for change in changes {
            match change.range {
                Some(range) => {
                    if !index_valid.covers(range.end.line) {
                        line_index = LineIndex::new(&self.content);
                    }
                    index_valid = IndexValid::UpToLineExclusive(range.start.line);
                    if let Some(range) = safe_text_range(&line_index, range) {
                        if self.content.is_char_boundary(range.start().into())
                            && self.content.is_char_boundary(range.end().into())
                        {
                            self.content
                                .replace_range(Range::<usize>::from(range), &change.text);
                        } else {
                            log::warn!(
                                "discarding invalid document change char boundary: {:?}",
                                &change
                            );
                        }
                    } else {
                        log::warn!("discarding invalid document change range: {:?}", &change);
                    }
                }
                None => {
                    self.content = change.text;
                    index_valid = IndexValid::UpToLineExclusive(0);
                }
            }
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.content.into_bytes()
    }
}
