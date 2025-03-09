use crate::r#trait::Text;
use std::ops;
use tower_lsp::lsp_types::{self, Position};

impl Text for &str {
  fn position(&self, position: lsp_types::Position) -> usize {
    self
      .lines()
      .enumerate()
      .map(|(line, s)| {
        s.chars()
          .take(
            (line == position.line as usize)
              .then_some(position.character)
              .unwrap_or(u32::MAX) as usize,
          )
          .map(char::len_utf8)
          .sum::<usize>()
          + ((line != position.line as usize) as usize)
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
        .lines()
        .enumerate()
        .last()
        .map(|(line, s)| Position::new(line as u32, s.chars().count() as u32))
        .unwrap_or(Position::new(0, 1)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tower_lsp::lsp_types::Position;

  #[test]
  fn test_position_ascii() {
    let text = "Hello\nworld\ntest";

    // First line
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.position(Position::new(0, 1)), 1);
    assert_eq!(text.position(Position::new(0, 4)), 4);
    assert_eq!(text.position(Position::new(0, 5)), 5);
    assert_eq!(text.position(Position::new(0, 6)), 5);

    // Second line
    assert_eq!(text.position(Position::new(1, 0)), 6);
    assert_eq!(text.position(Position::new(1, 3)), 9);
    assert_eq!(text.position(Position::new(1, 5)), 11);

    // Third line
    assert_eq!(text.position(Position::new(2, 0)), 12);
    assert_eq!(text.position(Position::new(2, 4)), 16);
  }

  #[test]
  fn test_position_unicode() {
    // Text with Unicode characters: emoji and non-ASCII characters
    let text = "Hello 😊\n北京 Shanghai\n←↑→↓";

    // First line (with emoji is a single character but counts as one position)
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.position(Position::new(0, 6)), 6); // Position before emoji
    assert_eq!(text.position(Position::new(0, 7)), 10); // Position after emoji

    // Second line with Chinese characters (each is one character)
    assert_eq!(text.position(Position::new(1, 0)), 11);
    assert_eq!(text.position(Position::new(1, 1)), 14); // After "北"
    assert_eq!(text.position(Position::new(1, 2)), 17); // After "京"
    assert_eq!(text.position(Position::new(1, 10)), 25); // End of line

    // Third line with arrow symbols (each is one character)
    assert_eq!(text.position(Position::new(2, 0)), 27);
    assert_eq!(text.position(Position::new(2, 2)), 33); // After first two arrows
    assert_eq!(text.position(Position::new(2, 4)), 39); // End of text
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
    assert_eq!(text.position(Position::new(0, 28)), 28); // Last character
    assert_eq!(text.position(Position::new(0, 29)), 28); // One character out of bounds
  }
}
