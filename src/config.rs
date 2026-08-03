use super::*;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Config {
  pub(crate) opencode_args: Vec<String>,
}

impl Config {
  pub(crate) fn load() -> Result<Self> {
    confy::load("ocs", Some("config")).context("could not load configuration")
  }
}
