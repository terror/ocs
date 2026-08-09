use super::*;

pub(crate) struct SessionItem {
  pub(crate) cost: Option<String>,
  pub(crate) database: PathBuf,
  pub(crate) id: String,
  pub(crate) model: Option<String>,
  pub(crate) preview: OnceLock<String>,
  pub(crate) project: Option<String>,
  pub(crate) search_text: String,
  pub(crate) title: String,
  pub(crate) tokens: Option<String>,
  pub(crate) updated: String,
}

impl SessionItem {
  fn format_cost(cost: f64) -> Option<String> {
    if cost <= 0.0 {
      None
    } else if cost < 0.01 {
      Some("<$0.01".into())
    } else {
      Some(format!("${cost:.2}"))
    }
  }

  fn format_tokens(tokens: u64) -> Option<String> {
    let tokens = if tokens == 0 {
      return None;
    } else if tokens >= 1_000_000 {
      format!("{}.{}m", tokens / 1_000_000, tokens % 1_000_000 / 100_000)
    } else if tokens >= 1_000 {
      format!("{}.{}k", tokens / 1_000, tokens % 1_000 / 100)
    } else {
      tokens.to_string()
    };

    Some(tokens)
  }

  pub(crate) fn new(
    storage: &Storage,
    session: &Session,
    show_project: bool,
  ) -> Self {
    let project = Path::new(&session.directory)
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or(&session.directory);

    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_millis()
      .try_into()
      .unwrap_or(u64::MAX);

    Self {
      cost: Self::format_cost(session.cost),
      database: storage.database.clone(),
      id: session.id.clone(),
      model: session.model.clone(),
      preview: OnceLock::new(),
      project: show_project.then(|| project.into()),
      search_text: session.search_text(),
      title: session.title.clone(),
      tokens: Self::format_tokens(session.tokens),
      updated: session.time.relative_updated(now),
    }
  }
}

impl SkimItem for SessionItem {
  fn display(&self, _context: DisplayContext) -> Line<'_> {
    let muted = Style::new().fg(DARK_GRAY);

    let mut metadata = Vec::new();

    let mut push_metadata = |value: &str, style: Style| {
      if !metadata.is_empty() {
        metadata.push(Span::styled(" · ", muted));
      }

      metadata.push(Span::styled(value.to_owned(), style));
    };

    if let Some(project) = &self.project {
      push_metadata(project, muted);
    }

    push_metadata(&self.updated, muted);

    if let Some(model) = &self.model {
      push_metadata(model, muted);
    }

    if let Some(cost) = &self.cost {
      push_metadata(cost, muted);
    }

    if let Some(tokens) = &self.tokens {
      push_metadata(tokens, muted);
    }

    let mut line = vec![Span::raw(self.title.as_str()), Span::raw("  ")];

    line.extend(metadata);

    Line::from(line)
  }

  fn output(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.id)
  }

  fn preview(&self, _context: PreviewContext) -> ItemPreview {
    ItemPreview::AnsiText(
      self
        .preview
        .get_or_init(|| {
          Storage::new(self.database.clone())
            .and_then(|storage| storage.get_session(&self.id))
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

  fn session() -> Session {
    Session {
      cost: 0.125,
      directory: "/tmp/bar".into(),
      id: "ses_foo".into(),
      messages: vec![Message {
        id: "msg_foo".into(),
        role: "user".into(),
        session_id: "ses_foo".into(),
        text: "first line\nsecond line".into(),
        time: Time::default(),
      }],
      model: Some("model-foo".into()),
      time: Time {
        created: u64::MAX,
        updated: u64::MAX,
      },
      title: "foo".into(),
      tokens: 12_345,
    }
  }

  #[test]
  fn project_name_visibility() {
    let storage = Storage::new("/tmp/foo.db".into()).unwrap();
    let session = session();

    let item = SessionItem::new(&storage, &session, true);

    let display = item.display(DisplayContext::default());

    assert_eq!(display.spans[0].content, "foo");
    assert_eq!(display.spans[1].content, "  ");
    assert_eq!(display.spans[2].content, "bar");
    assert_eq!(display.spans[2].style.fg, Some(DARK_GRAY));
    assert_eq!(display.spans[3].content, " · ");
    assert_eq!(display.spans[3].style.fg, Some(DARK_GRAY));

    let item = SessionItem::new(&storage, &session, false);

    let display = item.display(DisplayContext::default());

    assert_eq!(display.spans[0].content, "foo");
  }

  #[test]
  fn renders_metadata() {
    let item = SessionItem::new(
      &Storage::new("/tmp/foo.db".into()).unwrap(),
      &session(),
      false,
    );

    let display = item.display(DisplayContext {
      container_width: 100,
      ..DisplayContext::default()
    });

    let text = display
      .spans
      .iter()
      .map(|span| span.content.as_ref())
      .collect::<String>();

    assert!(text.contains("now"));
    assert!(text.contains("model-foo"));
    assert!(text.contains("$0.12"));
    assert!(text.contains("12.3k"));
  }
}
