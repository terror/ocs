use super::*;

#[derive(Deserialize)]
pub(crate) struct Message {
  pub(crate) id: String,
  #[serde(default)]
  pub(crate) role: String,
  #[serde(rename = "sessionID")]
  pub(crate) session_id: String,
  #[serde(default)]
  pub(crate) text: String,
  #[serde(default)]
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
