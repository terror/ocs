use super::*;

pub(crate) struct SessionPicker<'a> {
  pub(crate) query: Option<String>,
  pub(crate) sessions: &'a [Session],
}

impl<'a> SessionPicker<'a> {
  pub(crate) fn new(sessions: &'a [Session], query: Option<String>) -> Self {
    Self { query, sessions }
  }

  fn options(query: Option<String>) -> Result<SkimOptions> {
    let mut options = SkimOptionsBuilder::default();

    options
      .height("100%")
      .prompt("> ")
      .header(
        "\x1b[2m↑/↓ up/down • type to search • enter open • esc cancel\x1b[0m",
      )
      .no_hscroll(true)
      .preview("")
      .preview_window("down:50%:wrap");

    if let Some(query) = query {
      options.query(query);
    }

    options
      .build()
      .context("could not configure the session picker")
  }

  pub(crate) fn pick(self) -> Result<Option<String>> {
    let options = Self::options(self.query)?;

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn disables_hscroll_for_custom_display() {
    assert!(SessionPicker::options(None).unwrap().no_hscroll);
  }
}
