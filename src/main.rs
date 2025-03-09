use text_language_server::{Error, ServerBuilder};
use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing::level_filters::LevelFilter;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Error> {
  let (writer, _guard) = non_blocking(rolling::never("/tmp", "text-language-server.log"));
  tracing_subscriber::registry()
    .with(LevelFilter::TRACE)
    .with(console_subscriber::spawn())
    .with(tracing_subscriber::fmt::layer().with_writer(writer))
    .try_init()?;
  let (service, socket) =
    LspService::new(|client| ServerBuilder::default().client(client).build().unwrap());
  Server::new(stdin(), stdout(), socket)
    .concurrency_level(usize::MAX)
    .serve(service)
    .await;
  Ok(())
}
