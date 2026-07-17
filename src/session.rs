use super::*;

const MAX_SEARCH_MESSAGE_CHARS: usize = 512;
const MAX_SEARCH_MESSAGES: usize = 4;

#[derive(Deserialize)]
pub(crate) struct Session {
  #[serde(default)]
  pub(crate) directory: String,
  pub(crate) id: String,
  #[serde(default)]
  pub(crate) messages: Vec<Message>,
  #[serde(default)]
  pub(crate) time: Time,
  #[serde(default)]
  pub(crate) title: String,
}

impl Session {
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
    let mut preview = format!(
      "\x1b[1;38;5;255m{}\x1b[0m\n\x1b[38;5;244mDirectory\x1b[0m  \x1b[2;38;5;248m{}\x1b[0m\n\x1b[38;5;244mSession\x1b[0m    \x1b[2;38;5;248m{}\x1b[0m",
      self.title, self.directory, self.id
    );

    let mut message_count = 0;

    for message in self
      .messages
      .iter()
      .filter(|message| !message.text.is_empty())
    {
      message_count += 1;
      preview.push_str("\n\n");
      preview.push_str(match message.role.as_str() {
        "user" => "\x1b[1;38;5;230mUSER\x1b[0m",
        "assistant" => "\x1b[1;38;5;255mASSISTANT\x1b[0m",
        _ => "\x1b[1;38;5;244mMESSAGE\x1b[0m",
      });
      preview.push('\n');
      preview.push_str(&message.text);
    }

    if message_count == 0 {
      preview.push_str(
        "\n\n\x1b[2;38;5;248mNo text messages stored for this session.\x1b[0m",
      );
    }

    preview
  }

  pub(crate) fn push_message(&mut self, message: Message) {
    self.messages.push(message);
  }

  pub(crate) fn search_text(&self) -> String {
    let mut search_text =
      format!("{}\n{}\n{}", self.title, self.directory, self.id);

    for message in self
      .messages
      .iter()
      .rev()
      .filter(|message| message.role == "user" && !message.text.is_empty())
      .take(MAX_SEARCH_MESSAGES)
    {
      search_text.push('\n');
      let end = message
        .text
        .char_indices()
        .nth(MAX_SEARCH_MESSAGE_CHARS)
        .map_or(message.text.len(), |(index, _)| index);
      search_text.push_str(&message.text[..end]);
    }

    search_text
  }

  pub(crate) fn sort_messages(&mut self) {
    self.messages.sort_by_key(|message| message.time.created);
  }

  pub(crate) fn updated(&self) -> u64 {
    self.time.updated.max(self.time.created)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn search_text_uses_recent_user_messages() {
    let session = Session {
      directory: "bar".into(),
      id: "ses_foo".into(),
      messages: vec![
        Message {
          id: "msg_one".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "one".into(),
          time: Time::default(),
        },
        Message {
          id: "msg_two".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "two".into(),
          time: Time::default(),
        },
        Message {
          id: "msg_three".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "three".into(),
          time: Time::default(),
        },
        Message {
          id: "msg_four".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "four".into(),
          time: Time::default(),
        },
        Message {
          id: "msg_five".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "five".into(),
          time: Time::default(),
        },
        Message {
          id: "msg_six".into(),
          role: "assistant".into(),
          session_id: "ses_foo".into(),
          text: "six".into(),
          time: Time::default(),
        },
      ],
      time: Time::default(),
      title: "foo".into(),
    };

    assert_eq!(
      session.search_text(),
      "foo\nbar\nses_foo\nfive\nfour\nthree\ntwo"
    );
  }

  #[test]
  fn search_text_truncates_messages_at_character_boundary() {
    let text = format!("{}bar", "é".repeat(MAX_SEARCH_MESSAGE_CHARS));
    let session = Session {
      directory: "bar".into(),
      id: "ses_foo".into(),
      messages: vec![Message {
        id: "msg_foo".into(),
        role: "user".into(),
        session_id: "ses_foo".into(),
        text,
        time: Time::default(),
      }],
      time: Time::default(),
      title: "foo".into(),
    };

    assert_eq!(
      session.search_text(),
      format!(
        "foo\nbar\nses_foo\n{}",
        "é".repeat(MAX_SEARCH_MESSAGE_CHARS)
      )
    );
  }
}
