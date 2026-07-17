pub(crate) struct Message {
  created: u64,
  role: String,
  text: String,
}

impl Message {
  pub(crate) fn created(&self) -> u64 {
    self.created
  }

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

  pub(crate) fn role(&self) -> &str {
    &self.role
  }

  pub(crate) fn text(&self) -> &str {
    &self.text
  }
}
