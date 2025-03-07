use tower_lsp::{
  Client, LanguageServer,
  jsonrpc::Result,
  lsp_types::{
    CodeActionOptions, CodeActionParams, CodeActionResponse, InitializeParams, InitializeResult,
    InitializedParams, ServerCapabilities,
  },
};

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
          // TODO: Specify code action options
          ..Default::default()
        })),
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
    // TODO: Implement code action
    Ok(None)
  }

  #[tracing::instrument(ret)]
  async fn shutdown(&self) -> Result<()> {
    Ok(())
  }
}
