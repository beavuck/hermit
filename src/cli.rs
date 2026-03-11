use crate::constants::{DEFAULT_MAX_ITEMS, DEFAULT_MIN_ITEMS, DEFAULT_PORT};

#[derive(clap::Parser)]
pub struct Args {
    #[arg(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// One or more OpenAPI spec files to load. Can be specified multiple times or as a space-separated list.
    #[arg(long, num_args = 1..)]
    pub specs: Vec<std::path::PathBuf>,

    /// The minimum number of items to generate for array schemas. Must not be greater than `--max-items`.
    #[arg(long, default_value_t = DEFAULT_MIN_ITEMS)]
    pub min_items: usize,

    /// The maximum number of items to generate for array schemas. Must not be less than `--min-items`.
    #[arg(long, default_value_t = DEFAULT_MAX_ITEMS)]
    pub max_items: usize,

    /// Skip `example` values from the spec and generate random data instead
    #[arg(long, default_value_t = false)]
    pub ignore_examples: bool,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_items > self.max_items {
            return Err(format!(
                "--min-items ({}) must not exceed --max-items ({})",
                self.min_items, self.max_items
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Args;
    use crate::constants::{DEFAULT_MAX_ITEMS, DEFAULT_MIN_ITEMS};

    #[test]
    fn min_items_and_max_items_have_correct_defaults() {
        let args = Args::try_parse_from(["hermit", "--specs", "a.yaml"]).unwrap();
        assert_eq!(args.min_items, DEFAULT_MIN_ITEMS);
        assert_eq!(args.max_items, DEFAULT_MAX_ITEMS);
    }

    #[test]
    fn min_items_and_max_items_can_be_configured_via_cli() {
        let args = Args::try_parse_from([
            "hermit",
            "--specs",
            "a.yaml",
            "--min-items",
            "5",
            "--max-items",
            "10",
        ])
        .unwrap();
        assert_eq!(args.min_items, 5);
        assert_eq!(args.max_items, 10);
    }

    #[test]
    fn multiple_specs_are_accepted() {
        let args =
            Args::try_parse_from(["hermit", "--specs", "a.yaml", "b.yaml", "c.yaml"]).unwrap();
        assert_eq!(args.specs.len(), 3);
    }

    #[test]
    fn validate_rejects_min_greater_than_default_max() {
        let min = (DEFAULT_MAX_ITEMS + 1).to_string();
        let args =
            Args::try_parse_from(["hermit", "--specs", "a.yaml", "--min-items", &min]).unwrap();
        assert!(args.validate().is_err());
    }

    #[test]
    fn validate_rejects_max_less_than_default_min() {
        let max = (DEFAULT_MIN_ITEMS - 1).to_string();
        let args =
            Args::try_parse_from(["hermit", "--specs", "a.yaml", "--max-items", &max]).unwrap();
        assert!(args.validate().is_err());
    }

    #[test]
    fn validate_rejects_min_greater_than_max() {
        let args = Args::try_parse_from([
            "hermit",
            "--specs",
            "a.yaml",
            "--min-items",
            "10",
            "--max-items",
            "3",
        ])
        .unwrap();
        assert!(args.validate().is_err());
    }

    #[test]
    fn validate_accepts_equal_min_and_max() {
        let args = Args::try_parse_from([
            "hermit",
            "--specs",
            "a.yaml",
            "--min-items",
            "5",
            "--max-items",
            "5",
        ])
        .unwrap();
        assert!(args.validate().is_ok());
    }

    #[test]
    fn validate_accepts_max_below_default_min_when_min_is_also_set_lower() {
        let val = (DEFAULT_MIN_ITEMS - 1).to_string();
        let args = Args::try_parse_from([
            "hermit",
            "--specs",
            "a.yaml",
            "--min-items",
            &val,
            "--max-items",
            &val,
        ])
        .unwrap();
        assert!(args.validate().is_ok());
    }

    #[test]
    fn ignore_examples_defaults_to_false() {
        let args = Args::try_parse_from(["hermit", "--specs", "a.yaml"]).unwrap();
        assert!(!args.ignore_examples);
    }

    #[test]
    fn ignore_examples_flag_can_be_set() {
        let args =
            Args::try_parse_from(["hermit", "--specs", "a.yaml", "--ignore-examples"]).unwrap();
        assert!(args.ignore_examples);
    }

    #[test]
    fn validate_accepts_min_above_default_max_when_max_is_also_set_higher() {
        let min = (DEFAULT_MAX_ITEMS + 1).to_string();
        let max = (DEFAULT_MAX_ITEMS + 2).to_string();
        let args = Args::try_parse_from([
            "hermit",
            "--specs",
            "a.yaml",
            "--min-items",
            &min,
            "--max-items",
            &max,
        ])
        .unwrap();
        assert!(args.validate().is_ok());
    }
}
