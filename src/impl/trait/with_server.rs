use crate::{Server, r#trait::WithServer, r#type};

impl<'a, T> WithServer<'a, Server> for T {
  fn with_server(self, server: &'a Server) -> r#type::WithServer<'a, T> {
    r#type::WithServer::builder()
      .server(server)
      .inner(self)
      .build()
  }
}
