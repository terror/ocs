use super::*;

#[derive(Parser)]
#[command(about = "A fuzzy OpenCode session picker")]
pub(crate) struct Arguments {
  #[arg(long, help = "Only show sessions from the current directory")]
  pub(crate) cwd: bool,
  #[arg(long, value_name = "PATH", help = "OpenCode data directory")]
  pub(crate) data_dir: Option<PathBuf>,
  #[arg(long, help = "Print the selected session ID instead of opening it")]
  pub(crate) print: bool,
  #[arg(long, help = "Initial fuzzy-search query")]
  pub(crate) query: Option<String>,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    let storage = match self.data_dir {
      Some(data_dir) => Storage::new(data_dir),
      None => Storage::default()?,
    };

    let directory = self
      .cwd
      .then(env::current_dir)
      .transpose()
      .context("could not determine the current directory")?;

    let mut query = self.query;

    loop {
      let sessions = storage.sessions(directory.as_deref())?;

      let Some(selection) =
        SessionPicker::new(&storage, &sessions, query).pick()?
      else {
        return Ok(());
      };

      match selection {
        Selection::Delete {
          id,
          query: picker_query,
        } => {
          storage.delete(&id)?;

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
