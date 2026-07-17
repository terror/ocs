use super::*;

pub(crate) struct SessionItem {
  pub(crate) data_dir: PathBuf,
  pub(crate) display: String,
  pub(crate) id: String,
  pub(crate) preview: OnceLock<String>,
  pub(crate) search_text: String,
}

impl SessionItem {
  pub(crate) fn new(storage: &Storage, session: &Session) -> Self {
    let project = Path::new(&session.directory)
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or(&session.directory);

    Self {
      data_dir: storage.data_dir.clone(),
      display: format!("{} • {project}", session.title),
      id: session.id.clone(),
      preview: OnceLock::new(),
      search_text: session.search_text(),
    }
  }
}

impl SkimItem for SessionItem {
  fn display(&self, _context: DisplayContext) -> Line<'_> {
    Line::from(self.display.as_str())
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

    assert_eq!(SessionItem::new(&storage, &session).display, "foo • bar");
  }
}
