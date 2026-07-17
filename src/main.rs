use {
  anyhow::{Context, bail},
  arguments::Arguments,
  clap::Parser,
  message::Message,
  ratatui::text::Line,
  serde::Deserialize,
  session::{Session, SessionPicker},
  session_item::SessionItem,
  skim::prelude::*,
  std::{
    borrow::Cow,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
  },
  storage::Storage,
};

mod arguments;
mod message;
mod session;
mod session_item;
mod storage;

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
