use std::fs;
use std::process::Command;

use anyhow::{Result, anyhow};

use crate::binder;
use crate::types::{DocKind, ProjectData, ProjectHandle};

pub enum CompileOutcome {
    BuiltIn,
    AppBridgeUnsupported,
}

pub fn run(
    _handle: &ProjectHandle,
    data: &ProjectData,
    format: &str,
    output: &std::path::Path,
    preset: Option<&str>,
) -> Result<CompileOutcome> {
    match format {
        "md" | "txt" => {
            let mut out = String::new();
            for id in binder::ordered_nodes(data)? {
                let node = data
                    .docs
                    .get(&id)
                    .ok_or_else(|| anyhow!("node not found"))?;
                if matches!(node.kind, DocKind::Text) {
                    if format == "md" {
                        out.push_str(&format!("\n# {}\n\n", node.title));
                    } else {
                        out.push_str(&format!(
                            "\n{}\n{}\n",
                            node.title,
                            "=".repeat(node.title.len())
                        ));
                    }
                    out.push_str(&node.content);
                    out.push_str("\n");
                }
            }
            fs::write(output, out)?;
            Ok(CompileOutcome::BuiltIn)
        }
        "app" => {
            if cfg!(target_os = "macos") {
                let script = format!(
                    "tell application \"Scrivener\" to activate\ndo shell script \"echo app-compile preset={}\"",
                    preset.unwrap_or("default")
                );
                let status = Command::new("osascript").arg("-e").arg(script).status();
                match status {
                    Ok(s) if s.success() => Ok(CompileOutcome::BuiltIn),
                    Ok(_) => Err(anyhow!("Scrivener app compile failed")),
                    Err(e) => Err(anyhow!("failed to invoke osascript: {e}")),
                }
            } else {
                Ok(CompileOutcome::AppBridgeUnsupported)
            }
        }
        _ => Err(anyhow!("unsupported format: use app|md|txt")),
    }
}
