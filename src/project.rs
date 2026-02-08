use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use quick_xml::de::from_str;
use serde::Deserialize;
use sha1::{Digest as Sha1Digest, Sha1};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::io;
use crate::types::{DocKind, DocMeta, DocNode, ProjectData, ProjectHandle, ProjectIssue};

const CLI_STATE_FILE: &str = ".scriv-cli/project.json";

pub fn create_project(name: &str, dir: Option<PathBuf>, template: &str) -> Result<ProjectHandle> {
    let base_dir = dir.unwrap_or(std::env::current_dir()?);
    let root_dir = base_dir.join(name);
    let scriv_dir = root_dir.join(format!("{name}.scriv"));
    let mirror_dir = root_dir.join(format!("{name}.scriv-mirror"));

    fs::create_dir_all(&scriv_dir)?;
    fs::create_dir_all(&mirror_dir)?;
    fs::create_dir_all(scriv_dir.join("Files/Data"))?;

    let root_id = Uuid::new_v4();
    let mut docs = BTreeMap::new();
    docs.insert(
        root_id,
        DocNode {
            id: root_id,
            parent: None,
            children: Vec::new(),
            title: "Draft".to_string(),
            path_hint: "Draft".to_string(),
            kind: DocKind::Folder,
            content: String::new(),
            meta: DocMeta {
                updated_at: Utc::now(),
                ..DocMeta::default()
            },
        },
    );

    let data = ProjectData {
        title: name.to_string(),
        template: template.to_string(),
        root_id,
        docs,
    };

    save_project_data(
        &ProjectHandle {
            root_dir,
            scriv_dir,
            mirror_dir,
        },
        &data,
    )?;

    write_minimal_scrivx(&data, &base_dir.join(name).join(format!("{name}.scriv")))?;

    Ok(ProjectHandle {
        root_dir: base_dir.join(name),
        scriv_dir: base_dir.join(name).join(format!("{name}.scriv")),
        mirror_dir: base_dir.join(name).join(format!("{name}.scriv-mirror")),
    })
}

pub fn open_project(explicit: Option<&Path>) -> Result<ProjectHandle> {
    if let Some(path) = explicit {
        return resolve_project_path(path);
    }

    let cwd = std::env::current_dir()?;
    resolve_project_path(&cwd)
}

fn resolve_project_path(path: &Path) -> Result<ProjectHandle> {
    if path.extension().and_then(|s| s.to_str()) == Some("scriv") && path.is_dir() {
        let root_dir = path
            .parent()
            .ok_or_else(|| anyhow!("invalid project path"))?
            .to_path_buf();
        let project_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("invalid project name"))?;
        return Ok(ProjectHandle {
            root_dir: root_dir.clone(),
            scriv_dir: path.to_path_buf(),
            mirror_dir: root_dir.join(format!("{project_stem}.scriv-mirror")),
        });
    }

    if path.is_dir() {
        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("scriv") && p.is_dir() {
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Project")
                    .to_string();
                return Ok(ProjectHandle {
                    root_dir: path.to_path_buf(),
                    scriv_dir: p.clone(),
                    mirror_dir: path.join(format!("{stem}.scriv-mirror")),
                });
            }
        }
    }

    Err(anyhow!(
        "could not resolve project. pass --project <Project.scriv|project_root>"
    ))
}

pub fn load_project_data(handle: &ProjectHandle) -> Result<ProjectData> {
    let state_path = handle.scriv_dir.join(CLI_STATE_FILE);
    if state_path.exists() {
        let text = fs::read_to_string(&state_path)?;
        return Ok(serde_json::from_str(&text)?);
    }

    let imported = import_from_scrivx(handle)?;
    save_project_data(handle, &imported)?;
    Ok(imported)
}

pub fn save_project_data(handle: &ProjectHandle, data: &ProjectData) -> Result<()> {
    let state_path = handle.scriv_dir.join(CLI_STATE_FILE);
    io::ensure_parent(&state_path)?;
    io::atomic_write(&state_path, &serde_json::to_string_pretty(data)?)?;
    write_native_data_files(handle, data)?;
    Ok(())
}

