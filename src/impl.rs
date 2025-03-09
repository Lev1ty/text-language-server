use crate::r#trait::Text;
use std::ops;
use tap::prelude::*;
use tower_lsp::lsp_types::{self, Position};

impl Text for &str {
  fn position(&self, position: lsp_types::Position) -> usize {
    self
      .lines()
      .enumerate()
      .map(|(line, s)| {
        (line == position.line as usize)
          .then(|| {
            s.encode_utf16()
              .take((position.character as usize).saturating_add(1))
              .pipe(char::decode_utf16)
              .map(|res| res.unwrap_or(char::REPLACEMENT_CHARACTER))
              .collect::<String>()
              .len()
              .saturating_sub(1)
          })
          .unwrap_or_else(|| s.len().saturating_add(1))
      })
      .take((position.line as usize).saturating_add(1))
      .sum::<usize>()
  }

  fn range(&self, range: lsp_types::Range) -> ops::Range<usize> {
    self.position(range.start)..self.position(range.end)
  }

  fn range_full(&self) -> lsp_types::Range {
    lsp_types::Range {
      start: Position::new(0, 0),
      end: self
        .ends_with('\n')
        .then(|| Position::new(self.lines().count() as u32, 0))
        .unwrap_or_else(|| {
          self
            .lines()
            .enumerate()
            .last()
            .map(|(line, s)| Position::new(line as u32, s.encode_utf16().count() as u32))
            .unwrap_or(Position::new(0, 0))
        }),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tower_lsp::lsp_types::{Position, Range};

  #[test]
  fn test_position_ascii() {
    let text = "Hello\nworld\ntest";

    // First line
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.position(Position::new(0, 1)), 1);
    assert_eq!(text.position(Position::new(0, 4)), 4);
    assert_eq!(text.position(Position::new(0, 5)), 4);

    // Second line
    assert_eq!(text.position(Position::new(1, 0)), 6);
    assert_eq!(text.position(Position::new(1, 3)), 9);
    assert_eq!(text.position(Position::new(1, 4)), 10);

    // Third line
    assert_eq!(text.position(Position::new(2, 0)), 12);
    assert_eq!(text.position(Position::new(2, 3)), 15);
  }

  #[test]
  fn test_position_unicode() {
    // Text with Unicode characters: emoji and non-ASCII characters
    let text = "Hello 😊\n北京 Shanghai\n←↑→↓";

    // First line (with emoji is a single character but counts as one position)
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.as_bytes()[5], b' ');
    assert_eq!(text.position(Position::new(0, 6)), 8); // Position start of emoji
    assert_eq!(text.position(Position::new(0, 9)), 9); // Position end of emoji
    assert_eq!(text.position(Position::new(1, 0)), 13);
    assert_eq!(text.position(Position::new(2, 0)), 29);
  }

  #[test]
  fn test_position_empty_text() {
    let text = "";
    assert_eq!(text.position(Position::new(0, 0)), 0);
  }

  #[test]
  fn test_position_single_line() {
    let text = "Single line with no newlines";
    assert_eq!(text.position(Position::new(0, 10)), 10);
    assert_eq!(text.position(Position::new(0, 27)), 27); // Last character
    assert_eq!(text.position(Position::new(0, 28)), 27); // One character out of bounds
  }

  #[test]
  fn test_position_new_line() {
    let text = "First line\n";
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.position(Position::new(0, 5)), 5);
    assert_eq!(text.position(Position::new(0, 9)), 9);
    assert_eq!(text.as_bytes()[9], b'e');
    assert_eq!(text.position(Position::new(0, 10)), 9);
    assert_eq!(text.position(Position::new(0, 11)), 9);
    assert_eq!(text.position(Position::new(1, 0)), 11);
    assert_eq!(text.position(Position::new(1, 1)), 11);
    assert_eq!(text.as_bytes()[10], b'\n');
  }

  #[test]
  fn test_range() {
    let text = r#"{ "text": "hello\n👋\n👋world" }"#;
    assert_eq!(text.lines().count(), 1);
    assert_eq!(
      text.range(Range {
        start: Position::new(0, 29),
        end: Position::new(0, 29),
      }),
      33..33
    );
    assert_eq!(text.as_bytes()[33], b'"');
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(33..33, " "))
        .as_str(),
      r#"{ "text": "hello\n👋\n👋world " }"#
    );
  }
}
