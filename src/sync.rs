use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::Serialize;

use crate::binder;
use crate::conflict;
use crate::io;
use crate::mirror;
use crate::project;
use crate::types::{ConflictRecord, DocKind, ProjectData, ProjectHandle, SyncState, SyncStatus};

#[derive(Debug)]
pub enum SyncError {
    Conflict(Vec<ConflictRecord>),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for SyncError {
    fn from(value: anyhow::Error) -> Self {
        SyncError::Other(value)
    }
}

pub fn pull(handle: &ProjectHandle, data: &ProjectData) -> Result<()> {
    mirror::ensure_dirs(handle)?;
    let existing = mirror::read_manifest(handle).unwrap_or_default();
    let generation = existing.generation.saturating_add(1);
    let manifest = mirror::materialize(handle, data, generation)?;
    mirror::write_manifest(handle, &manifest)?;
    Ok(())
}

pub fn push(handle: &ProjectHandle, data: &mut ProjectData) -> std::result::Result<(), SyncError> {
    mirror::ensure_dirs(handle)?;
    let mut manifest = mirror::read_manifest(handle)?;
    let docs = mirror::read_mirror_docs(handle, data)?;

    let mut conflicts = Vec::new();
    for doc in docs {
        let node = data
            .docs
            .get(&doc.id)
            .ok_or_else(|| anyhow!("document {} missing", doc.id))?
            .clone();
        if matches!(node.kind, DocKind::Folder) {
            continue;
        }

        let current_source_hash = mirror::hash_text(&node.content);
        let entry = manifest.docs.entry(doc.id).or_insert_with(|| SyncState {
            source_hash: current_source_hash.clone(),
            mirror_hash: doc.mirror_hash.clone(),
            generation: manifest.generation,
        });

        let mirror_changed = entry.mirror_hash != doc.mirror_hash;
        let project_changed = entry.source_hash != current_source_hash;

        if mirror_changed && project_changed {
            let conflict =
                conflict::create_artifact(handle, data, doc.id, &doc.content, &node.content)?;
            conflicts.push(conflict);
            continue;
        }

        if mirror_changed {
            if let Some(target) = data.docs.get_mut(&doc.id) {
                target.content = doc.content;
                target.meta.updated_at = Utc::now();
            }
        }

        let final_content = data
            .docs
            .get(&doc.id)
            .map(|n| n.content.clone())
            .unwrap_or_default();
        entry.source_hash = mirror::hash_text(&final_content);
        entry.mirror_hash = mirror::hash_text(&final_content);
        entry.generation = manifest.generation.saturating_add(1);
    }

    if !conflicts.is_empty() {
        return Err(SyncError::Conflict(conflicts));
    }

    manifest.generation = manifest.generation.saturating_add(1);
    mirror::write_manifest(handle, &manifest)?;
    project::save_project_data(handle, data)?;
    pull(handle, data)?;

    Ok(())
}

pub fn status(handle: &ProjectHandle) -> Result<SyncStatus> {
    let manifest = mirror::read_manifest(handle).unwrap_or_default();
    let conflicts = conflict::status(handle)?;
    Ok(SyncStatus {
        summary: if conflicts.is_empty() {
            "clean".to_string()
        } else {
            format!("{} conflict(s)", conflicts.len())
        },
        generation: manifest.generation,
        tracked_docs: manifest.docs.len(),
        conflict_count: conflicts.len(),
    })
}

pub fn with_write_through<F>(
    handle: &ProjectHandle,
    data: &mut ProjectData,
    mutator: F,
) -> Result<()>
where
    F: FnOnce(&mut ProjectData) -> Result<()>,
{
    let _lock = ProjectLock::acquire(handle)?;
    mirror::ensure_dirs(handle)?;

    mutator(data)?;

    let backup_path = handle.scriv_dir.join(".scriv-cli/project.json");
    let _ = io::backup_file(&backup_path, &mirror::backup_root(handle));
    project::save_project_data(handle, data)?;
    pull(handle, data)?;

    match push(handle, data) {
        Ok(_) => Ok(()),
        Err(SyncError::Conflict(conflicts)) => Err(anyhow!(
            "write blocked by conflict(s): {}",
            serde_json::to_string(&ConflictList(conflicts)).unwrap_or_else(|_| "[]".to_string())
        )),
        Err(SyncError::Other(err)) => Err(err),
    }
}

#[derive(Serialize)]
struct ConflictList(Vec<ConflictRecord>);

struct ProjectLock {
    path: std::path::PathBuf,
}

impl ProjectLock {
    fn acquire(handle: &ProjectHandle) -> Result<Self> {
        let path = handle.mirror_dir.join(".scriv/state/lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.exists() {
            return Err(anyhow!("project is locked by another mutation"));
        }
        fs::write(&path, std::process::id().to_string())?;
        Ok(Self { path })
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn mirror_file_to_doc_id(
    handle: &ProjectHandle,
    data: &ProjectData,
    file: &Path,
) -> Result<Option<uuid::Uuid>> {
    let rel = match mirror::relative_to_binder(handle, file) {
        Some(r) => r,
        None => return Ok(None),
    };

    if !rel.ends_with(".md") {
        return Ok(None);
    }
    let stem = rel.trim_end_matches(".md").replace('_', " ");
    let id = binder::resolve_id_by_path(data, &stem).ok();
    Ok(id)
}
