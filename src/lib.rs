pub mod binder;
pub mod cli;
pub mod compile;
pub mod conflict;
pub mod gitwrap;
pub mod io;
pub mod mirror;
pub mod project;
pub mod sync;
pub mod types;

use anyhow::Result;
use std::io::Write;

use crate::cli::{Cli, Commands};
use crate::types::{DocKind, ExitCode, JsonEnvelope, ResolveTarget};

pub fn run(cli: Cli) -> Result<i32> {
    let json = cli.json;

    if let Commands::Project(cli::ProjectCommands::Create {
        name,
        dir,
        template,
    }) = &cli.command
    {
        project::create_project(name, dir.clone(), template)?;
        print_out(json, "project created", serde_json::json!({ "name": name }));
        return Ok(ExitCode::Success.as_i32());
    }

    let project_handle = project::open_project(cli.project.as_deref())?;

    let exit = match cli.command {
        Commands::Project(args) => match args {
            cli::ProjectCommands::Create { .. } => ExitCode::Success,
            cli::ProjectCommands::Info => {
                let data = project::load_project_data(&project_handle)?;
                print_out(
                    json,
                    &format!(
                        "project: {} ({})",
                        data.title,
                        project_handle.scriv_dir.display()
                    ),
                    serde_json::to_value(data)?,
                );
                ExitCode::Success
            }
            cli::ProjectCommands::Validate { strict } => {
                project::validate_project(&project_handle, strict)?;
                print_out(
                    json,
                    "project valid",
                    serde_json::json!({ "strict": strict }),
                );
                ExitCode::Success
            }
            cli::ProjectCommands::Doctor { check } => {
                let issues = project::doctor_project(&project_handle, check)?;
                let msg = if issues.is_empty() {
                    "no issues"
                } else {
                    "issues found"
                };
                print_out(json, msg, serde_json::to_value(issues)?);
                if msg == "no issues" {
                    ExitCode::Success
                } else {
                    ExitCode::ValidationFailure
                }
            }
        },
        Commands::Tree(args) => {
            let mut data = project::load_project_data(&project_handle)?;
            match args {
                cli::TreeCommands::Ls { path, recursive } => {
                    let out = binder::list(&data, path.as_deref(), recursive)?;
                    print_out(json, &out.join("\n"), serde_json::to_value(out)?);
                    ExitCode::Success
                }
                cli::TreeCommands::Mkdir { path } => {
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::mkdir(d, &path)
                    })?;
                    print_out(json, "folder created", serde_json::json!({ "path": path }));
                    ExitCode::Success
                }
                cli::TreeCommands::Mkdoc { path } => {
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::mkdoc(d, &path)
                    })?;
                    print_out(
                        json,
                        "document created",
                        serde_json::json!({ "path": path }),
                    );
                    ExitCode::Success
                }
                cli::TreeCommands::Mv { from, to } => {
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::mv(d, &from, &to)
                    })?;
                    print_out(
                        json,
                        "node moved",
                        serde_json::json!({ "from": from, "to": to }),
                    );
                    ExitCode::Success
                }
                cli::TreeCommands::Rm { path, force } => {
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::rm(d, &path, force)
                    })?;
                    print_out(json, "node removed", serde_json::json!({ "path": path }));
                    ExitCode::Success
                }
                cli::TreeCommands::Reorder {
                    path,
                    before,
                    after,
                } => {
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::reorder(d, &path, before.as_deref(), after.as_deref())
                    })?;
                    print_out(json, "node reordered", serde_json::json!({ "path": path }));
                    ExitCode::Success
                }
            }
        }
        Commands::Doc(args) => {
            let mut data = project::load_project_data(&project_handle)?;
            match args {
                cli::DocCommands::Cat { id, path } => {
                    let target = ResolveTarget::new(id, path)?;
                    let doc = binder::resolve_doc(&data, &target)?;
                    print_out(
                        json,
                        &doc.content,
                        serde_json::json!({ "id": doc.id, "content": doc.content }),
                    );
                    ExitCode::Success
                }
                cli::DocCommands::Write {
                    id,
                    path,
                    from_file,
                    stdin,
                } => {
                    let target = ResolveTarget::new(id, path)?;
                    let content = io::read_input(from_file.as_deref(), stdin)?;
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::set_content(d, &target, &content)
                    })?;
                    verify_doc_content(&project_handle, &target, |actual| actual == content)?;
                    print_out(
                        json,
                        "document written",
                        serde_json::json!({ "bytes": content.len() }),
                    );
                    ExitCode::Success
                }
                cli::DocCommands::Append {
                    id,
                    path,
                    from_file,
                    stdin,
                } => {
                    let target = ResolveTarget::new(id, path)?;
                    let content = io::read_input(from_file.as_deref(), stdin)?;
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::append_content(d, &target, &content)
                    })?;
                    verify_doc_content(&project_handle, &target, |actual| {
                        actual.ends_with(&content)
                    })?;
                    print_out(
                        json,
                        "document appended",
                        serde_json::json!({ "bytes": content.len() }),
                    );
                    ExitCode::Success
                }
                cli::DocCommands::Prepend {
                    id,
                    path,
                    from_file,
                    stdin,
                } => {
                    let target = ResolveTarget::new(id, path)?;
                    let content = io::read_input(from_file.as_deref(), stdin)?;
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::prepend_content(d, &target, &content)
                    })?;
                    verify_doc_content(&project_handle, &target, |actual| {
                        actual.starts_with(&content)
                    })?;
                    print_out(
                        json,
                        "document prepended",
                        serde_json::json!({ "bytes": content.len() }),
                    );
                    ExitCode::Success
                }
                cli::DocCommands::Edit {
                    id,
                    path,
                    set_title,
                    set_text,
                    stdin,
                } => {
                    let target = ResolveTarget::new(id, path)?;
                    let text = if let Some(text) = set_text {
                        Some(text)
                    } else if stdin {
                        Some(io::read_input(None, true)?)
                    } else {
                        None
                    };
                    sync::with_write_through(&project_handle, &mut data, |d| {
                        binder::edit_doc(d, &target, set_title.as_deref(), text.as_deref())
                    })?;
                    if let Some(expected) = text.as_deref() {
                        verify_doc_content(&project_handle, &target, |actual| actual == expected)?;
                    }
                    print_out(json, "document edited", serde_json::json!({}));
                    ExitCode::Success
                }
            }
        }
        Commands::Meta(args) => {
            let mut data = project::load_project_data(&project_handle)?;
            match args {
                cli::MetaCommands::Notes(sub) => match sub {
                    cli::MetaNotesCommands::Get { id, path } => {
                        let target = ResolveTarget::new(id, path)?;
                        let doc = binder::resolve_doc(&data, &target)?;
                        print_out(
                            json,
                            &doc.meta.notes,
                            serde_json::json!({ "notes": doc.meta.notes }),
                        );
                        ExitCode::Success
                    }
                    cli::MetaNotesCommands::Set {
                        id,
                        path,
                        from_file,
                        stdin,
                    } => {
                        let target = ResolveTarget::new(id, path)?;
                        let notes = io::read_input(from_file.as_deref(), stdin)?;
                        sync::with_write_through(&project_handle, &mut data, |d| {
                            binder::set_notes(d, &target, &notes)
                        })?;
                        print_out(json, "notes updated", serde_json::json!({}));
                        ExitCode::Success
                    }
                },
                cli::MetaCommands::Synopsis(sub) => match sub {
                    cli::MetaSynopsisCommands::Get { id, path } => {
                        let target = ResolveTarget::new(id, path)?;
                        let doc = binder::resolve_doc(&data, &target)?;
                        print_out(
                            json,
                            &doc.meta.synopsis,
                            serde_json::json!({ "synopsis": doc.meta.synopsis }),
                        );
                        ExitCode::Success
                    }
                    cli::MetaSynopsisCommands::Set {
                        id,
                        path,
                        text,
                        stdin,
                    } => {
                        let target = ResolveTarget::new(id, path)?;
                        let synopsis = if let Some(text) = text {
                            text
                        } else {
                            io::read_input(None, stdin)?
                        };
                        sync::with_write_through(&project_handle, &mut data, |d| {
                            binder::set_synopsis(d, &target, &synopsis)
                        })?;
                        print_out(json, "synopsis updated", serde_json::json!({}));
                        ExitCode::Success
                    }
                },
            }
        }
        Commands::Sync(args) => match args {
            cli::SyncCommands::Pull => {
                let data = project::load_project_data(&project_handle)?;
                sync::pull(&project_handle, &data)?;
                print_out(json, "sync pull complete", serde_json::json!({}));
                ExitCode::Success
            }
            cli::SyncCommands::Push => {
                let mut data = project::load_project_data(&project_handle)?;
                match sync::push(&project_handle, &mut data) {
                    Ok(()) => {
                        print_out(json, "sync push complete", serde_json::json!({}));
                        ExitCode::Success
                    }
                    Err(sync::SyncError::Conflict(conflicts)) => {
                        print_out(json, "conflicts detected", serde_json::to_value(conflicts)?);
                        ExitCode::Conflict
                    }
                    Err(sync::SyncError::Other(err)) => return Err(err),
                }
            }
            cli::SyncCommands::Status => {
                let status = sync::status(&project_handle)?;
                let summary = status.summary.clone();
                print_out(json, &summary, serde_json::to_value(status)?);
                ExitCode::Success
            }
        },
        Commands::Conflict(args) => match args {
            cli::ConflictCommands::Status => {
                let conflicts = conflict::status(&project_handle)?;
                let msg = if conflicts.is_empty() {
                    "no conflicts"
                } else {
                    "conflicts present"
                };
                print_out(json, msg, serde_json::to_value(conflicts)?);
                ExitCode::Success
            }
            cli::ConflictCommands::Resolve {
                id,
                path,
                use_strategy,
                manual_file,
            } => {
                let target = ResolveTarget::new(id, path)?;
                conflict::resolve(
                    &project_handle,
                    &target,
                    &use_strategy,
                    manual_file.as_deref(),
                )?;
                print_out(
                    json,
                    "conflict resolved",
                    serde_json::json!({ "strategy": use_strategy }),
                );
                ExitCode::Success
            }
        },
        Commands::Compile(args) => match args {
            cli::CompileCommands::Run {
                format,
                output,
                preset,
            } => {
                let data = project::load_project_data(&project_handle)?;
                match compile::run(&project_handle, &data, &format, &output, preset.as_deref())? {
                    compile::CompileOutcome::BuiltIn => {
                        print_out(
                            json,
                            "compile complete",
                            serde_json::json!({ "format": format }),
                        );
                        ExitCode::Success
                    }
                    compile::CompileOutcome::AppBridgeUnsupported => {
                        print_out(
                            json,
                            "app compile unsupported on this platform",
                            serde_json::json!({ "format": format }),
                        );
                        ExitCode::CompileFailed
                    }
                }
            }
        },
        Commands::Git(args) => {
            let code = gitwrap::run(&project_handle, args)?;
            ExitCode::from_i32(code)
        }
    };

    Ok(exit.as_i32())
}

fn print_out(json: bool, message: &str, payload: serde_json::Value) {
    let line = if json {
        let envelope = JsonEnvelope {
            ok: true,
            message: message.to_string(),
            data: payload,
        };
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string())
    } else {
        message.to_string()
    };

    if !line.is_empty() {
        let mut out = std::io::stdout().lock();
        if let Err(err) = writeln!(out, "{line}") {
            if err.kind() == std::io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
            eprintln!("output error: {err}");
            std::process::exit(1);
        }
    }
}

fn verify_doc_content<F>(
    handle: &crate::types::ProjectHandle,
    target: &ResolveTarget,
    check: F,
) -> Result<()>
where
    F: FnOnce(&str) -> bool,
{
    let data = project::load_project_data(handle)?;
    let doc = binder::resolve_doc(&data, target)?;
    if check(&doc.content) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "content update could not be applied without risking rich-text formatting"
        ))
    }
}

impl DocKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocKind::Folder => "folder",
            DocKind::Text => "text",
        }
    }
}
