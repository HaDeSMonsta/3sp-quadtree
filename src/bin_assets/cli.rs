use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::env;
use std::env::VarError;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LVL: &str = "debug";
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LVL: &str = "info";

#[derive(Debug, Parser)]
/// Generate quadtree and run queries I guess
pub struct Args {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
    #[clap(long, default_value_t = log_lvl())]
    /// Log level
    pub log_lvl: String,
    #[clap(short, long, default_value_t = 100)]
    /// Max items per level
    pub cap: usize,
    #[clap(short, long, default_value_t = 100)]
    /// Max levels per tree
    pub depth: usize,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Generate shell completions
    Complete {
        /// Which shell to generate completions for
        shell: Shell,
    },
}

fn log_lvl() -> String {
    env::var("LOG_LVL").unwrap_or_else(|e| {
        if e == VarError::NotPresent {
            String::from(DEFAULT_LOG_LVL)
        } else {
            panic!("failed to get log lvl env var: {e:?}");
        }
    })
}
