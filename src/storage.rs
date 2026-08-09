use super::*;

const REQUIRED_SCHEMA: &[(&str, &[&str])] = &[
  (
    "session_v2",
    &[
      "id",
      "directory",
      "parent_id",
      "slug",
      "title",
      "model",
      "cost",
      "tokens_input",
      "tokens_output",
      "tokens_reasoning",
      "tokens_cache_read",
      "tokens_cache_write",
      "time_created",
      "time_updated",
    ],
  ),
  (
    "session_message",
    &["id", "session_id", "type", "seq", "time_created", "data"],
  ),
];

pub(crate) struct Storage {
  pub(crate) database: PathBuf,
}

impl Storage {
  pub(crate) fn default() -> Result<Self> {
    if let Some(database) = env::var_os("OPENCODE_DB")
      .filter(|database| !database.is_empty())
      .map(PathBuf::from)
    {
      return Self::new(database);
    }

    let data_home = env::var_os("XDG_DATA_HOME")
      .map(PathBuf::from)
      .or_else(|| {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
      })
      .context(
        "could not determine an OpenCode data directory; pass --data-dir or --database",
      )?;

    Self::new(data_home.join("opencode").join("opencode-next.db"))
  }

  pub(crate) fn delete_session(&self, id: &str) -> Result {
    let status = Command::new("opencode2")
      .args(["api", "delete", &format!("/api/session/{id}")])
      .env("OPENCODE_DB", &self.database)
      .status()
      .context("could not start opencode2")?;

    if !status.success() {
      bail!("opencode2 exited with {status}");
    }

    Ok(())
  }

  pub(crate) fn get_session(&self, id: &str) -> Result<Session> {
    let connection = self.open()?;
    let mut session = connection
      .query_row(
        "
          SELECT
            id,
            directory,
            COALESCE(title, slug),
            time_created,
            time_updated,
            cost,
            COALESCE(json_extract(model, '$.id'), ''),
            tokens_input + tokens_output + tokens_reasoning
              + tokens_cache_read + tokens_cache_write
          FROM session_v2
          WHERE id = ?
        ",
        [id],
        |row| {
          let model = row.get::<_, String>(6)?;

          Ok(Session {
            cost: row.get(5)?,
            directory: row.get(1)?,
            id: row.get(0)?,
            model: (!model.is_empty()).then_some(model),
            time: Time {
              created: row.get_u64(3)?,
              updated: row.get_u64(4)?,
            },
            title: row.get(2)?,
            tokens: row.get_u64(7)?,
            ..Default::default()
          })
        },
      )
      .optional()
      .context("could not query OpenCode session")?
      .context("selected session was not indexed")?;

    let mut statement = connection
      .prepare(
        "
          SELECT
            id,
            type,
            time_created,
            CASE
              WHEN type = 'assistant' THEN COALESCE((
                SELECT group_concat(json_extract(value, '$.text'), char(10))
                FROM json_each(json_extract(data, '$.content'))
                WHERE json_extract(value, '$.type') = 'text'
              ), '')
              ELSE COALESCE(json_extract(data, '$.text'), '')
            END
          FROM session_message
          WHERE session_id = ?
          ORDER BY seq
        ",
      )
      .context("could not query OpenCode messages")?;

    let messages = statement
      .query_map([id], |row| {
        Ok(Message {
          id: row.get(0)?,
          role: row.get(1)?,
          session_id: id.into(),
          text: row.get(3)?,
          time: Time {
            created: row.get_u64(2)?,
            updated: 0,
          },
        })
      })
      .context("could not read OpenCode messages")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode messages")?;

    for message in messages {
      session.push_message(message);
    }

    Ok(session)
  }

  pub(crate) fn new(database: PathBuf) -> Result<Self> {
    let database = if database.is_absolute() {
      database
    } else {
      env::current_dir()
        .context("could not determine the current directory")?
        .join(database)
    };

    Ok(Self { database })
  }

