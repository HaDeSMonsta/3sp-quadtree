use clap::CommandFactory;
use clap_complete::Shell::*;
use color_eyre::Result;
use color_eyre::eyre::Context;
use std::fs;

const BIN_NAME: &str = env!("CARGO_PKG_NAME");
const COMPLETIONS_DIR: &str = "completions/";

include!("src/bin_assets/cli.rs");

fn main() -> Result<()> {
    color_eyre::install().context("failed to install :(")?;
    generate_completions().context("failed to generate completions")?;

    Ok(())
}

fn generate_completions() -> Result<()> {
    fs::create_dir_all(COMPLETIONS_DIR)
        .with_context(|| format!("failed to create completions dir {COMPLETIONS_DIR:?}"))?;

    for shell in [Bash, Fish, Zsh, Elvish, PowerShell] {
        let mut cmd = Args::command();
        clap_complete::generate_to(shell, &mut cmd, BIN_NAME, COMPLETIONS_DIR)
            .with_context(|| format!("failed to generate completions for shell \"{shell:?}\""))?;
    }
    Ok(())
}
