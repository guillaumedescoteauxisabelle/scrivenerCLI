use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
    name = "scriv",
    about = "Scrivener CLI (mirror-first, non-interactive)"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Project(ProjectCommands),
    #[command(subcommand)]
    Tree(TreeCommands),
    #[command(subcommand)]
    Doc(DocCommands),
    #[command(subcommand)]
    Meta(MetaCommands),
    #[command(subcommand)]
    Sync(SyncCommands),
    #[command(subcommand)]
    Conflict(ConflictCommands),
    #[command(subcommand)]
    Compile(CompileCommands),
    #[command(subcommand)]
    Git(GitCommands),
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    Create {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value = "blank")]
        template: String,
    },
    Info,
    Validate {
        #[arg(long)]
        strict: bool,
    },
    Doctor {
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum TreeCommands {
    Ls {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        recursive: bool,
    },
    Mkdir {
        #[arg(long)]
        path: String,
    },
    Mkdoc {
        #[arg(long)]
        path: String,
    },
    Mv {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    Rm {
        #[arg(long)]
        path: String,
        #[arg(long)]
        force: bool,
    },
    Reorder {
        #[arg(long)]
        path: String,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        after: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum DocCommands {
    Cat {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
    },
    Write {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        from_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
    },
    Append {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        from_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
    },
    Prepend {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        from_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
    },
    Edit {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        set_title: Option<String>,
        #[arg(long)]
        set_text: Option<String>,
        #[arg(long)]
        stdin: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum MetaCommands {
    #[command(subcommand)]
    Notes(MetaNotesCommands),
    #[command(subcommand)]
    Synopsis(MetaSynopsisCommands),
}

#[derive(Debug, Subcommand)]
pub enum MetaNotesCommands {
    Get {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
    },
    Set {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        from_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum MetaSynopsisCommands {
    Get {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
    },
    Set {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        stdin: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SyncCommands {
    Pull,
    Push,
    Status,
}

#[derive(Debug, Subcommand)]
pub enum ConflictCommands {
    Status,
    Resolve {
        #[arg(long)]
        id: Option<Uuid>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long = "use")]
        use_strategy: String,
        #[arg(long)]
        manual_file: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CompileCommands {
    Run {
        #[arg(long)]
        format: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        preset: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum GitCommands {
    Status {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Diff {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Add {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Commit {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Log {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Restore {
        #[arg(last = true)]
        args: Vec<String>,
    },
}
