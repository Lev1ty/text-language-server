use crate::{
  r#trait::{CommandMeta, Text, Transform},
  r#type::Unescape,
};
use tower_lsp::lsp_types::{self, CodeActionKind};
use unescaper::unescape;

impl CommandMeta for Unescape {
  fn command_name(&self) -> &'static str {
    "text-language-server.unescape"
  }

  fn command_display_name(&self) -> &'static str {
    "Unescape"
  }
}

impl Transform for Unescape {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, source: ropey::RopeSlice, range: lsp_types::Range) -> bool {
    source.slice(source.range(range)).chars().any(|c| c == '\\')
  }

  fn transform(&self, text: ropey::RopeSlice) -> Option<String> {
    unescape(text.to_string().as_str()).ok()
  }
}
