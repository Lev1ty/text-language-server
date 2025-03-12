use crate::{
  r#trait::{CodeAction, CommandMeta, ExecuteCommand, Text, WithServer},
  r#type::{EpochToUTC, Source, Unescape},
};
use bon::Builder;
use getset::Getters;
use ropey::Rope;
use scc::HashMap;
use serde_json::Value;
use std::{ops::Deref, process};
use tap::prelude::*;
use tower_lsp::{
  Client, LanguageServer,
  jsonrpc::Result,
  lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    ExecuteCommandOptions, ExecuteCommandParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
  },
};

#[derive(Debug, Builder, Getters)]
pub struct Server {
  #[getset(get = "pub")]
  client: Client,
  #[getset(get = "pub")]
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
          commands: [Unescape.command_name(), EpochToUTC.command_name()]
            .map(ToString::to_string)
            .pipe(Vec::from_iter),
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
  }

  #[tracing::instrument(ret)]
  async fn did_close(&self, params: DidCloseTextDocumentParams) {
    self.text.remove_async(&params.text_document.uri).await;
  }

  #[tracing::instrument(ret, err)]
  async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    Unescape
      .with_server(self)
      .code_action(&params)
      .await?
      .into_iter()
      .chain(
        Source(Unescape)
          .with_server(self)
          .code_action(&params)
          .await?,
      )
      .chain(EpochToUTC.with_server(self).code_action(&params).await?)
      .pipe(Vec::from_iter)
      .pipe(Some)
      .pipe(Ok)
  }

  #[tracing::instrument(ret, err)]
  async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
    if Unescape.command_name() == params.command.as_str() {
      Unescape.with_server(self).execute_command(&params).await
    } else if EpochToUTC.command_name() == params.command.as_str() {
      EpochToUTC.with_server(self).execute_command(&params).await
    } else {
      Ok(None)
    }
  }
}
