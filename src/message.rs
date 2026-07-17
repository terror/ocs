use super::*;

pub(crate) struct Message {
  pub(crate) id: String,
  pub(crate) role: String,
  pub(crate) session_id: String,
  pub(crate) text: String,
  pub(crate) time: Time,
}

impl Message {
  pub(crate) fn push_text(&mut self, text: &str) {
    if !self.text.is_empty() {
      self.text.push('\n');
    }

    self.text.push_str(text);
  }
}
