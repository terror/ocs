use {
  anyhow::{Context, bail},
  arguments::Arguments,
  clap::Parser,
  message::Message,
  ratatui::{
    style::{Color, Style},
    text::{Line, Span},
  },
  row_ext::RowExt,
  rusqlite::{Connection, OpenFlags, OptionalExtension},
  session::Session,
  session_item::SessionItem,
  session_picker::SessionPicker,
  skim::prelude::*,
  std::{
    borrow::Cow,
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::OnceLock,
  },
  storage::Storage,
  time::Time,
};

mod arguments;
mod message;
mod row_ext;
mod session;
mod session_item;
mod session_picker;
mod storage;
mod time;

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
