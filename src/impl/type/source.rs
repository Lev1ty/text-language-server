use crate::{
  r#trait::{CommandMeta, Transform},
  r#type::Source,
};
use ropey::RopeSlice;
use tower_lsp::lsp_types::{CodeActionKind, Range};

impl<T: CommandMeta> CommandMeta for Source<T> {
  fn command_name(&self) -> &'static str {
    self.0.command_name()
  }

  fn command_display_name(&self) -> &'static str {
    self.0.command_display_name()
  }
}

impl<T: Transform> Transform for Source<T> {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, _: RopeSlice, range: Range) -> bool {
    range == Default::default()
  }

  fn transform(&self, text: RopeSlice) -> Option<String> {
    self.0.transform(text)
  }
}
