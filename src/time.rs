use super::*;

#[derive(Default, Deserialize)]
pub(crate) struct Time {
  #[serde(default)]
  pub(crate) created: u64,
  #[serde(default)]
  pub(crate) updated: u64,
}
