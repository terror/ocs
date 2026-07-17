use super::*;

#[derive(Deserialize)]
pub(crate) struct Session {
  #[serde(default)]
  pub(crate) directory: String,
  pub(crate) id: String,
  #[serde(default)]
  pub(crate) messages: Vec<Message>,
  #[serde(default)]
  pub(crate) time: Time,
  #[serde(default)]
  pub(crate) title: String,
}

impl Session {
  pub(crate) fn open(&self) -> Result {
    let mut command = Command::new("opencode");

    command.arg("--session").arg(&self.id);

    if Path::new(&self.directory).is_dir() {
      command.current_dir(&self.directory);
    }

    let status = command.status().context("could not start opencode")?;

    if !status.success() {
      bail!("opencode exited with {status}");
    }

    Ok(())
  }

  pub(crate) fn preview(&self) -> String {
    let mut preview = format!(
      "\x1b[1;38;5;255m{}\x1b[0m\n\x1b[38;5;244mDirectory\x1b[0m  \x1b[2;38;5;248m{}\x1b[0m\n\x1b[38;5;244mSession\x1b[0m    \x1b[2;38;5;248m{}\x1b[0m",
      self.title, self.directory, self.id
    );

    let mut message_count = 0;

    for message in self
      .messages
      .iter()
      .filter(|message| !message.text.is_empty())
    {
      message_count += 1;
      preview.push_str("\n\n");
      preview.push_str(match message.role.as_str() {
        "user" => "\x1b[1;38;5;230mUSER\x1b[0m",
        "assistant" => "\x1b[1;38;5;255mASSISTANT\x1b[0m",
        _ => "\x1b[1;38;5;244mMESSAGE\x1b[0m",
      });
      preview.push('\n');
      preview.push_str(&message.text);
    }

    if message_count == 0 {
      preview.push_str(
        "\n\n\x1b[2;38;5;248mNo text messages stored for this session.\x1b[0m",
      );
    }

    preview
  }

  pub(crate) fn push_message(&mut self, message: Message) {
    self.messages.push(message);
  }

  pub(crate) fn search_text(&self) -> String {
    let mut search_text = format!("{}\n{}", self.title, self.directory);

    for message in self
      .messages
      .iter()
      .filter(|message| !message.text.is_empty())
    {
      search_text.push('\n');
      search_text.push_str(&message.text);
    }

    search_text
  }

  pub(crate) fn sort_messages(&mut self) {
    self.messages.sort_by_key(|message| message.time.created);
  }

  pub(crate) fn updated(&self) -> u64 {
    self.time.updated.max(self.time.created)
  }
}
