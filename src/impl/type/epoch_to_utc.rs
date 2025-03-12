use crate::{
  r#trait::{CommandMeta, Text, Transform},
  r#type::EpochToUTC,
};
use chrono::{DateTime, SecondsFormat, Utc};
use ropey::RopeSlice;
use serde_json::to_value;
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{self, CodeActionKind, CodeActionOrCommand, CodeActionParams, Command, Range},
};

impl CommandMeta for EpochToUTC {
  fn command_name(&self) -> &'static str {
    "text-language-server.epoch-to-utc"
  }

  fn command_display_name(&self) -> &'static str {
    "Epoch to UTC"
  }
}

impl Transform for EpochToUTC {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, source: RopeSlice, range: Range) -> bool {
    source
      .slice(source.range(range))
      .to_string()
      .parse::<i64>()
      .is_ok()
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

  fn transform(&self, text: RopeSlice) -> Option<String> {
    text
      .to_string()
      .parse::<i64>()
      .ok()
      .and_then(|secs| DateTime::<Utc>::from_timestamp(secs, 0))
      .map(|datetime| datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
  }
}