pub fn validate_project(handle: &ProjectHandle, strict: bool) -> Result<()> {
    if !handle.scriv_dir.exists() {
        return Err(anyhow!("scrivener package not found"));
    }
    if strict {
        let data = load_project_data(handle)?;
        if !data.docs.contains_key(&data.root_id) {
            return Err(anyhow!("root node is missing"));
        }
    }
    Ok(())
}

pub fn doctor_project(handle: &ProjectHandle, _check: bool) -> Result<Vec<ProjectIssue>> {
    let mut issues = Vec::new();
    if !handle.scriv_dir.join(CLI_STATE_FILE).exists() {
        issues.push(ProjectIssue {
            code: "missing_state".to_string(),
            message: "project state has not been initialized yet; run sync pull".to_string(),
        });
    }
    if !handle.mirror_dir.exists() {
        issues.push(ProjectIssue {
            code: "missing_mirror".to_string(),
            message: "mirror directory does not exist yet; run sync pull".to_string(),
        });
    }
    Ok(issues)
}

fn write_minimal_scrivx(data: &ProjectData, scriv_dir: &Path) -> Result<()> {
    let path = scriv_dir.join(format!("{}.scrivx", data.title));
    let xml = format!(
        "<ScrivenerProject><Binder><BinderItem UUID=\"{}\" Type=\"Folder\" Title=\"Draft\"/></Binder></ScrivenerProject>",
        data.root_id
    );
    io::atomic_write(&path, &xml)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ScrivenerProject {
    #[serde(rename = "Binder")]
    binder: Option<BinderRoot>,
}

#[derive(Debug, Deserialize)]
struct BinderRoot {
    #[serde(rename = "BinderItem", default)]
    items: Vec<ScrivBinderItem>,
}

#[derive(Debug, Deserialize)]
struct ScrivBinderItem {
    #[serde(rename = "@UUID")]
    uuid: Option<String>,
    #[serde(rename = "@Type")]
    item_type: Option<String>,
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "Children")]
    children: Option<ScrivChildren>,
}

#[derive(Debug, Deserialize)]
struct ScrivChildren {
    #[serde(rename = "BinderItem", default)]
    items: Vec<ScrivBinderItem>,
}

fn import_from_scrivx(handle: &ProjectHandle) -> Result<ProjectData> {
    let mut scrivx_files = fs::read_dir(&handle.scriv_dir)?
        .filter_map(|e| e.ok().map(|v| v.path()))
        .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("scrivx"))
        .collect::<Vec<_>>();
    scrivx_files.sort();

    let title = handle
        .scriv_dir
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Project")
        .to_string();

    if scrivx_files.is_empty() {
        let root_id = Uuid::new_v4();
        let mut docs = BTreeMap::new();
        docs.insert(
            root_id,
            DocNode {
                id: root_id,
                parent: None,
                children: Vec::new(),
                title: "Draft".to_string(),
                path_hint: "Draft".to_string(),
                kind: DocKind::Folder,
                content: String::new(),
                meta: DocMeta {
                    updated_at: Utc::now(),
                    ..DocMeta::default()
                },
            },
        );
        return Ok(ProjectData {
            title,
            template: "imported".to_string(),
            root_id,
            docs,
        });
    }

    let xml = fs::read_to_string(&scrivx_files[0]).context("failed to read .scrivx file")?;
    let parsed: ScrivenerProject = from_str(&xml).unwrap_or(ScrivenerProject { binder: None });

    let mut docs = BTreeMap::new();
    let root_id = Uuid::new_v4();
    docs.insert(
        root_id,
        DocNode {
            id: root_id,
            parent: None,
            children: Vec::new(),
            title: "Draft".to_string(),
            path_hint: "Draft".to_string(),
            kind: DocKind::Folder,
            content: String::new(),
            meta: DocMeta {
                updated_at: Utc::now(),
                ..DocMeta::default()
            },
        },
    );

    if let Some(binder) = parsed.binder {
        for item in binder.items {
            import_item(handle, &mut docs, root_id, item);
        }
    }

    Ok(ProjectData {
        title,
        template: "imported".to_string(),
        root_id,
        docs,
    })
}

