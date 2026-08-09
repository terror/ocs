use {
  anyhow::{Context, bail},
  arguments::Arguments,
  clap::Parser,
  config::Config,
  message::Message,
  ratatui::{
    style::{Color, Style},
    text::{Line, Span},
  },
  row_ext::RowExt,
  rusqlite::{Connection, OpenFlags, OptionalExtension},
  selection::Selection,
  serde::{Deserialize, Serialize},
  session::{Backend, Session},
  session_item::SessionItem,
  session_picker::SessionPicker,
  shell::Shell,
  skim::prelude::*,
  std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env,
    fmt::{self, Display, Formatter, Write},
    path::{Path, PathBuf},
    process::{self, Command},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
  },
  storage::Storage,
  style::{
    BOLD_BRIGHT_WHITE, BOLD_GRAY, BOLD_YELLOW, DARK_GRAY, DIM, DIM_LIGHT_GRAY,
    GRAY, style,
  },
  subcommand::Subcommand,
  time::Time,
};

mod arguments;
mod config;
mod message;
mod row_ext;
mod selection;
mod session;
mod session_item;
mod session_picker;
mod shell;
mod storage;
mod style;
mod subcommand;
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
