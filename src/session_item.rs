use super::*;

pub(crate) struct SessionItem {
  pub(crate) data_dir: PathBuf,
  pub(crate) id: String,
  pub(crate) preview: OnceLock<String>,
  pub(crate) project: Option<String>,
  pub(crate) search_text: String,
  pub(crate) title: String,
}

impl SessionItem {
  pub(crate) fn new(
    storage: &Storage,
    session: &Session,
    show_project: bool,
  ) -> Self {
    let project = show_project
      .then(|| {
        Path::new(&session.directory)
          .file_name()
          .and_then(|name| name.to_str())
          .unwrap_or(&session.directory)
      })
      .map(String::from);

    Self {
      data_dir: storage.data_dir.clone(),
      id: session.id.clone(),
      preview: OnceLock::new(),
      project,
      search_text: session.search_text(),
      title: session.title.clone(),
    }
  }
}

impl SkimItem for SessionItem {
  fn display(&self, _context: DisplayContext) -> Line<'_> {
    let project = match &self.project {
      Some(project) => vec![
        Span::raw(" "),
        Span::styled(project.as_str(), Style::new().fg(DARK_GRAY)),
      ],
      None => Vec::new(),
    };

    Line::from(
      std::iter::once(Span::raw(self.title.as_str()))
        .chain(project)
        .collect::<Vec<_>>(),
    )
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
            .delete_session(&self.id)
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
  fn displays_the_project_name_with_all() {
    let storage = Storage::new("/tmp/foo".into());

    let session = Session {
      directory: "/tmp/bar".into(),
      id: "ses_foo".into(),
      messages: Vec::new(),
      time: Time::default(),
      title: "foo".into(),
    };

    let item = SessionItem::new(&storage, &session, true);

    let display = item.display(DisplayContext::default());

    assert_eq!(display.spans[0].content, "foo");
    assert_eq!(display.spans[1].content, " ");
    assert_eq!(display.spans[2].content, "bar");
    assert_eq!(display.spans[2].style.fg, Some(DARK_GRAY));
  }

  #[test]
  fn hides_the_project_name_by_default() {
    let storage = Storage::new("/tmp/foo".into());

    let session = Session {
      directory: "/tmp/bar".into(),
      id: "ses_foo".into(),
      messages: Vec::new(),
      time: Time::default(),
      title: "foo".into(),
    };

    let item = SessionItem::new(&storage, &session, false);

    let display = item.display(DisplayContext::default());

    assert_eq!(display.spans.len(), 1);
    assert_eq!(display.spans[0].content, "foo");
  }
}
