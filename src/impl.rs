use crate::r#trait::Text;
use ropey::{LineType, RopeSlice};
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
              .take(position.character as usize)
              .pipe(char::decode_utf16)
              .map(|res| res.unwrap_or(char::REPLACEMENT_CHARACTER))
              .collect::<String>()
              .len()
          })
          .unwrap_or_else(|| s.len().saturating_add(1))
      })
      .take((position.line as usize).saturating_add(1))
      .sum::<usize>()
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

impl Text for RopeSlice<'_> {
  fn position(&self, position: lsp_types::Position) -> usize {
    self
      .lines(LineType::LF_CR)
      .enumerate()
      .map(|(line, s)| {
        (line == position.line as usize)
          .then(|| s.utf16_to_byte_idx(position.character as usize))
          .unwrap_or_else(|| s.len())
      })
      .take((position.line as usize).saturating_add(1))
      .sum::<usize>()
  }

  fn range_full(&self) -> lsp_types::Range {
    lsp_types::Range {
      start: Position::new(0, 0),
      end: self
        .lines(LineType::LF_CR)
        .enumerate()
        .last()
        .map(|(line, s)| Position::new(line as u32, s.len_utf16() as u32))
        .unwrap_or(Position::new(0, 0)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ropey::Rope;
  use tower_lsp::lsp_types::{Position, Range};

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
    assert_eq!(text.position(Position::new(0, 6)), 6); // Position start of emoji
    assert_eq!(text.position(Position::new(0, 9)), 10); // Position end of emoji
    assert_eq!(text.position(Position::new(0, 10)), 10);
    assert_eq!(text.position(Position::new(1, 0)), 11);
    assert_eq!(text.position(Position::new(2, 0)), 27);
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
    assert_eq!(text.position(Position::new(0, 28)), 28); // One character out of bounds for exclusive upper bound
    assert_eq!(text.position(Position::new(0, 29)), 28); // Two characters out of bounds clipped
  }

  #[test]
  fn test_position_new_line() {
    let text = "First line\n";
    assert_eq!(text.position(Position::new(0, 0)), 0);
    assert_eq!(text.position(Position::new(0, 5)), 5);
    assert_eq!(text.position(Position::new(0, 9)), 9);
    assert_eq!(text.as_bytes()[9], b'e');
    assert_eq!(text.position(Position::new(0, 10)), 10);
    assert_eq!(text.position(Position::new(0, 11)), 10);
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

  #[test]
  fn test_range_unescape() {
    let text = r#"{ "text": "hello\n👋\n👋world" }"#;
    // 2025-03-09T20:05:26.535680Z TRACE tower_lsp::codec: <-
    // {"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///Users/lev1ty/Projects/text-language-server/test.json","version":2},"contentChanges":[{"range":{"start":{"line":0,"character":16},"end":{"line":0,"character":18}},"text":"\n"},{"range":{"start":{"line":1,"character":2},"end":{"line":1,"character":4}},"text":"\n"}]}}
    assert_eq!(
      text.range(Range {
        start: Position::new(0, 16),
        end: Position::new(0, 18),
      }),
      16..18
    );
    assert_eq!(text.as_bytes()[16], b'\\');
    assert_eq!(text.as_bytes()[17], b'n');
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(16..18, "\n"))
        .as_str(),
      "{ \"text\": \"hello\n👋\\n👋world\" }"
    );
    let text = "{ \"text\": \"hello\n👋\\n👋world\" }";
    assert_eq!(
      text.range(Range {
        start: Position::new(1, 2),
        end: Position::new(1, 4),
      }),
      21..23
    );
    assert_eq!(text.as_bytes()[21], b'\\');
    assert_eq!(text.as_bytes()[22], b'n');
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(21..23, "\n"))
        .as_str(),
      "{ \"text\": \"hello\n👋\n👋world\" }"
    );
  }

  #[test]
  fn test_range_unescape_line_end() {
    let text = "\\n";
    // 2025-03-09T20:25:42.131072Z TRACE tower_lsp::codec: <-
    // {"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///Users/lev1ty/Projects/text-language-server/test.json","version":1},"contentChanges":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":2}},"text":"\\"}]}}
    assert_eq!(
      text.range(Range {
        start: Position::new(0, 0),
        end: Position::new(0, 2),
      }),
      0..2
    );
    assert_eq!(text.as_bytes()[0], b'\\');
    assert_eq!(text.as_bytes()[1], b'n');
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(0..2, "\n"))
        .as_str(),
      "\n"
    );
  }

  #[test]
  fn test_range_unescape_rope() {
    let text = Rope::from_str(r#"{ "text": "hello\n👋\n👋world" }"#);
    assert_eq!(
      text.slice(..).range(Range {
        start: Position::new(0, 16),
        end: Position::new(0, 18),
      }),
      16..18
    );
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(16..18, "\n"))
        .as_str(),
      "{ \"text\": \"hello\n👋\\n👋world\" }"
    );
    let text = Rope::from_str("{ \"text\": \"hello\n👋\\n👋world\" }");
    assert_eq!(
      text.slice(..).range(Range {
        start: Position::new(1, 2),
        end: Position::new(1, 4),
      }),
      21..23
    );
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(21..23, "\n"))
        .as_str(),
      "{ \"text\": \"hello\n👋\n👋world\" }"
    );
  }

  #[test]
  fn test_range_unescape_line_end_rope() {
    let text = Rope::from_str("\\n");
    assert_eq!(
      text.slice(..).range(Range {
        start: Position::new(0, 0),
        end: Position::new(0, 2),
      }),
      0..2
    );
    assert_eq!(
      String::from(text)
        .tap_mut(|s| s.replace_range(0..2, "\n"))
        .as_str(),
      "\n"
    );
  }
}
