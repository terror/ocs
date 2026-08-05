use super::*;

#[derive(clap::Args)]
pub(crate) struct Init {
  shell: Shell,
}

impl Init {
  pub(crate) fn run(self) {
    print!("{}", self.shell.init());
  }
}
