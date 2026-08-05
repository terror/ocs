#[derive(Clone, clap::ValueEnum)]
pub(crate) enum Shell {
  Zsh,
}

impl Shell {
  pub(crate) fn init(self) -> &'static str {
    match self {
      Self::Zsh => include_str!("shell/zsh/init.zsh"),
    }
  }
}
