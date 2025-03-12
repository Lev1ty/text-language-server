use crate::{
  r#trait::{CommandMeta, Text, Transform},
  r#type::Unescape,
};
use serde_json::to_value;
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{self, CodeActionKind, CodeActionOrCommand, CodeActionParams, Command},
};
use unescaper::unescape;

impl CommandMeta for Unescape {
  fn command_name(&self) -> &'static str {
    "text-language-server.unescape"
  }

  fn command_display_name(&self) -> &'static str {
    "Unescape"
  }
}

impl Transform for Unescape {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, source: ropey::RopeSlice, range: lsp_types::Range) -> bool {
    source.slice(source.range(range)).chars().any(|c| c == '\\')
  }

  fn code_action_definition(&self, params: &CodeActionParams) -> Result<CodeActionOrCommand> {
    Ok(CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
      title: String::from(self.command_name()),
      command: Some(Command {
        title: String::from(self.command_display_name()),
        command: String::from(self.command_name()),
        arguments: Some(vec![
          to_value(&params.text_document.uri).map_err(|err| {
            format!("Failed to convert text document URI to JSON value: {err:?}")
              .pipe(Error::invalid_params)
          })?,
          to_value(&params.range).map_err(|err| {
            format!("Failed to convert range to JSON value: {err:?}").pipe(Error::invalid_params)
          })?,
        ]),
      }),
      ..Default::default()
    }))
  }

  fn transform(&self, text: ropey::RopeSlice) -> Option<String> {
    unescape(text.to_string().as_str()).ok()
  }
}