fn import_item(
    handle: &ProjectHandle,
    docs: &mut BTreeMap<Uuid, DocNode>,
    parent: Uuid,
    item: ScrivBinderItem,
) {
    let id = item
        .uuid
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::new_v4);
    let title = item.title.unwrap_or_else(|| "Untitled".to_string());
    let raw_type = item.item_type.unwrap_or_else(|| "Text".to_string());
    let kind = if raw_type.contains("Folder") {
        DocKind::Folder
    } else {
        DocKind::Text
    };
    let content = if matches!(kind, DocKind::Text) {
        read_rtf_doc_text(handle, &id).unwrap_or_default()
    } else {
        String::new()
    };
    let notes = read_notes(handle, &id).unwrap_or_default();
    let synopsis = read_synopsis(handle, &id).unwrap_or_default();

    let node = DocNode {
        id,
        parent: Some(parent),
        children: Vec::new(),
        title: title.clone(),
        path_hint: title,
        kind,
        content,
        meta: DocMeta {
            notes,
            synopsis,
            updated_at: Utc::now(),
            ..DocMeta::default()
        },
    };

    if let Some(parent_node) = docs.get_mut(&parent) {
        parent_node.children.push(id);
    }
    docs.insert(id, node);

    for child in item
        .children
        .map(|children| children.items)
        .unwrap_or_default()
    {
        import_item(handle, docs, id, child);
    }
}

fn read_rtf_doc_text(handle: &ProjectHandle, id: &Uuid) -> Result<String> {
    let folder = id.to_string().to_uppercase();
    let path = handle
        .scriv_dir
        .join("Files/Data")
        .join(folder)
        .join("content.rtf");
    if !path.exists() {
        return Ok(String::new());
    }
    let rtf = fs::read_to_string(path)?;
    Ok(strip_rtf(&rtf))
}

fn read_notes(handle: &ProjectHandle, id: &Uuid) -> Result<String> {
    let folder = id.to_string().to_uppercase();
    let path = handle
        .scriv_dir
        .join("Files/Data")
        .join(folder)
        .join("notes.rtf");
    if !path.exists() {
        return Ok(String::new());
    }
    let rtf = fs::read_to_string(path)?;
    Ok(strip_rtf(&rtf))
}

