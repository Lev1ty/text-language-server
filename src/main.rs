use text_language_server::{Error, ServerBuilder};
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() -> Result<(), Error> {
  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .try_init()
    .map_err(Error::TracingSubscriberInit)?;
  let (service, socket) =
    LspService::new(|client| ServerBuilder::default().client(client).build().unwrap());
  Server::new(stdin(), stdout(), socket).serve(service).await;
  Ok(())
}
