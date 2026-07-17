pub(crate) enum Selection {
  Delete { id: String, query: String },
  Open(String),
}
