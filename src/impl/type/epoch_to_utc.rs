use crate::{
  r#trait::{CommandMeta, Text, Transform},
  r#type::EpochToUTC,
};
use chrono::{DateTime, SecondsFormat, Utc};
use ropey::RopeSlice;
use tower_lsp::lsp_types::{CodeActionKind, Range};

impl CommandMeta for EpochToUTC {
  fn command_name(&self) -> &'static str {
    "text-language-server.epoch-to-utc"
  }

  fn command_display_name(&self) -> &'static str {
    "Epoch to UTC"
  }
}

impl Transform for EpochToUTC {
  fn code_action_kind(&self) -> Vec<CodeActionKind> {
    vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]
  }

  fn code_action_condition(&self, source: RopeSlice, range: Range) -> bool {
    source
      .slice(source.range(range))
      .to_string()
      .parse::<i64>()
      .is_ok()
  }

  fn transform(&self, text: RopeSlice) -> Option<String> {
    text
      .to_string()
      .parse::<i64>()
      .ok()
      .and_then(|secs| DateTime::<Utc>::from_timestamp(secs, 0))
      .map(|datetime| datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
  }
}
