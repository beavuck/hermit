use crate::constants::{DEFAULT_PORT, DEFAULT_SPEC_GLOB};

#[derive(clap::Parser)]
pub struct Args {
    #[arg(long, default_value = DEFAULT_SPEC_GLOB)]
    pub specs: std::path::PathBuf,

    #[arg(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
}
