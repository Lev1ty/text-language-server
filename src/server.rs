use serde_json::{Value, from_value, json, to_string, to_value};
use tokio::fs;
use tower_lsp::{
  Client, LanguageServer,
  jsonrpc::{Error, Result},
  lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionResponse, Command, DocumentChanges, ExecuteCommandOptions, ExecuteCommandParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, ServerCapabilities, TextDocumentEdit,
    TextEdit, Url, WorkspaceEdit,
  },
};
use unescape::unescape;

#[derive(Debug, derive_builder::Builder)]
pub struct Server {
  client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
  #[tracing::instrument(ret)]
  async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
      server_info: None,
      capabilities: ServerCapabilities {
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
    tracing::info!("server initialized!");
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
        // TODO: Implement stateful server and track changes via did_open and did_change notifications.
        // This current implementation has two risks:
        // 1. The file on disk may be stale.
        // 2. This server binary may not have permission to read the file.
        let path = uri.to_file_path().map_err(|_| {
          Error::invalid_params(format!("Could not convert URI to file path: {}", uri))
        })?;
        let content = fs::read_to_string(path)
          .await
          .map_err(|err| Error::invalid_params(format!("Failed to read file content: {err:?}")))?;
        let range = Range {
          start: Position::new(0, 0),
          end: Position::new(content.lines().count() as u32, 0),
        };
        self
          .client
          .log_message(MessageType::LOG, to_string(&range).unwrap_or_default())
          .await;
        let new_text =
          unescape(&content).ok_or_else(|| Error::invalid_params("Failed to unescape content"))?;
        self
          .client
          .apply_edit(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
              text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
              edits: vec![OneOf::Left(TextEdit { range, new_text })],
            }])),
            ..Default::default()
          })
          .await?;
        Ok(Some(json!({"success": true})))
      }
      Err(err) => Err(Error::invalid_params(format!("Invalid command: {err:?}"))),
    }
  }

  #[tracing::instrument(ret)]
  async fn shutdown(&self) -> Result<()> {
    Ok(())
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
