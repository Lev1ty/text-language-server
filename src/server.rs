use scc::HashMap;
use serde_json::{Value, from_value, to_string, to_value};
use std::{collections, ops::Deref, process};
use tower_lsp::{
  Client, LanguageServer,
  jsonrpc::{Error, Result},
  lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionResponse, Command, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, ExecuteCommandOptions, ExecuteCommandParams, InitializeParams,
    InitializeResult, InitializedParams, MessageType, PositionEncodingKind, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit, Url,
    WorkspaceEdit,
  },
};
use unescape::unescape;

use crate::r#trait::Text;

#[derive(Debug, derive_builder::Builder)]
pub struct Server {
  client: Client,
  #[builder(default)]
  text: HashMap<Url, String>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
  #[tracing::instrument(ret)]
  async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
      server_info: None,
      capabilities: ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF8),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
          TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::INCREMENTAL),
            ..Default::default()
          },
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

  #[tracing::instrument]
  async fn initialized(&self, _: InitializedParams) {
    self
      .client
      .log_message(
        MessageType::INFO,
        format!("server initialized! PID: {}", process::id()),
      )
      .await;
  }

  #[tracing::instrument(ret)]
  async fn shutdown(&self) -> Result<()> {
    self
      .client
      .log_message(MessageType::INFO, "server shutdown!")
      .await;
    Ok(())
  }

  #[tracing::instrument]
  async fn did_open(&self, params: DidOpenTextDocumentParams) {
    self
      .text
      .upsert_async(params.text_document.uri, params.text_document.text)
      .await;
  }

  #[tracing::instrument]
  async fn did_change(&self, params: DidChangeTextDocumentParams) {
    self
      .client
      .log_message(
        MessageType::LOG,
        format!(
          "before: {}",
          self
            .text
            .get_async(&params.text_document.uri)
            .await
            .as_deref()
            .map(Deref::deref)
            .unwrap_or_default()
        ),
      )
      .await;
    self
      .client
      .log_message(
        MessageType::LOG,
        to_string(&params.content_changes).unwrap_or_default(),
      )
      .await;
    self
      .text
      .update_async(&params.text_document.uri, |_, text| {
        params.content_changes.into_iter().for_each(|change| {
          if let Some(range) = change.range {
            text.replace_range(text.deref().deref().range(range), &change.text);
          } else {
            *text = change.text;
          }
        })
      })
      .await;
    self
      .client
      .log_message(
        MessageType::LOG,
        format!(
          "after: {}",
          self
            .text
            .get_async(&params.text_document.uri)
            .await
            .as_deref()
            .map(Deref::deref)
            .unwrap_or_default()
        ),
      )
      .await;
  }

  #[tracing::instrument]
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

  #[tracing::instrument(err)]
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
        let content = self
          .text
          .get_async(&uri)
          .await
          .ok_or_else(|| Error::internal_error())?;
        if let Some(new_text) = unescape(&content)
          .as_deref()
          .and_then(|s| s.strip_suffix('\n'))
          .map(ToString::to_string)
        {
          let range = content.deref().deref().range_full();
          let request = WorkspaceEdit {
            changes: Some(collections::HashMap::from_iter([(
              uri,
              vec![TextEdit { range, new_text }],
            )])),
            ..Default::default()
          };
          self
            .client
            .log_message(
              MessageType::LOG,
              format!("request: {}", to_string(&request).unwrap_or_default()),
            )
            .await;
          let result = self.client.apply_edit(request).await;
          self
            .client
            .log_message(
              MessageType::LOG,
              format!("result: {}", to_string(&result).unwrap_or_default()),
            )
            .await;
        }
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