  fn open(&self) -> Result<Connection> {
    Connection::open_with_flags(
      &self.database,
      OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .with_context(|| {
      format!(
        "could not open OpenCode database {}",
        self.database.display()
      )
    })
  }

  pub(crate) fn sessions(
    &self,
    directory: Option<&Path>,
  ) -> Result<Vec<Session>> {
    let connection = self.open()?;
    let mut statement = connection
      .prepare(
        "
          SELECT
            id,
            directory,
            COALESCE(title, slug),
            time_created,
            time_updated,
            cost,
            COALESCE(json_extract(model, '$.id'), ''),
            tokens_input + tokens_output + tokens_reasoning
              + tokens_cache_read + tokens_cache_write
          FROM session_v2
          WHERE parent_id IS NULL
        ",
      )
      .context("could not query OpenCode sessions")?;

    let mut sessions = statement
      .query_map([], |row| {
        let model = row.get::<_, String>(6)?;

        Ok(Session {
          cost: row.get(5)?,
          directory: row.get(1)?,
          id: row.get(0)?,
          model: (!model.is_empty()).then_some(model),
          time: Time {
            created: row.get_u64(3)?,
            updated: row.get_u64(4)?,
          },
          title: row.get(2)?,
          tokens: row.get_u64(7)?,
          ..Default::default()
        })
      })
      .context("could not read OpenCode sessions")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode sessions")?;

    if let Some(directory) = directory {
      sessions.retain(|session| Path::new(&session.directory) == directory);
    }

    let session_indexes = sessions
      .iter()
      .enumerate()
      .map(|(index, session)| (session.id.clone(), index))
      .collect::<HashMap<_, _>>();

    let mut statement = connection
      .prepare(
        "
          SELECT id, session_id, time_created, COALESCE(json_extract(data, '$.text'), '')
          FROM (
            SELECT
              *,
              ROW_NUMBER() OVER (
                PARTITION BY session_id
                ORDER BY seq DESC
              ) AS position
            FROM session_message
            WHERE type = 'user'
          )
          WHERE position <= 4
        ",
      )
      .context("could not query OpenCode messages")?;

    let messages = statement
      .query_map([], |row| {
        Ok(Message {
          id: row.get(0)?,
          session_id: row.get(1)?,
          role: "user".into(),
          text: row.get(3)?,
          time: Time {
            created: row.get_u64(2)?,
            updated: 0,
          },
        })
      })
      .context("could not read OpenCode messages")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode messages")?;

    for message in messages {
      if let Some(&index) = session_indexes.get(&message.session_id) {
        sessions[index].push_message(message);
      }
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
      match directory {
        Some(directory) => {
          bail!("no OpenCode sessions found in {}", directory.display());
        }
        None => {
          bail!("no OpenCode sessions found in {}", self.database.display())
        }
      }
    }

    Ok(sessions)
  }

  pub(crate) fn validate_schema(&self) -> Result {
    let connection = self.open()?;
    let tables = {
      let mut statement = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table'")
        .context("could not inspect OpenCode schema")?;

      statement
        .query_map([], |row| row.get::<_, String>(0))
        .context("could not inspect OpenCode schema")?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .context("could not inspect OpenCode schema")?
    };

    let mut statement = connection
      .prepare("SELECT name FROM pragma_table_info(?)")
      .context("could not inspect OpenCode schema")?;
    let mut missing = Vec::new();

    for (table, required_columns) in REQUIRED_SCHEMA {
      if !tables.contains(*table) {
        missing.push(format!("table `{table}`"));
        continue;
      }

      let columns = statement
        .query_map([table], |row| row.get::<_, String>(0))
        .context("could not inspect OpenCode schema")?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .context("could not inspect OpenCode schema")?;

      for column in *required_columns {
        if !columns.contains(*column) {
          missing.push(format!("column `{table}.{column}`"));
        }
      }
    }

    if !missing.is_empty() {
      bail!(
        "unsupported OpenCode schema in {}: missing {}; update ocs or use \
         --database to select a compatible OpenCode database",
        self.database.display(),
        missing.join(", "),
      );
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn database() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("opencode.db");
    let connection = Connection::open(database).unwrap();

    connection
      .execute_batch(
        r#"
          CREATE TABLE session_v2 (
            id TEXT NOT NULL,
            directory TEXT NOT NULL,
            parent_id TEXT,
            slug TEXT NOT NULL,
            title TEXT,
            model TEXT,
            cost REAL NOT NULL,
            tokens_input INTEGER NOT NULL,
            tokens_output INTEGER NOT NULL,
            tokens_reasoning INTEGER NOT NULL,
            tokens_cache_read INTEGER NOT NULL,
            tokens_cache_write INTEGER NOT NULL,
            time_created INTEGER NOT NULL,
            time_updated INTEGER NOT NULL
          );

          CREATE TABLE session_message (
            id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            type TEXT NOT NULL,
            seq INTEGER NOT NULL,
            time_created INTEGER NOT NULL,
            data TEXT NOT NULL
          );

          INSERT INTO session_v2 VALUES (
            'ses_foo', '/tmp/foo', NULL, 'slug', 'Add picker',
            '{"id":"model-foo","providerID":"example"}',
            0.125, 1, 2, 3, 4, 5, 1, 2
          );

          INSERT INTO session_message VALUES (
            'msg_user', 'ses_foo', 'user', 1, 1,
            '{"text":"Build a picker"}'
          );

          INSERT INTO session_message VALUES (
            'msg_assistant', 'ses_foo', 'assistant', 2, 2,
            '{"content":[{"type":"text","text":"Use skim"}]}'
          );
        "#,
      )
      .unwrap();

    (directory, connection)
  }

  #[test]
  fn excludes_subagent_sessions() {
    let (temp, connection) = database();

    connection
      .execute(
        "INSERT INTO session_v2 VALUES ('ses_child', '/tmp/foo', 'ses_foo', 'child', 'Child', NULL, 0, 0, 0, 0, 0, 0, 3, 4)",
        [],
      )
      .unwrap();

    let sessions = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .sessions(None)
      .unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "ses_foo");
  }

  #[test]
  fn filters_sessions_by_directory() {
    let (temp, connection) = database();

    connection
      .execute(
        "INSERT INTO session_v2 VALUES ('ses_bar', '/tmp/bar', NULL, 'bar', 'Bar', NULL, 0, 0, 0, 0, 0, 0, 3, 4)",
        [],
      )
      .unwrap();

    let sessions = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .sessions(Some(Path::new("/tmp/bar")))
      .unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "ses_bar");
  }

