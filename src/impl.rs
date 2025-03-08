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
      end: Position::new(
        self.lines().count().saturating_sub(1) as u32,
        self.chars().rev().take_while(|c| c != &'\n').count() as u32,
      ),
    }
  }
}
