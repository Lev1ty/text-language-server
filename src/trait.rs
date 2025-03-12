use crate::r#type;
use ropey::RopeSlice;
use serde_json::Value;
use std::ops;
use tower_lsp::{
  jsonrpc::Result,
  lsp_types::{self, CodeActionKind, CodeActionOrCommand, CodeActionParams, ExecuteCommandParams},
};

pub trait CommandMeta {
  fn command_name(&self) -> &'static str;
  fn command_display_name(&self) -> &'static str;
}

pub trait CodeAction {
  async fn code_action(&self, params: &CodeActionParams) -> Result<Vec<CodeActionOrCommand>>;
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

pub trait Transform {
  fn code_action_kind(&self) -> Vec<CodeActionKind>;
  fn code_action_condition(&self, source: RopeSlice, range: lsp_types::Range) -> bool;
  fn code_action_definition(&self, params: &CodeActionParams) -> Result<CodeActionOrCommand>;
  fn transform(&self, text: RopeSlice) -> Option<String>;
}

pub trait WithServer<'a, S>: Sized {
  fn with_server(self, server: &'a S) -> r#type::WithServer<'a, Self>;
}
