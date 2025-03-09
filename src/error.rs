use tracing_subscriber::util::TryInitError;

#[derive(Debug, derive_more::Display, thiserror::Error)]
pub enum Error {
  TracingSubscriberInit(#[from] TryInitError),
}
