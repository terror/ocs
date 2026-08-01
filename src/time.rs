#[derive(Default)]
pub(crate) struct Time {
  pub(crate) created: u64,
  pub(crate) updated: u64,
}

impl Time {
  pub(crate) fn relative_updated(&self, now: u64) -> String {
    let seconds = now.saturating_sub(self.updated.max(self.created)) / 1_000;

    let (value, unit) = match seconds {
      0..60 => return "now".into(),
      60..3_600 => (seconds / 60, "m"),
      3_600..86_400 => (seconds / 3_600, "h"),
      86_400..604_800 => (seconds / 86_400, "d"),
      604_800..2_629_800 => (seconds / 604_800, "w"),
      2_629_800..31_557_600 => (seconds / 2_629_800, "mo"),
      _ => (seconds / 31_557_600, "y"),
    };

    format!("{value}{unit}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_relative_updated_time() {
    #[track_caller]
    fn case(elapsed: u64, expected: &str) {
      assert_eq!(
        Time {
          created: 0,
          updated: 1_000,
        }
        .relative_updated(1_000 + elapsed),
        expected,
      );
    }

    case(0, "now");
    case(59_000, "now");
    case(60_000, "1m");
    case(7_200_000, "2h");
    case(172_800_000, "2d");
    case(1_209_600_000, "2w");
    case(5_259_600_000, "2mo");
    case(63_115_200_000, "2y");
  }
}
