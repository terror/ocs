use super::*;

pub(crate) struct Storage {
  pub(crate) data_dir: PathBuf,
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

  pub(crate) fn session(&self, id: &str) -> Result<Session> {
    let database = self.data_dir.join("opencode.db");

    let connection =
      Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
          format!("could not open OpenCode database {}", database.display())
        })?;

    let session = connection
      .query_row(
        "SELECT id, directory, title, time_created, time_updated FROM session WHERE id = ?",
        [id],
        |row| {
          Ok(Session {
            id: row.get(0)?,
            directory: row.get(1)?,
            title: row.get(2)?,
            messages: Vec::new(),
            time: Time {
              created: row.get_u64(3)?,
              updated: row.get_u64(4)?,
            },
          })
        },
      )
      .optional()
      .context("could not query OpenCode session")?
      .context("selected session was not indexed")?;

    let messages = {
      let mut statement = connection
        .prepare(
          "
            SELECT
              id,
              session_id,
              time_created,
              COALESCE(json_extract(data, '$.role'), '')
            FROM message
            WHERE session_id = ?
            ORDER BY time_created
          ",
        )
        .context("could not query OpenCode messages")?;

      statement
        .query_map([id], |row| {
          Ok(Message {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(3)?,
            text: String::new(),
            time: Time {
              created: row.get_u64(2)?,
              updated: 0,
            },
          })
        })
        .context("could not read OpenCode messages")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode messages")?
    };

    let parts = {
      let mut statement = connection
        .prepare(
          "
            SELECT
              message_id,
              COALESCE(json_extract(data, '$.type'), ''),
              COALESCE(json_extract(data, '$.text'), '')
            FROM part
            WHERE session_id = ?
            ORDER BY time_created
          ",
        )
        .context("could not query OpenCode parts")?;

      statement
        .query_map([id], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
          ))
        })
        .context("could not read OpenCode parts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode parts")?
    };

    let mut messages = messages
      .into_iter()
      .map(|message| (message.id.clone(), message))
      .collect::<HashMap<_, _>>();

    for (message_id, kind, text) in parts {
      if kind == "text"
        && let Some(message) = messages.get_mut(&message_id)
      {
        message.push_text(&text);
      }
    }

    let mut session = session;

    for (_, message) in messages {
      session.push_message(message);
    }

    session.sort_messages();

    Ok(session)
  }

  pub(crate) fn sessions(&self) -> Result<Vec<Session>> {
    let database = self.data_dir.join("opencode.db");

    let connection =
      Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
          format!("could not open OpenCode database {}", database.display())
        })?;

    let mut sessions = {
      let mut statement = connection
        .prepare(
          "SELECT id, directory, title, time_created, time_updated FROM session",
        )
        .context("could not query OpenCode sessions")?;

      statement
        .query_map([], |row| {
          Ok(Session {
            id: row.get(0)?,
            directory: row.get(1)?,
            title: row.get(2)?,
            messages: Vec::new(),
            time: Time {
              created: row.get_u64(3)?,
              updated: row.get_u64(4)?,
            },
          })
        })
        .context("could not read OpenCode sessions")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode sessions")?
    };

    let messages = {
      let mut statement = connection
        .prepare(
          "
            SELECT id, session_id, time_created
            FROM (
              SELECT
                id,
                session_id,
                time_created,
                ROW_NUMBER() OVER (
                  PARTITION BY session_id
                  ORDER BY time_created DESC
                ) AS position
              FROM message
              WHERE json_extract(data, '$.role') = 'user'
            )
            WHERE position <= 4
          ",
        )
        .context("could not query OpenCode messages")?;

      statement
        .query_map([], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get_u64(2)?,
          ))
        })
        .context("could not read OpenCode messages")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode messages")?
    };

    let parts = {
      let mut statement = connection
        .prepare(
          "
            WITH recent_messages AS (
              SELECT
                id,
                session_id,
                ROW_NUMBER() OVER (
                  PARTITION BY session_id
                  ORDER BY time_created DESC
                ) AS position
              FROM message
              WHERE json_extract(data, '$.role') = 'user'
            )
            SELECT
              part.message_id,
              substr(COALESCE(json_extract(part.data, '$.text'), ''), 1, 512)
            FROM part
            JOIN recent_messages ON recent_messages.id = part.message_id
            WHERE recent_messages.position <= 4
              AND json_extract(part.data, '$.type') = 'text'
            ORDER BY part.time_created
          ",
        )
        .context("could not query OpenCode parts")?;

      statement
        .query_map([], |row| {
          Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .context("could not read OpenCode parts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode parts")?
    };

    let messages = messages
      .into_iter()
      .map(|(id, session_id, created)| Message {
        id,
        session_id,
        role: "user".into(),
        text: String::new(),
        time: Time {
          created,
          updated: 0,
        },
      })
      .collect::<Vec<_>>();

    let session_indexes = sessions
      .iter()
      .enumerate()
      .map(|(index, session)| (session.id.clone(), index))
      .collect::<HashMap<_, _>>();

    let mut messages = messages
      .into_iter()
      .filter_map(|message| {
        session_indexes
          .get(&message.session_id)
          .map(|&session_index| (message.id.clone(), (session_index, message)))
      })
      .collect::<HashMap<_, _>>();

    for (message_id, text) in parts {
      if let Some((_, message)) = messages.get_mut(&message_id) {
        message.push_text(&text);
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn indexes_sqlite_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("opencode.db");
    let connection = Connection::open(database).unwrap();

    connection
      .execute_batch(
        r#"
          CREATE TABLE session (
            id TEXT NOT NULL,
            directory TEXT NOT NULL,
            title TEXT NOT NULL,
            time_created INTEGER NOT NULL,
            time_updated INTEGER NOT NULL
          );
          CREATE TABLE message (
            id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            time_created INTEGER NOT NULL,
            data TEXT NOT NULL
          );
          CREATE TABLE part (
            message_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            time_created INTEGER NOT NULL,
            data TEXT NOT NULL
          );
          INSERT INTO session VALUES ('ses_foo', '/tmp/foo', 'Add picker', 1, 2);
          INSERT INTO message VALUES ('msg_one', 'ses_foo', 2, '{"role":"assistant"}');
          INSERT INTO message VALUES ('msg_two', 'ses_foo', 1, '{"role":"user"}');
          INSERT INTO part VALUES ('msg_one', 'ses_foo', 2, '{"type":"text","text":"Use skim"}');
          INSERT INTO part VALUES ('msg_two', 'ses_foo', 1, '{"type":"text","text":"Build a picker"}');
        "#,
      )
      .unwrap();

    let sessions = Storage::new(temp.path().to_owned()).sessions().unwrap();

    assert_eq!(
      sessions[0].search_text(),
      "Add picker\n/tmp/foo\nses_foo\nBuild a picker"
    );
    assert_eq!(sessions[0].messages.len(), 1);

    let session = Storage::new(temp.path().to_owned())
      .session("ses_foo")
      .unwrap();

    assert_eq!(
      session.preview(),
      "\x1b[1;38;5;255mAdd picker\x1b[0m\n\x1b[38;5;244mDirectory\x1b[0m  \x1b[2;38;5;248m/tmp/foo\x1b[0m\n\x1b[38;5;244mSession\x1b[0m    \x1b[2;38;5;248mses_foo\x1b[0m\n\n\x1b[1;38;5;230mUSER\x1b[0m\nBuild a picker\n\n\x1b[1;38;5;255mASSISTANT\x1b[0m\nUse skim"
    );
  }
}
