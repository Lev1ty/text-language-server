use crate::{r#trait::Transform, r#type::Source};
use ropey::RopeSlice;
use tower_lsp::{
  jsonrpc::Result,
  lsp_types::{CodeActionKind, CodeActionOrCommand, CodeActionParams, Range},
};

impl<T: Transform> Transform for Source<T> {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, _: RopeSlice, range: Range) -> bool {
    range == Default::default()
  }

  fn code_action_definition(&self, params: &CodeActionParams) -> Result<CodeActionOrCommand> {
    self.0.code_action_definition(params)
  }

  fn transform(&self, text: RopeSlice) -> Option<String> {
    self.0.transform(text)
  }
}
