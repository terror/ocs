use super::*;

pub(crate) struct SessionItem {
  display: String,
  id: String,
  preview: String,
  search_text: String,
}

impl SessionItem {
  pub(crate) fn new(session: &Session) -> Self {
    Self {
      display: format!("{}  {}", session.title(), session.directory()),
      id: session.id().to_owned(),
      preview: session.preview(),
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
    ItemPreview::Text(self.preview.clone())
  }

  fn text(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.search_text)
  }
}
