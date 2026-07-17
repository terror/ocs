use super::*;

pub(crate) struct Session {
  directory: String,
  id: String,
  messages: Vec<Message>,
  title: String,
  updated: u64,
}

pub(crate) struct SessionPicker<'a> {
  query: Option<String>,
  sessions: &'a [Session],
}

impl Session {
  pub(crate) fn directory(&self) -> &str {
    &self.directory
  }

  pub(crate) fn id(&self) -> &str {
    &self.id
  }

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
      .filter(|message| !message.text().is_empty())
    {
      message_count += 1;
      preview.push_str("\n\n");
      preview.push_str(&message.role().to_uppercase());
      preview.push_str(":\n");
      preview.push_str(message.text());
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
      .filter(|message| !message.text().is_empty())
    {
      search_text.push('\n');
      search_text.push_str(message.text());
    }

    search_text
  }

  pub(crate) fn sort_messages(&mut self) {
    self.messages.sort_by_key(Message::created);
  }

  pub(crate) fn title(&self) -> &str {
    &self.title
  }

  pub(crate) fn updated(&self) -> u64 {
    self.updated
  }
}

impl<'a> SessionPicker<'a> {
  pub(crate) fn new(sessions: &'a [Session], query: Option<String>) -> Self {
    Self { query, sessions }
  }

  pub(crate) fn pick(self) -> Result<Option<String>> {
    let mut options = SkimOptionsBuilder::default();

    options
      .height("100%")
      .prompt("ocs> ")
      .header(
        "\x1b[2m↑/↓ up/down • type to search • enter open • esc cancel\x1b[0m",
      )
      .preview("")
      .preview_window("down:50%:wrap");

    if let Some(query) = self.query {
      options.query(query);
    }

    let options = options
      .build()
      .context("could not configure the session picker")?;

    let (sender, receiver): (SkimItemSender, SkimItemReceiver) = unbounded();

    let items = self
      .sessions
      .iter()
      .map(|session| Arc::new(SessionItem::new(session)) as Arc<dyn SkimItem>)
      .collect::<Vec<_>>();

    sender
      .send(items)
      .context("could not send sessions to the picker")?;

    drop(sender);

    let output = Skim::run_with(options, Some(receiver))
      .map_err(|error| anyhow::anyhow!("session picker failed: {error:?}"))?;

    if output.is_abort {
      return Ok(None);
    }

    Ok(
      output
        .selected_items
        .first()
        .map(|item| item.output().into_owned()),
    )
  }
}
