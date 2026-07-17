use super::*;

pub(crate) struct Session {
  pub(crate) directory: String,
  pub(crate) id: String,
  pub(crate) messages: Vec<Message>,
  pub(crate) title: String,
  pub(crate) updated: u64,
}

impl Session {
  pub(crate) fn new(
    directory: String,
    id: String,
    title: String,
    updated: u64,
  ) -> Self {
    Self {
      directory,
      id,
      messages: Vec::new(),
      title,
      updated,
    }
  }

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
    let mut preview =
      format!("{}\n{}\n{}", self.title, self.directory, self.id);

    let mut message_count = 0;

    for message in self
      .messages
      .iter()
      .filter(|message| !message.text.is_empty())
    {
      message_count += 1;
      preview.push_str("\n\n");
      preview.push_str(&message.role.to_uppercase());
      preview.push_str(":\n");
      preview.push_str(&message.text);
    }

    if message_count == 0 {
      preview.push_str("\n\nNo text messages stored for this session.");
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
    self.messages.sort_by_key(|message| message.created);
  }
}