fn read_synopsis(handle: &ProjectHandle, id: &Uuid) -> Result<String> {
    let folder = id.to_string().to_uppercase();
    let path = handle
        .scriv_dir
        .join("Files/Data")
        .join(folder)
        .join("synopsis.txt");
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn strip_rtf(input: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '{' | '}' => {
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                if chars[i] == '\'' {
                    if i + 2 < chars.len() {
                        let hex = format!("{}{}", chars[i + 1], chars[i + 2]);
                        if let Ok(v) = u8::from_str_radix(&hex, 16) {
                            out.push(v as char);
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '-') {
                    i += 1;
                }
                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                if word == "par" || word == "line" {
                    out.push('\n');
                } else if word == "tab" {
                    out.push('\t');
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }

    out.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn write_native_data_files(handle: &ProjectHandle, data: &ProjectData) -> Result<()> {
    prune_orphan_data_dirs(handle, data)?;

    for node in data.docs.values() {
        if !matches!(node.kind, DocKind::Text) {
            continue;
        }

        let folder = handle
            .scriv_dir
            .join("Files/Data")
            .join(node.id.to_string().to_uppercase());
        fs::create_dir_all(&folder)?;
        sync_content_rtf_preserving_formatting(&folder.join("content.rtf"), &node.content)?;

        let synopsis = node.meta.synopsis.trim();
        let synopsis_path = folder.join("synopsis.txt");
        if synopsis.is_empty() {
            if synopsis_path.exists() {
                fs::remove_file(&synopsis_path)?;
            }
        } else {
            io::atomic_write(&synopsis_path, synopsis)?;
        }

        let notes = node.meta.notes.trim();
        let notes_path = folder.join("notes.rtf");
        if notes.is_empty() {
            if notes_path.exists() {
                fs::remove_file(&notes_path)?;
            }
        } else {
            io::atomic_write(&notes_path, &to_basic_rtf(notes))?;
        }
    }
    refresh_docs_checksum(handle)?;
    Ok(())
}

fn prune_orphan_data_dirs(handle: &ProjectHandle, data: &ProjectData) -> Result<()> {
    let data_root = handle.scriv_dir.join("Files/Data");
    if !data_root.exists() {
        return Ok(());
    }

    let keep: std::collections::HashSet<String> = data
        .docs
        .keys()
        .map(|id| id.to_string().to_uppercase())
        .collect();

    for entry in fs::read_dir(&data_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !looks_like_uuid(&name) {
            continue;
        }
        if keep.contains(&name) {
            continue;
        }

        // Only remove directories that look like normal Scrivener text-item payloads.
        if is_safe_orphan_dir(&path)? {
            fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

fn looks_like_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn is_safe_orphan_dir(path: &Path) -> Result<bool> {
    let allowed = [
        "content.rtf",
        "notes.rtf",
        "synopsis.txt",
        "content.styles",
        ".DS_Store",
    ];
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.path().is_dir() {
            return Ok(false);
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !allowed.contains(&name.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn sync_content_rtf_preserving_formatting(content_path: &Path, desired_text: &str) -> Result<()> {
    if !content_path.exists() {
        if !desired_text.is_empty() {
            io::atomic_write(content_path, &to_basic_rtf(desired_text))?;
        }
        return Ok(());
    }

    let current_rtf = fs::read_to_string(content_path)?;
    let current_text = strip_rtf(&current_rtf);
    if current_text == desired_text {
        return Ok(());
    }

    if let Some(prefix) = desired_text.strip_suffix(&current_text) {
        if !prefix.is_empty() {
            let patched = prepend_plain_text_to_rtf(&current_rtf, prefix);
            io::atomic_write(content_path, &patched)?;
            return Ok(());
        }
    }

    if is_basic_rtf(&current_rtf) {
        io::atomic_write(content_path, &to_basic_rtf(desired_text))?;
        return Ok(());
    }

    // Preserve existing rich formatting when we cannot safely transform.
    Ok(())
}

fn prepend_plain_text_to_rtf(rtf: &str, prefix: &str) -> String {
    let insert = find_rtf_text_insert_idx(rtf);
    let escaped = escape_rtf(prefix);
    format!("{}{}{}", &rtf[..insert], escaped, &rtf[insert..])
}

fn find_rtf_text_insert_idx(rtf: &str) -> usize {
    if let Some(pard) = rtf.find("\\pard") {
        let bytes = rtf.as_bytes();
        let mut i = pard + "\\pard".len();
        while i < bytes.len() {
            if bytes[i] == b' ' {
                return i + 1;
            }
            i += 1;
        }
    }
    if let Some(pos) = rtf.find('\n') {
        return pos + 1;
    }
    0
}

fn to_basic_rtf(text: &str) -> String {
    let escaped = escape_rtf(text);
    format!("{{\\rtf1\\ansi\\deff0\n{}\n}}", escaped)
}

fn escape_rtf(text: &str) -> String {
    let mut escaped = String::new();
    for c in text.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '\n' => escaped.push_str("\\par\n"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn is_basic_rtf(rtf: &str) -> bool {
    rtf.trim_start().starts_with("{\\rtf1\\ansi\\deff0")
}

fn refresh_docs_checksum(handle: &ProjectHandle) -> Result<()> {
    let data_root = handle.scriv_dir.join("Files/Data");
    if !data_root.exists() {
        return Ok(());
    }

    let mut lines = Vec::new();
    for entry in WalkDir::new(&data_root).follow_links(false) {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("docs.checksum") {
            continue;
        }
        let rel = path
            .strip_prefix(&data_root)?
            .to_string_lossy()
            .replace('\\', "/");
        let hash = sha1_file(path)?;
        lines.push(format!("{}={}", rel.to_lowercase(), hash));
    }
    lines.sort();
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    io::atomic_write(&data_root.join("docs.checksum"), &content)?;
    Ok(())
}

fn sha1_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
