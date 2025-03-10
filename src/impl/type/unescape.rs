use crate::{
  r#trait::{CodeAction, CommandMeta, ExecuteCommand, Text},
  r#type::{Unescape, WithServer},
};
use futures_lite::FutureExt;
use serde_json::{Value, from_value, to_value};
use std::{collections::HashMap, convert::identity, future::ready};
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{
    self, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, Command,
    ExecuteCommandParams, Range, TextEdit, Url, WorkspaceEdit,
  },
};
use tracing::error;
use unescaper::unescape;

impl CommandMeta for Unescape {
  const COMMAND_NAMES: &'static [&'static str] = &[
    "text-language-server.source.unescape",
    "text-language-server.quickfix.unescape",
  ];
  const COMMAND_DISPLAY_NAMES: &'static [&'static str] = &["Unescape source", "Unescape selection"];
}

impl CodeAction for WithServer<'_, Unescape> {
  async fn code_action(&self, params: &CodeActionParams) -> Result<CodeActionResponse> {
    let mut actions = vec![];
    actions.push(CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
      title: String::from(Unescape::COMMAND_DISPLAY_NAMES[0]),
      kind: Some(CodeActionKind::SOURCE),
      command: Some(Command {
        title: String::from(Unescape::COMMAND_DISPLAY_NAMES[0]),
        command: String::from(Unescape::COMMAND_NAMES[0]),
        arguments: Some(vec![to_value(&params.text_document.uri).map_err(
          |err| {
            format!("Failed to convert text document URI to JSON value: {err:?}")
              .pipe(Error::invalid_params)
          },
        )?]),
      }),
      ..Default::default()
    }));
    if (params.context.only.is_none()
      || params
        .context
        .only
        .iter()
        .flat_map(identity)
        .any(|kind| kind == &CodeActionKind::SOURCE || kind == &CodeActionKind::QUICKFIX))
      && self
        .server()
        .text()
        .get_async(&params.text_document.uri)
        .await
        .as_deref()
        .map(|rope| {
          rope
            .slice(rope.slice(..).range(params.range))
            .chars()
            .any(|c| c == '\\')
        })
        .unwrap_or_default()
    {
      actions.push(CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
        title: String::from(Unescape::COMMAND_DISPLAY_NAMES[1]),
        kind: Some(CodeActionKind::QUICKFIX),
        command: Some(Command {
          title: String::from(Unescape::COMMAND_DISPLAY_NAMES[1]),
          command: String::from(Unescape::COMMAND_NAMES[1]),
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
      }));
    }
    Ok(actions)
  }
}

impl ExecuteCommand for WithServer<'_, Unescape> {
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
      .transpose()
      .inspect_err(|err| error!(?err))
      .map_err(|_| Error::internal_error())?;
    Ok(None)
  }
}
