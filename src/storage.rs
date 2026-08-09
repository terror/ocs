use super::*;

const REQUIRED_SCHEMA: &[(&str, &[&str])] = &[
  (
    "session",
    &[
      "id",
      "directory",
      "parent_id",
      "title",
      "time_created",
      "time_updated",
    ],
  ),
  ("message", &["id", "session_id", "time_created", "data"]),
  (
    "part",
    &["id", "message_id", "session_id", "time_created", "data"],
  ),
];

const REQUIRED_V2_SCHEMA: &[(&str, &[&str])] = &[
  (
    "session_v2",
    &[
      "id",
      "directory",
      "parent_id",
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
    if let Some(database) = Self::discover() {
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

    let data_directory = data_home.join("opencode");
    let v1_database = data_directory.join("opencode.db");
    let v2_database = data_directory.join("opencode-next.db");

    Self::new(if !v1_database.exists() && v2_database.exists() {
      v2_database
    } else {
      v1_database
    })
  }

  pub(crate) fn delete_session(&self, id: &str) -> Result {
    let session = self.get_session(id)?;
    let mut command = Command::new(match session.backend {
      Backend::V1 => "opencode",
      Backend::V2 => "opencode2",
    });

    match session.backend {
      Backend::V1 => command.args(["session", "delete", id]),
      Backend::V2 => {
        command.args(["api", "delete", &format!("/api/session/{id}")])
      }
    };

    let status = command
      .env("OPENCODE_DB", &self.database)
      .status()
      .context("could not start OpenCode")?;

    if !status.success() {
      bail!("OpenCode exited with {status}");
    }

    Ok(())
  }

  fn discover() -> Option<PathBuf> {
    if let Ok(output) = Command::new("opencode").args(["db", "path"]).output()
      && output.status.success()
      && let Ok(database) = String::from_utf8(output.stdout)
      && let database = database.trim()
      && !database.is_empty()
    {
      return Some(PathBuf::from(database));
    }

    env::var_os("OPENCODE_DB")
      .filter(|database| !database.is_empty())
      .map(PathBuf::from)
  }

  pub(crate) fn get_session(&self, id: &str) -> Result<Session> {
    let connection = Connection::open_with_flags(
      &self.database,
      OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .with_context(|| {
      format!(
        "could not open OpenCode database {}",
        self.database.display()
      )
    })?;

    if Self::has_table(&connection, "session_v2")?
      && let Some(session) = Self::get_v2_session(&connection, id)?
    {
      return Ok(session);
    }

    if !Self::has_table(&connection, "session")? {
      bail!("selected session was not indexed");
    }

    let session = connection
      .query_row(
        "SELECT id, directory, title, time_created, time_updated FROM session WHERE id = ?",
        [id],
        |row| {
          Ok(Session {
            directory: row.get(1)?,
            id: row.get(0)?,
            time: Time {
              created: row.get_u64(3)?,
              updated: row.get_u64(4)?,
            },
            title: row.get(2)?,
            ..Default::default()
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
            ORDER BY time_created, id
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
            ORDER BY time_created, id
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

  fn get_v2_session(
    connection: &Connection,
    id: &str,
  ) -> Result<Option<Session>> {
    let session = connection
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
            backend: Backend::V2,
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
      .context("could not query OpenCode V2 session")?;

    let Some(mut session) = session else {
      return Ok(None);
    };

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
      .context("could not query OpenCode V2 messages")?;

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
      .context("could not read OpenCode V2 messages")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode V2 messages")?;

    for message in messages {
      session.push_message(message);
    }

    Ok(Some(session))
  }

  fn has_table(connection: &Connection, table: &str) -> Result<bool> {
    connection
      .query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?)",
        [table],
        |row| row.get(0),
      )
      .context("could not inspect OpenCode schema")
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

  pub(crate) fn sessions(
    &self,
    directory: Option<&Path>,
  ) -> Result<Vec<Session>> {
    let connection = Connection::open_with_flags(
      &self.database,
      OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .with_context(|| {
      format!(
        "could not open OpenCode database {}",
        self.database.display()
      )
    })?;

    if !Self::has_table(&connection, "session")? {
      let mut sessions = Self::v2_sessions(&connection, directory)?;

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
            bail!("no OpenCode sessions found in {}", self.database.display());
          }
        }
      }

      return Ok(sessions);
    }

    let mut sessions = {
      let mut statement = connection
        .prepare(
          "
            SELECT id, directory, title, time_created, time_updated
            FROM session
            WHERE parent_id IS NULL
          ",
        )
        .context("could not query OpenCode sessions")?;

      statement
        .query_map([], |row| {
          Ok(Session {
            directory: row.get(1)?,
            id: row.get(0)?,
            time: Time {
              created: row.get_u64(3)?,
              updated: row.get_u64(4)?,
            },
            title: row.get(2)?,
            ..Default::default()
          })
        })
        .context("could not read OpenCode sessions")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode sessions")?
    };

    let usage = {
      let mut statement = connection
        .prepare(
          "
            WITH assistant_messages AS (
              SELECT
                session_id,
                COALESCE(json_extract(data, '$.modelID'), '') AS model,
                COALESCE(json_extract(data, '$.cost'), 0) AS cost,
                COALESCE(json_extract(data, '$.tokens.input'), 0)
                  + COALESCE(json_extract(data, '$.tokens.output'), 0)
                  + COALESCE(json_extract(data, '$.tokens.reasoning'), 0)
                  + COALESCE(json_extract(data, '$.tokens.cache.read'), 0)
                  + COALESCE(json_extract(data, '$.tokens.cache.write'), 0)
                  AS tokens,
                ROW_NUMBER() OVER (
                  PARTITION BY session_id
                  ORDER BY time_created DESC, id DESC
                ) AS position
              FROM message
              WHERE json_extract(data, '$.role') = 'assistant'
            )
            SELECT
              session_id,
              MAX(CASE WHEN position = 1 THEN model END),
              TOTAL(cost),
              SUM(tokens)
            FROM assistant_messages
            GROUP BY session_id
          ",
        )
        .context("could not query OpenCode usage")?;

      statement
        .query_map([], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get_u64(3)?,
          ))
        })
        .context("could not read OpenCode usage")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("could not read OpenCode usage")?
    };

    if let Some(directory) = directory {
      sessions.retain(|session| Path::new(&session.directory) == directory);
    }

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
                  ORDER BY time_created DESC, id DESC
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
                  ORDER BY time_created DESC, id DESC
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
            ORDER BY part.time_created, part.id
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

    for (session_id, model, cost, tokens) in usage {
      if let Some(&index) = session_indexes.get(&session_id) {
        sessions[index].cost = cost;
        sessions[index].model = (!model.is_empty()).then_some(model);
        sessions[index].tokens = tokens;
      }
    }

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

    if Self::has_table(&connection, "session_v2")? {
      let v2_sessions = Self::v2_sessions(&connection, directory)?;
      let v2_ids = v2_sessions
        .iter()
        .map(|session| session.id.as_str())
        .collect::<HashSet<_>>();

      sessions.retain(|session| !v2_ids.contains(session.id.as_str()));
      sessions.extend(v2_sessions);
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

  fn v2_sessions(
    connection: &Connection,
    directory: Option<&Path>,
  ) -> Result<Vec<Session>> {
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
      .context("could not query OpenCode V2 sessions")?;

    let mut sessions = statement
      .query_map([], |row| {
        let model = row.get::<_, String>(6)?;

        Ok(Session {
          backend: Backend::V2,
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
      .context("could not read OpenCode V2 sessions")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode V2 sessions")?;

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
      .context("could not query OpenCode V2 messages")?;

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
      .context("could not read OpenCode V2 messages")?
      .collect::<rusqlite::Result<Vec<_>>>()
      .context("could not read OpenCode V2 messages")?;

    for message in messages {
      if let Some(&index) = session_indexes.get(&message.session_id) {
        sessions[index].push_message(message);
      }
    }

    for session in &mut sessions {
      session.sort_messages();
    }

    Ok(sessions)
  }

  pub(crate) fn validate_schema(&self) -> Result {
    let connection = Connection::open_with_flags(
      &self.database,
      OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .with_context(|| {
      format!(
        "could not open OpenCode database {}",
        self.database.display()
      )
    })?;

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

    let mut schemas = Vec::new();

    if tables.contains("session") {
      schemas.push(REQUIRED_SCHEMA);
    }

    if tables.contains("session_v2") {
      schemas.push(REQUIRED_V2_SCHEMA);
    }

    if schemas.is_empty() {
      bail!(
        "unsupported OpenCode schema in {}: missing table `session` or table \
         `session_v2`; update ocs or use --database to select a compatible \
         OpenCode database",
        self.database.display(),
      );
    }

    for (table, required_columns) in schemas.into_iter().flatten() {
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
          CREATE TABLE session (
            id TEXT NOT NULL,
            directory TEXT NOT NULL,
            parent_id TEXT,
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
            id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            time_created INTEGER NOT NULL,
            data TEXT NOT NULL
          );

          INSERT INTO session
            VALUES ('ses_foo', '/tmp/foo', NULL, 'Add picker', 1, 2);

          INSERT INTO message
            VALUES (
              'msg_one',
              'ses_foo',
              2,
              '{"role":"assistant","modelID":"model-foo","cost":0.125,"tokens":{"input":1,"output":2,"reasoning":3,"cache":{"read":4,"write":5}}}'
            );

          INSERT INTO message
            VALUES ('msg_two', 'ses_foo', 1, '{"role":"user"}');

          INSERT INTO message
            VALUES (
              'msg_three',
              'ses_foo',
              3,
              '{"role":"assistant","modelID":"model-bar","cost":0.25,"tokens":{"input":5}}'
            );

          INSERT INTO part
            VALUES (
              'prt_one',
              'msg_one',
              'ses_foo',
              2,
              '{"type":"text","text":"Use skim"}'
            );

          INSERT INTO part
            VALUES (
              'prt_two',
              'msg_two',
              'ses_foo',
              1,
              '{"type":"text","text":"Build a picker"}'
            );
        "#,
      )
      .unwrap();

    (directory, connection)
  }

  fn create_v2_schema(connection: &Connection) {
    connection
      .execute_batch(
        r"
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
        ",
      )
      .unwrap();
  }

  #[test]
  fn excludes_subagent_sessions() {
    let (temp, connection) = database();

    connection
      .execute(
        "INSERT INTO session VALUES ('ses_baz', '/tmp/foo', 'ses_foo', 'Baz', 3, 4)",
        [],
      )
      .unwrap();

    let sessions = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .sessions(None)
      .unwrap();

    assert_eq!(
      sessions,
      vec![Session {
        backend: Backend::V1,
        cost: 0.375,
        directory: "/tmp/foo".into(),
        id: "ses_foo".into(),
        messages: vec![Message {
          id: "msg_two".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "Build a picker".into(),
          time: Time {
            created: 1,
            updated: 0,
          },
        }],
        model: Some("model-bar".into()),
        time: Time {
          created: 1,
          updated: 2,
        },
        title: "Add picker".into(),
        tokens: 20,
      }]
    );
  }

  #[test]
  fn filters_sessions_by_directory() {
    let (temp, connection) = database();

    connection
      .execute(
        "INSERT INTO session VALUES ('ses_bar', '/tmp/bar', NULL, 'Bar', 3, 4)",
        [],
      )
      .unwrap();

    let sessions = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .sessions(Some(Path::new("/tmp/bar")))
      .unwrap();

    assert_eq!(
      sessions,
      vec![Session {
        directory: "/tmp/bar".into(),
        id: "ses_bar".into(),
        time: Time {
          created: 3,
          updated: 4,
        },
        title: "Bar".into(),
        ..Default::default()
      }]
    );
  }

  #[test]
  fn indexes_v2_sessions() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("opencode.db");
    let connection = Connection::open(&database).unwrap();

    create_v2_schema(&connection);

    connection
      .execute_batch(
        r#"
          INSERT INTO session_v2 VALUES (
            'ses_v2',
            '/tmp/foo',
            NULL,
            'v2-slug',
            'V2 session',
            '{"id":"model-v2","providerID":"example"}',
            0.5,
            1,
            2,
            3,
            4,
            5,
            10,
            20
          );

          INSERT INTO session_message VALUES (
            'msg_user',
            'ses_v2',
            'user',
            1,
            11,
            '{"text":"Build V2 support"}'
          );

          INSERT INTO session_message VALUES (
            'msg_assistant',
            'ses_v2',
            'assistant',
            2,
            12,
            '{"content":[{"type":"text","text":"Use session_v2"}]}'
          );
        "#,
      )
      .unwrap();

    let storage = Storage::new(database).unwrap();

    storage.validate_schema().unwrap();

    let sessions = storage.sessions(None).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].backend, Backend::V2);
    assert_eq!(sessions[0].model.as_deref(), Some("model-v2"));
    assert_eq!(sessions[0].tokens, 15);
    assert_eq!(
      sessions[0].search_text(),
      "V2 session\n/tmp/foo\nses_v2\nBuild V2 support"
    );

    let session = storage.get_session("ses_v2").unwrap();

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].text, "Build V2 support");
    assert_eq!(session.messages[1].text, "Use session_v2");
  }

  #[test]
  fn prefers_migrated_v2_sessions() {
    let (temp, connection) = database();

    create_v2_schema(&connection);

    connection
      .execute(
        "
          INSERT INTO session_v2 VALUES (
            'ses_foo', '/tmp/foo', NULL, 'slug', 'Migrated', NULL,
            0, 0, 0, 0, 0, 0, 1, 3
          )
        ",
        [],
      )
      .unwrap();

    let sessions = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .sessions(None)
      .unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].backend, Backend::V2);
    assert_eq!(sessions[0].title, "Migrated");
  }

  #[test]
  fn indexes_sqlite_sessions() {
    let (temp, _) = database();

    let storage = Storage::new(temp.path().join("opencode.db")).unwrap();

    storage.validate_schema().unwrap();

    let sessions = storage.sessions(None).unwrap();

    assert_eq!(
      sessions,
      vec![Session {
        backend: Backend::V1,
        cost: 0.375,
        directory: "/tmp/foo".into(),
        id: "ses_foo".into(),
        messages: vec![Message {
          id: "msg_two".into(),
          role: "user".into(),
          session_id: "ses_foo".into(),
          text: "Build a picker".into(),
          time: Time {
            created: 1,
            updated: 0,
          },
        }],
        model: Some("model-bar".into()),
        time: Time {
          created: 1,
          updated: 2,
        },
        title: "Add picker".into(),
        tokens: 20,
      }]
    );

    assert_eq!(
      sessions.first().unwrap().search_text(),
      "Add picker\n/tmp/foo\nses_foo\nBuild a picker"
    );

    let session = storage.get_session("ses_foo").unwrap();

    assert_eq!(
      session.preview(),
      format!(
        "{}\n{}  {}\n{}    {}\n\n{}\nBuild a picker\n\n{}\nUse skim",
        style(BOLD_BRIGHT_WHITE, "Add picker"),
        style(GRAY, "Directory"),
        style(DIM_LIGHT_GRAY, "/tmp/foo"),
        style(GRAY, "Session"),
        style(DIM_LIGHT_GRAY, "ses_foo"),
        style(BOLD_YELLOW, "USER"),
        style(BOLD_BRIGHT_WHITE, "ASSISTANT"),
      )
    );
  }

  #[test]
  fn makes_relative_database_paths_absolute() {
    let storage = Storage::new("foo.db".into()).unwrap();

    assert_eq!(storage.database, env::current_dir().unwrap().join("foo.db"));
  }

  #[test]
  fn orders_transcript_by_time_and_id() {
    let (temp, connection) = database();

    connection
      .execute_batch(
        r#"
          INSERT INTO session VALUES ('ses_bar', '/tmp/bar', NULL, 'Bar', 3, 4);
          INSERT INTO message VALUES ('msg_foo', 'ses_bar', 5, '{"role":"assistant"}');
          INSERT INTO message VALUES ('msg_bar', 'ses_bar', 5, '{"role":"user"}');
          INSERT INTO part VALUES ('prt_foo', 'msg_bar', 'ses_bar', 6, '{"type":"text","text":"foo"}');
          INSERT INTO part VALUES ('prt_bar', 'msg_bar', 'ses_bar', 6, '{"type":"text","text":"bar"}');
        "#,
      )
      .unwrap();

    let session = Storage::new(temp.path().join("opencode.db"))
      .unwrap()
      .get_session("ses_bar")
      .unwrap();

    assert_eq!(
      session.messages,
      vec![
        Message {
          id: "msg_bar".into(),
          role: "user".into(),
          session_id: "ses_bar".into(),
          text: "bar\nfoo".into(),
          time: Time {
            created: 5,
            updated: 0,
          },
        },
        Message {
          id: "msg_foo".into(),
          role: "assistant".into(),
          session_id: "ses_bar".into(),
          text: String::new(),
          time: Time {
            created: 5,
            updated: 0,
          },
        },
      ]
    );
  }

  #[test]
  fn rejects_unsupported_schema() {
    #[track_caller]
    fn case(change: &str, missing: &str) {
      let (temp, connection) = database();

      connection.execute_batch(change).unwrap();

      let database = temp.path().join("opencode.db");

      let error = Storage::new(database.clone())
        .unwrap()
        .validate_schema()
        .unwrap_err();

      assert_eq!(
        error.to_string(),
        format!(
          "unsupported OpenCode schema in {}: missing {missing}; update ocs or \
           use --database to select a compatible OpenCode database",
          database.display(),
        )
      );
    }

    case("DROP TABLE part", "table `part`");

    case(
      "ALTER TABLE session DROP COLUMN title",
      "column `session.title`",
    );
  }
}
