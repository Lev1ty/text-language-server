use crate::{
  r#trait::{CodeAction, CommandMeta, ExecuteCommand, Text, Transform},
  r#type::WithServer,
};
use futures_lite::FutureExt;
use serde_json::{Value, from_value, to_value};
use std::{collections::HashMap, convert::identity};
use tap::prelude::*;
use tower_lsp::{
  jsonrpc::{Error, Result},
  lsp_types::{
    self, CodeActionOrCommand, CodeActionParams, Command, ExecuteCommandParams, Range, TextEdit,
    Url, WorkspaceEdit,
  },
};
use tracing::error;

impl<T: CommandMeta + Transform> CodeAction for WithServer<'_, T> {
  async fn code_action(&self, params: &CodeActionParams) -> Result<Vec<CodeActionOrCommand>> {
    ((params.context.only.is_none()
      || params
        .context
        .only
        .iter()
        .flat_map(identity)
        .any(|kind| self.code_action_kind().contains(kind)))
      && self
        .server()
        .text()
        .get_async(&params.text_document.uri)
        .await
        .as_deref()
        .map(|rope| self.code_action_condition(rope.slice(..), params.range))
        .unwrap_or_default())
    .then(|| {
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
    })
    .transpose()?
    .pipe(Vec::from_iter)
    .pipe(Ok)
  }
}

impl<T: Transform + Sync> ExecuteCommand for WithServer<'_, T> {
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
        rope
          .slice(byte_range)
          .pipe(|text| self.transform(text))
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
      .unwrap_or(FutureExt::boxed(std::future::ready(None)))
      .await
      .transpose()
      .inspect_err(|err| error!(?err))
      .map_err(|_| Error::internal_error())?;
    Ok(None)
  }
}
