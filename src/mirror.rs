use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::binder;
use crate::io;
use crate::types::{DocKind, Manifest, MirrorIndex, ProjectData, ProjectHandle, SyncState};

#[derive(Debug, Serialize, Deserialize)]
pub struct MirrorDoc {
    pub id: Uuid,
    pub binder_path: String,
    pub content: String,
    pub source_hash: String,
    pub mirror_hash: String,
}

pub fn materialize(
    handle: &ProjectHandle,
    data: &ProjectData,
    generation: u64,
) -> Result<Manifest> {
    let binder_root = handle.mirror_dir.join("binder");
    let state_root = handle.mirror_dir.join(".scriv/state");
    fs::create_dir_all(&binder_root)?;
    fs::create_dir_all(&state_root)?;

    let mut entries = Vec::new();
    let mut docs_manifest = BTreeMap::new();

    for id in binder::ordered_nodes(data)? {
        let node = data.docs.get(&id).ok_or_else(|| anyhow!("node missing"))?;
        let path = binder::binder_path(data, id)?;

        entries.push(crate::types::IndexEntry {
            id,
            binder_path: path.clone(),
            kind: node.kind.as_str().to_string(),
        });

        let safe_path = sanitize_path(&path);
        match node.kind {
            DocKind::Folder => {
                let meta_path = binder_root.join(&safe_path).join("_folder.yml");
                io::atomic_write(&meta_path, &serde_yaml::to_string(&node.meta)?)?;
            }
            DocKind::Text => {
                let md_path = binder_root.join(format!("{safe_path}.md"));
                let sidecar_path = binder_root.join(format!("{safe_path}.meta.yml"));
                io::atomic_write(&md_path, &node.content)?;
                io::atomic_write(&sidecar_path, &serde_yaml::to_string(&node.meta)?)?;

                let source_hash = hash_text(&node.content);
                let mirror_hash = hash_text(&fs::read_to_string(&md_path)?);
                docs_manifest.insert(
                    id,
                    SyncState {
                        source_hash,
                        mirror_hash,
                        generation,
                    },
                );
            }
        }
    }

    let index = MirrorIndex {
        generated_at: Utc::now(),
        entries,
    };

    io::atomic_write(
        &state_root.join("index.json"),
        &serde_json::to_string_pretty(&index)?,
    )?;

    let manifest = Manifest {
        generation,
        docs: docs_manifest,
    };
    io::atomic_write(
        &state_root.join("manifest.json"),
        &serde_json::to_string_pretty(&manifest)?,
    )?;

    Ok(manifest)
}

pub fn read_manifest(handle: &ProjectHandle) -> Result<Manifest> {
    let path = handle.mirror_dir.join(".scriv/state/manifest.json");
    if !path.exists() {
        return Ok(Manifest::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn read_mirror_docs(handle: &ProjectHandle, data: &ProjectData) -> Result<Vec<MirrorDoc>> {
    let manifest = read_manifest(handle)?;
    let mut out = Vec::new();

    for (id, node) in &data.docs {
        if matches!(node.kind, DocKind::Folder) {
            continue;
        }
        let path = binder::binder_path(data, *id)?;
        let safe = sanitize_path(&path);
        let md_path = handle.mirror_dir.join("binder").join(format!("{safe}.md"));
        if !md_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&md_path)?;
        let mirror_hash = hash_text(&content);
        let source_hash = manifest
            .docs
            .get(id)
            .map(|s| s.source_hash.clone())
            .unwrap_or_default();

        out.push(MirrorDoc {
            id: *id,
            binder_path: path,
            content,
            source_hash,
            mirror_hash,
        });
    }

    Ok(out)
}

pub fn write_manifest(handle: &ProjectHandle, manifest: &Manifest) -> Result<()> {
    let path = handle.mirror_dir.join(".scriv/state/manifest.json");
    io::atomic_write(&path, &serde_json::to_string_pretty(manifest)?)
}

pub fn sanitize_path(path: &str) -> String {
    let replaced = path.replace(['\\', ':'], "-").replace("..", "_");
    replaced
        .split('/')
        .map(|seg| seg.trim().replace(' ', "_"))
        .collect::<Vec<_>>()
        .join("/")
}

pub fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn backup_root(handle: &ProjectHandle) -> PathBuf {
    handle.mirror_dir.join(".scriv/backups")
}

pub fn conflicts_root(handle: &ProjectHandle) -> PathBuf {
    handle.mirror_dir.join(".scriv/conflicts")
}

pub fn ensure_dirs(handle: &ProjectHandle) -> Result<()> {
    fs::create_dir_all(handle.mirror_dir.join("binder"))?;
    fs::create_dir_all(handle.mirror_dir.join(".scriv/state"))?;
    fs::create_dir_all(backup_root(handle))?;
    fs::create_dir_all(conflicts_root(handle))?;
    Ok(())
}

pub fn relative_to_binder(handle: &ProjectHandle, path: &Path) -> Option<String> {
    path.strip_prefix(handle.mirror_dir.join("binder"))
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}
