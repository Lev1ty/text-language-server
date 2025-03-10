use crate::{
  r#trait::{CodeAction, CommandMeta, ExecuteCommand, Text},
  r#type::{UnescapeSource, WithServer},
};
use futures_lite::FutureExt;
use serde_json::{from_value, to_value};
use std::{collections::HashMap, future::ready};
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{
    self, CodeActionKind, CodeActionOrCommand, CodeActionParams, Command, TextEdit, Url,
    WorkspaceEdit,
  },
};
use unescape::unescape;

impl CommandMeta for UnescapeSource {
  const COMMAND_NAME: &'static str = "text-language-server.unescape";
  const COMMAND_DISPLAY_NAME: &'static str = "Unescape";
}

impl CodeAction for UnescapeSource {
  async fn code_action(&self, params: &CodeActionParams) -> Result<CodeActionOrCommand> {
    Ok(CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
      title: String::from(Self::COMMAND_DISPLAY_NAME),
      kind: Some(CodeActionKind::SOURCE),
      command: Some(Command {
        title: String::from(Self::COMMAND_DISPLAY_NAME),
        command: String::from(Self::COMMAND_NAME),
        arguments: Some(vec![to_value(&params.text_document.uri).map_err(
          |err| {
            format!("Failed to convert text document URI to JSON value: {err:?}")
              .pipe(Error::invalid_params)
          },
        )?]),
      }),
      ..Default::default()
    }))
  }
}

impl ExecuteCommand for WithServer<'_, UnescapeSource> {
  async fn execute_command(
    &self,
    params: &tower_lsp::lsp_types::ExecuteCommandParams,
  ) -> tower_lsp::jsonrpc::Result<Option<serde_json::Value>> {
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
    self
      .server()
      .text()
      .get_async(&uri)
      .await
      .ok_or_else(|| Error::internal_error())?
      .pipe_deref(ToString::to_string)
      .pipe_deref(|content| unescape(content).map(|new_text| (content, new_text)))
      .map(|(content, new_text)| {
        content
          .range_full()
          .pipe(|range| TextEdit { range, new_text })
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
