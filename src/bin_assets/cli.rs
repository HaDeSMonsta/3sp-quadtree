use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LVL: &str = "debug";
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LVL: &str = "info";

#[derive(Debug, Parser)]
/// Generate quadtree and run queries I guess
pub struct Args {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
    #[clap(long, default_value_t = String::from(DEFAULT_LOG_LVL))]
    /// Log level
    pub log_lvl: String,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Generate shell completions
    Complete {
        /// Which shell to generate completions for
        shell: Shell,
    },
}
