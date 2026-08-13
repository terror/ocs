use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct Message {
  pub(crate) id: String,
  pub(crate) role: String,
  pub(crate) session_id: String,
  pub(crate) text: String,
  pub(crate) time: Time,
}
