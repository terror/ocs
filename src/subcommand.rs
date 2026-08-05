use {super::*, init::Init};

mod init;

#[derive(clap::Subcommand)]
pub(crate) enum Subcommand {
  #[command(about = "Generate shell integration")]
  Init(Init),
}

impl Subcommand {
  pub(crate) fn run(self) {
    match self {
      Self::Init(init) => init.run(),
    }
  }
}
