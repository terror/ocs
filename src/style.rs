use super::*;

pub(crate) const BOLD_BRIGHT_WHITE: &str = "1;38;5;255";
pub(crate) const BOLD_GRAY: &str = "1;38;5;244";
pub(crate) const BOLD_YELLOW: &str = "1;38;5;230";
pub(crate) const DIM: &str = "2";
pub(crate) const DIM_LIGHT_GRAY: &str = "2;38;5;248";
pub(crate) const GRAY: &str = "38;5;244";
pub(crate) const DARK_GRAY: Color = Color::DarkGray;

pub(crate) fn style<T: Display>(code: &'static str, value: T) -> Styled<T> {
  Styled { code, value }
}

pub(crate) struct Styled<T> {
  code: &'static str,
  value: T,
}

impl<T: Display> Display for Styled<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "\x1b[{}m{}\x1b[0m", self.code, self.value)
  }
}
