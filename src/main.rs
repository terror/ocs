mod arguments;
mod session;
mod storage;

use {
  anyhow::{Context, bail},
  clap::Parser,
  serde::Deserialize,
  skim::prelude::*,
  std::{
    borrow::Cow,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
  },
};

use arguments::Arguments;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

fn main() {
  if let Err(error) = Arguments::parse().run() {
    eprintln!("error: {error}");

    for cause in error.chain().skip(1) {
      eprintln!("because: {cause}");
    }

    process::exit(1);
  }
}
