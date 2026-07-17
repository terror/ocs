use super::*;

pub(crate) struct Storage {
  pub(crate) data_dir: PathBuf,
}

#[derive(Deserialize)]
struct Part {
  #[serde(rename = "type")]
  pub(crate) kind: String,
  #[serde(rename = "messageID")]
  pub(crate) message_id: String,
  #[serde(default)]
  pub(crate) text: String,
}

impl Storage {
  pub(crate) fn default() -> Result<Self> {
    let data_home = env::var_os("XDG_DATA_HOME")
      .map(PathBuf::from)
      .or_else(|| {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
      })
      .context(
        "could not determine an OpenCode data directory; pass --data-dir",
      )?;

    Ok(Self::new(data_home.join("opencode")))
  }

  pub(crate) fn new(data_dir: PathBuf) -> Self {
    Self { data_dir }
  }

  pub(crate) fn sessions(&self) -> Result<Vec<Session>> {
    let storage = self.data_dir.join("storage");

    let session_paths = json_files(&storage.join("session"))?;

    let mut sessions = session_paths
      .into_iter()
      .map(|path| read_json::<Session>(&path))
      .collect::<Result<Vec<_>>>()?;

    let session_indexes = sessions
      .iter()
      .enumerate()
      .map(|(index, session)| (session.id.clone(), index))
      .collect::<HashMap<_, _>>();

    let mut messages = json_files(&storage.join("message"))?
      .into_iter()
      .map(|path| read_json::<Message>(&path))
      .collect::<Result<Vec<_>>>()?
      .into_iter()
      .filter_map(|message| {
        session_indexes
          .get(&message.session_id)
          .map(|&session_index| (message.id.clone(), (session_index, message)))
      })
      .collect::<HashMap<_, _>>();

    for path in json_files(&storage.join("part"))? {
      let part = read_json::<Part>(&path)?;

      if part.kind == "text"
        && let Some((_, message)) = messages.get_mut(&part.message_id)
      {
        message.push_text(&part.text);
      }
    }

    for (_, (session_index, message)) in messages {
      sessions[session_index].push_message(message);
    }

    for session in &mut sessions {
      session.sort_messages();
    }

    sessions.sort_by(|left, right| {
      right
        .updated()
        .cmp(&left.updated())
        .then_with(|| left.title.cmp(&right.title))
    });

    if sessions.is_empty() {
      bail!("no OpenCode sessions found in {}", self.data_dir.display());
    }

    Ok(sessions)
  }
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>> {
  let mut paths = Vec::<PathBuf>::new();

  let entries = match fs::read_dir(directory) {
    Ok(entries) => entries,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
      return Ok(paths);
    }
    Err(error) => {
      return Err(error).with_context(|| {
        format!(
          "could not read OpenCode storage directory {}",
          directory.display()
        )
      });
    }
  };

  for entry in entries {
    let path = entry
      .with_context(|| {
        format!("could not read entry in {}", directory.display())
      })?
      .path();

    if path.is_dir() {
      paths.extend(json_files(&path)?);
    } else if path
      .extension()
      .is_some_and(|extension| extension == "json")
    {
      paths.push(path);
    }
  }

  paths.sort();
  Ok(paths)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
  let contents = fs::read(path)
    .with_context(|| format!("could not read {}", path.display()))?;

  serde_json::from_slice(&contents)
    .with_context(|| format!("could not parse {}", path.display()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn indexes_text_parts_and_orders_the_preview() {
    let temp = tempfile::tempdir().unwrap();
    let storage = temp.path().join("storage");
    let session = storage.join("session/project/ses_foo.json");
    let message_one = storage.join("message/ses_foo/msg_one.json");
    let message_two = storage.join("message/ses_foo/msg_two.json");
    let part_one = storage.join("part/msg_one/prt_one.json");
    let part_two = storage.join("part/msg_two/prt_two.json");

    for path in [&session, &message_one, &message_two, &part_one, &part_two] {
      fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    fs::write(
      session,
      r#"{"id":"ses_foo","title":"Add picker","directory":"/tmp/foo","time":{"updated":2}}"#,
    )
    .unwrap();
    fs::write(
      message_one,
      r#"{"id":"msg_one","sessionID":"ses_foo","role":"assistant","time":{"created":2}}"#,
    )
    .unwrap();
    fs::write(
      message_two,
      r#"{"id":"msg_two","sessionID":"ses_foo","role":"user","time":{"created":1}}"#,
    )
    .unwrap();
    fs::write(
      part_one,
      r#"{"messageID":"msg_one","type":"text","text":"Use skim"}"#,
    )
    .unwrap();
    fs::write(
      part_two,
      r#"{"messageID":"msg_two","type":"text","text":"Build a picker"}"#,
    )
    .unwrap();

    let sessions = Storage::new(temp.path().to_owned()).sessions().unwrap();

    assert_eq!(
      sessions[0].search_text(),
      "Add picker\n/tmp/foo\nBuild a picker\nUse skim"
    );
    assert_eq!(
      sessions[0].preview(),
      "\x1b[1;38;5;255mAdd picker\x1b[0m\n\x1b[38;5;244mDirectory\x1b[0m  \x1b[2;38;5;248m/tmp/foo\x1b[0m\n\x1b[38;5;244mSession\x1b[0m    \x1b[2;38;5;248mses_foo\x1b[0m\n\n\x1b[1;38;5;230mUSER\x1b[0m\nBuild a picker\n\n\x1b[1;38;5;255mASSISTANT\x1b[0m\nUse skim"
    );
  }
}