  #[test]
  fn indexes_sqlite_sessions() {
    let (temp, _) = database();
    let storage = Storage::new(temp.path().join("opencode.db")).unwrap();

    storage.validate_schema().unwrap();

    let sessions = storage.sessions(None).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].model.as_deref(), Some("model-foo"));
    assert_eq!(sessions[0].tokens, 15);
    assert_eq!(
      sessions[0].search_text(),
      "Add picker\n/tmp/foo\nses_foo\nBuild a picker"
    );

    let session = storage.get_session("ses_foo").unwrap();

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].text, "Build a picker");
    assert_eq!(session.messages[1].text, "Use skim");
  }

  #[test]
  fn makes_relative_database_paths_absolute() {
    let storage = Storage::new("foo.db".into()).unwrap();

    assert_eq!(storage.database, env::current_dir().unwrap().join("foo.db"));
  }

  #[test]
  fn rejects_unsupported_schema() {
    let (temp, connection) = database();

    connection
      .execute_batch("ALTER TABLE session_v2 DROP COLUMN title")
      .unwrap();

    let database = temp.path().join("opencode.db");
    let error = Storage::new(database.clone())
      .unwrap()
      .validate_schema()
      .unwrap_err();

    assert_eq!(
      error.to_string(),
      format!(
        "unsupported OpenCode schema in {}: missing column \
         `session_v2.title`; update ocs or use --database to select a \
         compatible OpenCode database",
        database.display(),
      )
    );
  }
}
