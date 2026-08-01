use super::*;

#[derive(Parser)]
#[command(about = "A fuzzy OpenCode session picker", version)]
pub(crate) struct Arguments {
  #[arg(long, help = "Show sessions from all directories")]
  pub(crate) all: bool,
  #[arg(long, value_name = "PATH", help = "OpenCode data directory")]
  pub(crate) data_dir: Option<PathBuf>,
  #[arg(
    long,
    value_name = "PATH",
    conflicts_with = "data_dir",
    help = "OpenCode database file"
  )]
  pub(crate) database: Option<PathBuf>,
  #[arg(long, help = "Print the selected session ID instead of opening it")]
  pub(crate) print: bool,
  #[arg(long, help = "Initial fuzzy-search query")]
  pub(crate) query: Option<String>,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    let storage = if let Some(database) = self.database {
      Storage::new(database)
    } else if let Some(data_dir) = self.data_dir {
      Storage::new(data_dir.join("opencode.db"))
    } else {
      Storage::default()?
    };

    storage.validate_schema()?;

    let directory = if self.all {
      None
    } else {
      Some(
        env::current_dir()
          .context("could not determine the current directory")?,
      )
    };

    let mut query = self.query;

    loop {
      let sessions = storage.sessions(directory.as_deref())?;

      let Some(selection) =
        SessionPicker::new(&storage, &sessions, query, self.all).pick()?
      else {
        return Ok(());
      };

      match selection {
        Selection::Delete {
          id,
          query: picker_query,
        } => {
          storage.delete_session(&id)?;

          if sessions.len() == 1 {
            return Ok(());
          }

          query = Some(picker_query);
        }
        Selection::Open(id) => {
          if self.print {
            println!("{id}");
            return Ok(());
          }

          let session = sessions
            .iter()
            .find(|session| session.id == id)
            .context("selected session was not indexed")?;

          return session.open();
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use clap::CommandFactory;

  #[test]
  fn database_and_data_dir_conflict() {
    let error = Arguments::command()
      .try_get_matches_from(["ocs", "--database", "foo", "--data-dir", "bar"])
      .err()
      .unwrap();

    assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
  }
}
