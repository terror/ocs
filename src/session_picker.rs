use super::*;

pub(crate) struct SessionPicker<'a> {
  pub(crate) query: Option<String>,
  pub(crate) sessions: &'a [Session],
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
