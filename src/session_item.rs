use super::*;

pub(crate) struct SessionItem {
  pub(crate) data_dir: PathBuf,
  pub(crate) id: String,
  pub(crate) preview: OnceLock<String>,
  pub(crate) project: String,
  pub(crate) search_text: String,
  pub(crate) title: String,
}

impl SessionItem {
  pub(crate) fn new(storage: &Storage, session: &Session) -> Self {
    let project = Path::new(&session.directory)
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or(&session.directory);

    Self {
      data_dir: storage.data_dir.clone(),
      id: session.id.clone(),
      preview: OnceLock::new(),
      project: project.into(),
      search_text: session.search_text(),
      title: session.title.clone(),
    }
  }
}

impl SkimItem for SessionItem {
  fn display(&self, _context: DisplayContext) -> Line<'_> {
    Line::from(vec![
      Span::raw(self.title.as_str()),
      Span::raw(" "),
      Span::styled(self.project.as_str(), Style::new().fg(Color::DarkGray)),
    ])
  }

  fn output(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.id)
  }

  fn preview(&self, _context: PreviewContext) -> ItemPreview {
    ItemPreview::AnsiText(
      self
        .preview
        .get_or_init(|| {
          Storage::new(self.data_dir.clone())
            .session(&self.id)
            .map_or_else(
              |error| format!("could not load preview: {error}"),
              |session| session.preview(),
            )
        })
        .clone(),
    )
  }

  fn text(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.search_text)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn displays_the_project_name() {
    let storage = Storage::new("/tmp/foo".into());
    let session = Session {
      directory: "/tmp/bar".into(),
      id: "ses_foo".into(),
      messages: Vec::new(),
      time: Time::default(),
      title: "foo".into(),
    };

    let item = SessionItem::new(&storage, &session);
    let display = item.display(DisplayContext::default());

    assert_eq!(display.spans[0].content, "foo");
    assert_eq!(display.spans[1].content, " ");
    assert_eq!(display.spans[2].content, "bar");
    assert_eq!(display.spans[2].style.fg, Some(Color::DarkGray));
  }
}
