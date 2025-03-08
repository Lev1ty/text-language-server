use text_language_server::{Error, ServerBuilder};
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing_appender::{non_blocking, rolling};

#[tokio::main]
async fn main() -> Result<(), Error> {
  let (writer, _guard) = non_blocking(rolling::never("/tmp", "text-language-server.log"));
  tracing_subscriber::fmt()
    .with_ansi(false)
    .with_max_level(tracing::Level::DEBUG)
    .with_writer(writer)
    .try_init()
    .map_err(Error::TracingSubscriberInit)?;
  let (service, socket) =
    LspService::new(|client| ServerBuilder::default().client(client).build().unwrap());
  Server::new(stdin(), stdout(), socket).serve(service).await;
  Ok(())
}
