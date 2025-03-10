use crate::{
  r#trait::{CodeAction, CommandMeta, ExecuteCommand, Text},
  r#type::{UnescapeSource, WithServer},
};
use futures_lite::FutureExt;
use serde_json::{Value, from_value, to_value};
use std::{collections::HashMap, future::ready};
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{
    self, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, Command,
    ExecuteCommandParams, Range, TextEdit, Url, WorkspaceEdit,
  },
};
use unescape::unescape;

impl CommandMeta for UnescapeSource {
  const COMMAND_NAMES: &'static [&'static str] = &[
    "text-language-server.source.unescape",
    "text-language-server.quickfix.unescape",
  ];
  const COMMAND_DISPLAY_NAMES: &'static [&'static str] = &["Unescape source", "Unescape selection"];
}

impl CodeAction for UnescapeSource {
  async fn code_action(&self, params: &CodeActionParams) -> Result<CodeActionResponse> {
    Ok(vec![
      CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
        title: String::from(Self::COMMAND_DISPLAY_NAMES[0]),
        kind: Some(CodeActionKind::SOURCE),
        command: Some(Command {
          title: String::from(Self::COMMAND_DISPLAY_NAMES[0]),
          command: String::from(Self::COMMAND_NAMES[0]),
          arguments: Some(vec![to_value(&params.text_document.uri).map_err(
            |err| {
              format!("Failed to convert text document URI to JSON value: {err:?}")
                .pipe(Error::invalid_params)
            },
          )?]),
        }),
        ..Default::default()
      }),
      CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
        title: String::from(Self::COMMAND_DISPLAY_NAMES[1]),
        kind: Some(CodeActionKind::QUICKFIX),
        command: Some(Command {
          title: String::from(Self::COMMAND_DISPLAY_NAMES[1]),
          command: String::from(Self::COMMAND_NAMES[1]),
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
      }),
    ])
  }
}

impl ExecuteCommand for WithServer<'_, UnescapeSource> {
  async fn execute_command(&self, params: &ExecuteCommandParams) -> Result<Option<Value>> {
    let Some(uri) = params
      .arguments
      .first()
      .cloned()
      .map(from_value::<Url>)
      .transpose()
      .map_err(|err| {
        Error::invalid_params(format!(
          "Failed to convert text document URI to JSON value: {err:?}"
        ))
      })?
    else {
      return Err(Error::invalid_params("Missing URI argument".to_string()));
    };
    let range = params
      .arguments
      .get(1)
      .cloned()
      .map(from_value::<Range>)
      .transpose()
      .map_err(|err| {
        Error::invalid_params(format!("Failed to convert range to JSON value: {err:?}"))
      })?;
    self
      .server()
      .text()
      .get_async(&uri)
      .await
      .ok_or_else(|| Error::internal_error())?
      .pipe_deref(|rope| {
        let byte_range = range
          .map(|range| rope.slice(..).range(range))
          .unwrap_or(0..rope.len());
        let range = range.unwrap_or(rope.slice(..).range_full());
        unescape(rope.slice(byte_range).to_string().as_str())
          .map(|new_text| TextEdit { range, new_text })
      })
      .map(|text_edit| Some(HashMap::from_iter([(uri, vec![text_edit])])))
      .map(|changes| WorkspaceEdit {
        changes,
        ..Default::default()
      })
      .map(|request| {
        FutureExt::boxed(async {
          self
            .server()
            .client()
            .apply_edit(request)
            .await
            .map(Some)
            .transpose()
        })
      })
      .unwrap_or(FutureExt::boxed(ready(None)))
      .await
      .transpose()?;
    Ok(None)
  }
}
