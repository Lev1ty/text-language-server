#[derive(Debug, derive_more::Display, thiserror::Error)]
pub enum Error {
  TracingSubscriberInit(Box<dyn std::error::Error + Send + Sync>),
}
