use anyhow::Context;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::cmds::fmt;

fn byte_index_to_position(str: &str, bindex: usize) -> Position {
    let mut cursor = bindex;
    let mut line_pos = 0;
    let mut col = 0;

    for line in str.lines() {
        // TODO: support CRLF EOL sequence.
        let len = line.len() + 1; // +1 for \n

        if len > cursor {
            col = cursor as u32;
            break;
        }

        cursor -= len;
        line_pos += 1;
    }

    Position::new(line_pos, col)
}

pub fn format(source: &str) -> anyhow::Result<Option<Vec<TextEdit>>> {
    // Format.
    let formatted_source = fmt::format_str(source).context("failed to format lua file")?;

    let mut edits = Vec::new();

    // Generate TextEdit.
    let diffs = similar::TextDiff::from_chars(source, &formatted_source);
    for op in diffs.ops() {
        match op {
            similar::DiffOp::Equal { .. } => continue,
            similar::DiffOp::Delete {
                old_index,
                old_len,
                new_index: _,
            } => edits.push(TextEdit {
                range: Range::new(
                    byte_index_to_position(source, *old_index),
                    byte_index_to_position(source, *old_index + *old_len),
                ),
                new_text: "".to_owned(),
            }),
            similar::DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => edits.push(TextEdit {
                range: Range::new(
                    byte_index_to_position(source, *old_index),
                    byte_index_to_position(source, *old_index),
                ),
                new_text: formatted_source[*new_index..*new_index + *new_len].to_owned(),
            }),
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => edits.push(TextEdit {
                range: Range::new(
                    byte_index_to_position(source, *old_index),
                    byte_index_to_position(source, *old_index + *old_len),
                ),
                new_text: formatted_source[*new_index..*new_index + *new_len].to_owned(),
            }),
        }
    }

    if edits.is_empty() {
        Ok(None)
    } else {
        Ok(Some(edits))
    }
}
