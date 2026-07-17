pub(crate) struct Message {
  pub(crate) created: u64,
  pub(crate) role: String,
  pub(crate) text: String,
}

impl Message {
  pub(crate) fn new(role: String, created: u64) -> Self {
    Self {
      created,
      role,
      text: String::new(),
    }
  }

  pub(crate) fn push_text(&mut self, text: &str) {
    if !self.text.is_empty() {
      self.text.push('\n');
    }

    self.text.push_str(text);
  }
}
