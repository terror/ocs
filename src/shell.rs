#[derive(Clone, clap::ValueEnum)]
pub(crate) enum Shell {
  Bash,
  Zsh,
}

impl Shell {
  pub(crate) fn init(self) -> &'static str {
    match self {
      Self::Bash => include_str!("shell/bash/init.bash"),
      Self::Zsh => include_str!("shell/zsh/init.zsh"),
    }
  }
}
