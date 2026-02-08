use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use chrono::Utc;
use uuid::Uuid;

use crate::binder;
use crate::io;
use crate::mirror;
use crate::project;
use crate::types::{ConflictRecord, ProjectHandle, ResolveTarget};

pub fn create_artifact(
    handle: &ProjectHandle,
    data: &crate::types::ProjectData,
    id: Uuid,
    ours: &str,
    theirs: &str,
) -> Result<ConflictRecord> {
    let binder_path = binder::binder_path(data, id)?;
    let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let conflict_dir = mirror::conflicts_root(handle).join(&ts);
    fs::create_dir_all(&conflict_dir)?;

    let ours_path = conflict_dir.join(format!("{id}.ours"));
    let theirs_path = conflict_dir.join(format!("{id}.theirs"));
    fs::write(&ours_path, ours)?;
    fs::write(&theirs_path, theirs)?;

    let record = ConflictRecord {
        id,
        binder_path,
        created_at: Utc::now(),
        folder: conflict_dir.to_string_lossy().to_string(),
    };

    let report = conflict_dir.join("report.json");
    fs::write(report, serde_json::to_string_pretty(&record)?)?;
    Ok(record)
}

pub fn status(handle: &ProjectHandle) -> Result<Vec<ConflictRecord>> {
    let root = mirror::conflicts_root(handle);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let report = path.join("report.json");
        if report.exists() {
            let parsed: ConflictRecord = serde_json::from_str(&fs::read_to_string(report)?)?;
            out.push(parsed);
        }
    }
    out.sort_by_key(|c| c.created_at);
    Ok(out)
}

pub fn resolve(
    handle: &ProjectHandle,
    target: &ResolveTarget,
    strategy: &str,
    manual_file: Option<&Path>,
) -> Result<()> {
    let mut data = project::load_project_data(handle)?;
    let id = binder::resolve_id(&data, target)?;

    let conflicts = status(handle)?;
    let conflict = conflicts
        .iter()
        .rev()
        .find(|c| c.id == id)
        .ok_or_else(|| anyhow!("no conflict found for target"))?;

    let folder = Path::new(&conflict.folder);
    let ours = fs::read_to_string(folder.join(format!("{id}.ours")))?;
    let theirs = fs::read_to_string(folder.join(format!("{id}.theirs")))?;

    let resolved = match strategy {
        "mirror" => ours,
        "project" => theirs,
        "manual" => {
            let path =
                manual_file.ok_or_else(|| anyhow!("--manual-file required for manual strategy"))?;
            fs::read_to_string(path)?
        }
        _ => return Err(anyhow!("invalid strategy: use mirror|project|manual")),
    };

    if let Some(node) = data.docs.get_mut(&id) {
        node.content = resolved.clone();
    }
    project::save_project_data(handle, &data)?;

    // Keep project, mirror, and manifest aligned after an explicit resolution.
    let binder_path = binder::binder_path(&data, id)?;
    let mirror_md = handle
        .mirror_dir
        .join("binder")
        .join(format!("{}.md", mirror::sanitize_path(&binder_path)));
    io::atomic_write(&mirror_md, &resolved)?;

    let mut manifest = mirror::read_manifest(handle).unwrap_or_default();
    let hash = mirror::hash_text(&resolved);
    let next_generation = manifest.generation.saturating_add(1);
    manifest
        .docs
        .entry(id)
        .and_modify(|s| {
            s.source_hash = hash.clone();
            s.mirror_hash = hash.clone();
            s.generation = next_generation;
        })
        .or_insert(crate::types::SyncState {
            source_hash: hash.clone(),
            mirror_hash: hash,
            generation: next_generation,
        });
    manifest.generation = next_generation;
    mirror::write_manifest(handle, &manifest)?;

    // Remove every outstanding artifact for this id.
    for rec in conflicts.iter().filter(|c| c.id == id) {
        let _ = fs::remove_dir_all(&rec.folder);
    }
    let _ = fs::remove_dir_all(folder);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProjectHandle;
    use tempfile::tempdir;

    #[test]
    fn status_empty_when_missing_dir() {
        let dir = tempdir().expect("tempdir");
        let handle = ProjectHandle {
            root_dir: dir.path().to_path_buf(),
            scriv_dir: dir.path().join("Test.scriv"),
            mirror_dir: dir.path().join("Test.scriv-mirror"),
        };
        let conflicts = status(&handle).expect("status");
        assert!(conflicts.is_empty());
    }
}
