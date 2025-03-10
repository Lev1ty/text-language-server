use crate::r#type;
use serde_json::Value;
use std::ops;
use tower_lsp::{
  jsonrpc::Result,
  lsp_types::{self, CodeActionParams, ExecuteCommandParams},
};

pub trait CommandMeta {
  const COMMAND_NAMES: &'static [&'static str];
  const COMMAND_DISPLAY_NAMES: &'static [&'static str];
}

pub trait CodeAction {
  async fn code_action(
    &self,
    params: &CodeActionParams,
  ) -> Result<Vec<lsp_types::CodeActionOrCommand>>;
}

pub trait ExecuteCommand {
  async fn execute_command(&self, params: &ExecuteCommandParams) -> Result<Option<Value>>;
}

pub trait Text {
  fn position(&self, position: lsp_types::Position) -> usize;
  fn range_full(&self) -> lsp_types::Range;
  fn range(&self, range: lsp_types::Range) -> ops::Range<usize> {
    self.position(range.start)..self.position(range.end)
  }
}

pub trait WithServer<'a, S>: Sized {
  fn with_server(self, server: &'a S) -> r#type::WithServer<'a, Self>;
}
