use crate::r#trait::Text;
use futures_lite::FutureExt;
use ropey::Rope;
use scc::HashMap;
use serde_json::{Value, from_value, to_value};
use std::{collections, future::ready, ops::Deref, process};
use tap::prelude::*;
use tower_lsp::{
  Client, LanguageServer,
  jsonrpc::{Error, Result},
  lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionResponse, Command, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, ExecuteCommandOptions, ExecuteCommandParams, InitializeParams,
    InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
  },
};
use tracing::{debug, error, info, trace};
use unescape::unescape;

#[derive(Debug, derive_builder::Builder)]
pub struct Server {
  client: Client,
  #[builder(default)]
  text: HashMap<Url, Rope>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
  #[tracing::instrument(ret)]
  async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
      server_info: None,
      capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
          TextDocumentSyncKind::INCREMENTAL,
        )),
        code_action_provider: Some(Into::into(CodeActionOptions {
          code_action_kinds: Some(vec![CodeActionKind::SOURCE]),
          ..Default::default()
        })),
        execute_command_provider: Some(ExecuteCommandOptions {
          commands: vec![CommandKind::UnescapeSource.to_string()],
          ..Default::default()
        }),
        ..Default::default()
      },
    })
  }

  #[tracing::instrument(ret)]
  async fn initialized(&self, _: InitializedParams) {
    self
      .client
      .log_message(
        MessageType::INFO,
        format!("Server initialized! PID: {}", process::id()),
      )
      .await;
  }

  #[tracing::instrument(ret)]
  async fn shutdown(&self) -> Result<()> {
    self
      .client
      .log_message(MessageType::INFO, "Server shutdown!")
      .await;
    Ok(())
  }

  #[tracing::instrument(ret)]
  async fn did_open(&self, params: DidOpenTextDocumentParams) {
    self
      .text
      .upsert_async(params.text_document.uri, params.text_document.text.into())
      .await;
  }

  // Uncommenting the tracing::instrument proc macro
  // causes requests to did_change to fail to route
  // #[tracing::instrument(ret)]
  async fn did_change(&self, params: DidChangeTextDocumentParams) {
    self
      .text
      .update_async(&params.text_document.uri, |_, text| {
        params.content_changes.into_iter().for_each(|change| {
          if let Some(range) = change.range {
            let range = text.deref().slice(..).range(range);
            text.remove(range.clone());
            text.insert(range.start, &change.text);
          } else {
            *text = change.text.into();
          }
        })
      })
      .await;
    trace!(
      text = %self
        .text
        .get_async(&params.text_document.uri)
        .await
        .as_deref()
        .map(ToString::to_string)
        .unwrap_or_default()
    );
  }

  #[tracing::instrument(ret)]
  async fn did_close(&self, params: DidCloseTextDocumentParams) {
    self.text.remove_async(&params.text_document.uri).await;
  }

  #[tracing::instrument(ret, err)]
  async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    Ok(Some(vec![CodeActionOrCommand::CodeAction(CodeAction {
      title: CommandKindTitle(CommandKind::UnescapeSource).to_string(),
      kind: Some(CodeActionKind::SOURCE),
      command: Some(Command {
        title: CommandKindTitle(CommandKind::UnescapeSource).to_string(),
        command: CommandKind::UnescapeSource.to_string(),
        arguments: Some(vec![to_value(params.text_document.uri).map_err(|err| {
          Error::invalid_params(format!(
            "Failed to convert text document URI to JSON value: {err:?}"
          ))
        })?]),
      }),
      ..Default::default()
    })]))
  }

  #[tracing::instrument(ret, err)]
  async fn execute_command(&self, mut params: ExecuteCommandParams) -> Result<Option<Value>> {
    match params.command.parse::<CommandKind>() {
      Ok(CommandKind::UnescapeSource) => {
        let Some(uri) = params
          .arguments
          .first_mut()
          .map(std::mem::take)
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
          .text
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
          .map(|text_edit| Some(collections::HashMap::from_iter([(uri, vec![text_edit])])))
          .map(|changes| WorkspaceEdit {
            changes,
            ..Default::default()
          })
          .inspect(|request| debug!(?request))
          .map(|request| {
            FutureExt::boxed(async { self.client.apply_edit(request).await.map(Some).transpose() })
          })
          .unwrap_or(FutureExt::boxed(ready(None)))
          .await
          .transpose()
          .inspect(|res| info!(?res))
          .inspect_err(|err| error!(?err))?;
        Ok(None)
      }
      Err(err) => Err(Error::invalid_params(format!("Invalid command: {err:?}"))),
    }
  }
}

#[derive(strum::Display, strum::EnumString)]
enum CommandKind {
  #[strum(serialize = "text-language-server.source.unescape")]
  UnescapeSource,
}

struct CommandKindTitle(CommandKind);

impl std::fmt::Display for CommandKindTitle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.0 {
      CommandKind::UnescapeSource => write!(f, "Unescape source"),
    }
  }
}
