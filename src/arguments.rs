use super::*;

use {crate::session::SessionPicker, crate::storage::Storage};

#[derive(Parser)]
#[command(about = "A fuzzy OpenCode session picker")]
pub(crate) struct Arguments {
  #[arg(long, value_name = "PATH", help = "OpenCode data directory")]
  data_dir: Option<PathBuf>,
  #[arg(long, help = "Print the selected session ID instead of opening it")]
  print: bool,
  #[arg(long, help = "Initial fuzzy-search query")]
  query: Option<String>,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    let storage = match self.data_dir {
      Some(data_dir) => Storage::new(data_dir),
      None => Storage::default()?,
    };

    let sessions = storage.sessions()?;

    let selected = SessionPicker::new(&sessions, self.query).pick()?;

    let Some(id) = selected else {
      return Ok(());
    };

    if self.print {
      println!("{id}");
      return Ok(());
    }

    let session = sessions
      .iter()
      .find(|session| session.id() == id)
      .context("selected session was not indexed")?;

    session.open()
  }
}
