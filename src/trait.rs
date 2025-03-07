use std::ops;
use tower_lsp::lsp_types;

pub trait Text {
  fn position(&self, position: lsp_types::Position) -> usize;
  fn range(&self, range: lsp_types::Range) -> ops::Range<usize>;
  fn range_full(&self) -> lsp_types::Range;
}
