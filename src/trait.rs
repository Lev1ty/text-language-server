use std::ops;
use tower_lsp::lsp_types;

pub trait Text {
  fn position(&self, position: lsp_types::Position) -> usize;
  fn range_full(&self) -> lsp_types::Range;
  fn range(&self, range: lsp_types::Range) -> ops::Range<usize> {
    self.position(range.start)..self.position(range.end)
  }
}
