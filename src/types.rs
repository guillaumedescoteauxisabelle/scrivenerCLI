use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHandle {
    pub root_dir: PathBuf,
    pub scriv_dir: PathBuf,
    pub mirror_dir: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub title: String,
    pub template: String,
    pub root_id: Uuid,
    pub docs: BTreeMap<Uuid, DocNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocNode {
    pub id: Uuid,
    pub parent: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub title: String,
    pub path_hint: String,
    pub kind: DocKind,
    pub content: String,
    pub meta: DocMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocKind {
    Folder,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocMeta {
    pub notes: String,
    pub synopsis: String,
    pub label: Option<String>,
    pub status: Option<String>,
    pub keywords: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorIndex {
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: Uuid,
    pub binder_path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub generation: u64,
    pub docs: BTreeMap<Uuid, SyncState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncState {
    pub source_hash: String,
    pub mirror_hash: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEnvelope {
    pub ok: bool,
    pub message: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ResolveTarget {
    pub id: Option<Uuid>,
    pub path: Option<String>,
}

impl ResolveTarget {
    pub fn new(id: Option<Uuid>, path: Option<String>) -> Result<Self> {
        if id.is_none() && path.is_none() {
            return Err(anyhow!("one of --id or --path is required"));
        }
        Ok(Self { id, path })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success,
    Runtime,
    InvalidArgs,
    NotFound,
    ValidationFailure,
    Conflict,
    CompileFailed,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::Runtime => 1,
            ExitCode::InvalidArgs => 2,
            ExitCode::NotFound => 3,
            ExitCode::ValidationFailure => 4,
            ExitCode::Conflict => 5,
            ExitCode::CompileFailed => 6,
        }
    }

    pub fn from_i32(code: i32) -> Self {
        match code {
            0 => ExitCode::Success,
            2 => ExitCode::InvalidArgs,
            3 => ExitCode::NotFound,
            4 => ExitCode::ValidationFailure,
            5 => ExitCode::Conflict,
            6 => ExitCode::CompileFailed,
            _ => ExitCode::Runtime,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProjectIssue {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub summary: String,
    pub generation: u64,
    pub tracked_docs: usize,
    pub conflict_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConflictRecord {
    pub id: Uuid,
    pub binder_path: String,
    pub created_at: DateTime<Utc>,
    pub folder: String,
}
