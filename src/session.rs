use super::*;

pub(crate) struct Session {
  id: String,
  title: String,
  directory: String,
  updated: u64,
  messages: Vec<Message>,
}

pub(crate) struct Message {
  role: String,
  created: u64,
  text: String,
}

pub(crate) struct SessionPicker<'a> {
  sessions: &'a [Session],
  query: Option<String>,
}

struct SessionItem {
  id: String,
  search_text: String,
  display: String,
  preview: String,
}

impl Session {
  pub(crate) fn new(
    id: String,
    title: String,
    directory: String,
    updated: u64,
  ) -> Self {
    Self {
      id,
      title,
      directory,
      updated,
      messages: Vec::new(),
    }
  }

  pub(crate) fn id(&self) -> &str {
    &self.id
  }

  pub(crate) fn updated(&self) -> u64 {
    self.updated
  }

  pub(crate) fn title(&self) -> &str {
    &self.title
  }

  pub(crate) fn push_message(&mut self, message: Message) {
    self.messages.push(message);
  }

  pub(crate) fn sort_messages(&mut self) {
    self.messages.sort_by_key(|message| message.created);
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
}

impl Message {
  pub(crate) fn new(role: String, created: u64) -> Self {
    Self {
      role,
      created,
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

impl<'a> SessionPicker<'a> {
  pub(crate) fn new(sessions: &'a [Session], query: Option<String>) -> Self {
    Self { sessions, query }
  }

  pub(crate) fn pick(self) -> Result<Option<String>> {
    let mut options = SkimOptionsBuilder::default();

    options
      .height("100%")
      .prompt("ocs> ")
      .header(
        "Enter: open session  Esc: cancel  Search titles, paths, and chat text",
      )
      .preview("")
      .preview_window("right:60%:wrap");

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

    Ok(
      output
        .selected_items
        .first()
        .map(|item| item.output().into_owned()),
    )
  }
}

impl SessionItem {
  fn new(session: &Session) -> Self {
    Self {
      id: session.id.clone(),
      search_text: session.search_text(),
      display: format!("{}  {}", session.title, session.directory),
      preview: session.preview(),
    }
  }
}

impl SkimItem for SessionItem {
  fn text(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.search_text)
  }

  fn display(&self, _context: DisplayContext) -> ratatui::text::Line<'_> {
    ratatui::text::Line::from(self.display.as_str())
  }

  fn preview(&self, _context: PreviewContext) -> ItemPreview {
    ItemPreview::Text(self.preview.clone())
  }

  fn output(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.id)
  }
}
