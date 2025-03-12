use crate::server::Server;
use bon::Builder;
use getset::Getters;

#[derive(derive_more::Deref, Builder, Getters)]
pub struct WithServer<'a, T> {
  #[getset(get = "pub")]
  server: &'a Server,
  #[deref]
  inner: T,
}

pub struct Source<T>(pub T);

pub struct EpochToUTC;
pub struct Unescape;
