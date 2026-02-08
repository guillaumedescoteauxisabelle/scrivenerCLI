use std::process::Command;

use anyhow::Result;

use crate::cli::GitCommands;
use crate::types::ProjectHandle;

pub fn run(handle: &ProjectHandle, command: GitCommands) -> Result<i32> {
    let (verb, args) = match command {
        GitCommands::Status { args } => ("status", args),
        GitCommands::Diff { args } => ("diff", args),
        GitCommands::Add { args } => ("add", args),
        GitCommands::Commit { args } => ("commit", args),
        GitCommands::Log { args } => ("log", args),
        GitCommands::Restore { args } => ("restore", args),
    };

    let status = Command::new("git")
        .current_dir(&handle.root_dir)
        .arg(verb)
        .args(args)
        .status()?;

    Ok(status.code().unwrap_or(1))
}
