use crate::r#trait::Text;
use std::ops;
use tower_lsp::lsp_types::{self, Position};

impl Text for &str {
  fn position(&self, position: lsp_types::Position) -> usize {
    self
      .lines()
      .take(position.line as usize)
      .map(|line| line.len() + 1)
      .sum::<usize>()
      + position.character as usize
  }

  fn range(&self, range: lsp_types::Range) -> ops::Range<usize> {
    let start = self.position(range.start);
    let end = self.position(range.end);
    start..end
  }

  fn range_full(&self) -> lsp_types::Range {
    lsp_types::Range {
      start: Position::new(0, 0),
      end: self
        .lines()
        .enumerate()
        .last()
        .map(|(line, s)| Position::new(line as u32, s.chars().count() as u32))
        .unwrap_or(Position::new(0, 0)),
    }
  }
}
