use std::collections::HashSet;

use anyhow::{Result, anyhow};
use chrono::Utc;
use uuid::Uuid;

use crate::io::normalize_binder_path;
use crate::types::{DocKind, DocNode, ProjectData, ResolveTarget};

pub fn list(data: &ProjectData, path: Option<&str>, recursive: bool) -> Result<Vec<String>> {
    let start = if let Some(path) = path {
        let target = resolve_id_by_path(data, path)?;
        target
    } else {
        data.root_id
    };

    let mut lines = Vec::new();
    walk_list(data, start, 0, recursive, &mut lines)?;
    Ok(lines)
}

fn walk_list(
    data: &ProjectData,
    id: Uuid,
    level: usize,
    recursive: bool,
    out: &mut Vec<String>,
) -> Result<()> {
    let node = data
        .docs
        .get(&id)
        .ok_or_else(|| anyhow!("node not found"))?;
    let indent = "  ".repeat(level);
    out.push(format!(
        "{}{} [{}] {}",
        indent,
        node.title,
        node.id,
        node.kind.as_str()
    ));
    if recursive || level == 0 {
        for child in &node.children {
            walk_list(data, *child, level + 1, recursive, out)?;
        }
    }
    Ok(())
}

pub fn resolve_doc<'a>(data: &'a ProjectData, target: &ResolveTarget) -> Result<&'a DocNode> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    if matches!(node.kind, DocKind::Folder) {
        return Err(anyhow!("target is folder; expected text document"));
    }
    Ok(node)
}

pub fn set_content(data: &mut ProjectData, target: &ResolveTarget, content: &str) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    ensure_text(node)?;
    node.content = content.to_string();
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn append_content(data: &mut ProjectData, target: &ResolveTarget, content: &str) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    ensure_text(node)?;
    node.content.push_str(content);
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn prepend_content(
    data: &mut ProjectData,
    target: &ResolveTarget,
    content: &str,
) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    ensure_text(node)?;
    node.content = format!("{content}{}", node.content);
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn edit_doc(
    data: &mut ProjectData,
    target: &ResolveTarget,
    title: Option<&str>,
    text: Option<&str>,
) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    if let Some(title) = title {
        node.title = title.to_string();
        node.path_hint = title.to_string();
    }
    if let Some(text) = text {
        ensure_text(node)?;
        node.content = text.to_string();
    }
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn set_notes(data: &mut ProjectData, target: &ResolveTarget, notes: &str) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    node.meta.notes = notes.to_string();
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn set_synopsis(data: &mut ProjectData, target: &ResolveTarget, synopsis: &str) -> Result<()> {
    let id = resolve_id(data, target)?;
    let node = data
        .docs
        .get_mut(&id)
        .ok_or_else(|| anyhow!("document not found"))?;
    node.meta.synopsis = synopsis.to_string();
    node.meta.updated_at = Utc::now();
    Ok(())
}

pub fn mkdir(data: &mut ProjectData, path: &str) -> Result<()> {
    let path = canonical_path(data, path);
    if path.is_empty() {
        return Err(anyhow!("path cannot be empty"));
    }

    let mut parent = data.root_id;
    let mut visited = HashSet::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }

        if !visited.insert((parent, segment.to_string())) {
            return Err(anyhow!("duplicate folder segment"));
        }

        let existing = data
            .docs
            .values()
            .find(|d| {
                d.parent == Some(parent) && d.title == segment && matches!(d.kind, DocKind::Folder)
            })
            .map(|d| d.id);

        if let Some(id) = existing {
            parent = id;
        } else {
            let id = Uuid::new_v4();
            let node = DocNode {
                id,
                parent: Some(parent),
                children: Vec::new(),
                title: segment.to_string(),
                path_hint: segment.to_string(),
                kind: DocKind::Folder,
                content: String::new(),
                meta: Default::default(),
            };
            data.docs.insert(id, node);
            if let Some(p) = data.docs.get_mut(&parent) {
                p.children.push(id);
            }
            parent = id;
        }
    }
    Ok(())
}

pub fn mkdoc(data: &mut ProjectData, path: &str) -> Result<()> {
    let path = canonical_path(data, path);
    if path.is_empty() {
        return Err(anyhow!("path cannot be empty"));
    }

    let mut parts = path.split('/').collect::<Vec<_>>();
    let name = parts.pop().ok_or_else(|| anyhow!("invalid path"))?;
    let folder_path = parts.join("/");
    if !folder_path.is_empty() {
        mkdir(data, &folder_path)?;
    }
    let parent = if folder_path.is_empty() {
        data.root_id
    } else {
        resolve_id_by_path(data, &folder_path)?
    };

    if data
        .docs
        .values()
        .any(|d| d.parent == Some(parent) && d.title == name)
    {
        return Err(anyhow!("node already exists at path"));
    }

    let id = Uuid::new_v4();
    data.docs.insert(
        id,
        DocNode {
            id,
            parent: Some(parent),
            children: Vec::new(),
            title: name.to_string(),
            path_hint: name.to_string(),
            kind: DocKind::Text,
            content: String::new(),
            meta: Default::default(),
        },
    );
    if let Some(parent_node) = data.docs.get_mut(&parent) {
        parent_node.children.push(id);
    }
    Ok(())
}

pub fn mv(data: &mut ProjectData, from: &str, to: &str) -> Result<()> {
    let from_id = resolve_id_by_path(data, from)?;
    let to_parent_path = canonical_path(data, to);
    let mut parts: Vec<&str> = to_parent_path.split('/').collect();
    let new_name = parts
        .pop()
        .ok_or_else(|| anyhow!("invalid destination path"))?;
    let parent_path = parts.join("/");

    let new_parent = if parent_path.is_empty() {
        data.root_id
    } else {
        resolve_id_by_path(data, &parent_path)?
    };

    let old_parent = data.docs.get(&from_id).and_then(|n| n.parent);
    if let Some(op) = old_parent {
        if let Some(node) = data.docs.get_mut(&op) {
            node.children.retain(|c| *c != from_id);
        }
    }
    if let Some(parent) = data.docs.get_mut(&new_parent) {
        parent.children.push(from_id);
    }

    if let Some(node) = data.docs.get_mut(&from_id) {
        node.parent = Some(new_parent);
        node.title = new_name.to_string();
        node.path_hint = new_name.to_string();
        node.meta.updated_at = Utc::now();
    }

    Ok(())
}

pub fn rm(data: &mut ProjectData, path: &str, force: bool) -> Result<()> {
    let id = resolve_id_by_path(data, path)?;
    if id == data.root_id {
        return Err(anyhow!("cannot remove root"));
    }

    let has_children = data
        .docs
        .get(&id)
        .map(|n| !n.children.is_empty())
        .unwrap_or(false);
    if has_children && !force {
        return Err(anyhow!("folder not empty; pass --force"));
    }

    let mut to_delete = Vec::new();
    collect_descendants(data, id, &mut to_delete)?;

    let parent = data.docs.get(&id).and_then(|n| n.parent);
    if let Some(parent) = parent {
        if let Some(p) = data.docs.get_mut(&parent) {
            p.children.retain(|c| *c != id);
        }
    }

    for doc_id in to_delete {
        data.docs.remove(&doc_id);
    }
    data.docs.remove(&id);
    Ok(())
}

fn collect_descendants(data: &ProjectData, id: Uuid, out: &mut Vec<Uuid>) -> Result<()> {
    let node = data
        .docs
        .get(&id)
        .ok_or_else(|| anyhow!("node not found"))?;
    for child in &node.children {
        collect_descendants(data, *child, out)?;
        out.push(*child);
    }
    Ok(())
}

pub fn reorder(
    data: &mut ProjectData,
    path: &str,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<()> {
    if before.is_some() && after.is_some() {
        return Err(anyhow!("pass only one of --before or --after"));
    }

    let id = resolve_id_by_path(data, path)?;
    let parent = data
        .docs
        .get(&id)
        .and_then(|n| n.parent)
        .ok_or_else(|| anyhow!("root cannot be reordered"))?;
    let anchor = before
        .or(after)
        .ok_or_else(|| anyhow!("--before or --after required"))?;
    let anchor_id = resolve_id_by_path(data, anchor)?;
    let siblings = data
        .docs
        .get_mut(&parent)
        .ok_or_else(|| anyhow!("parent not found"))?;
    siblings.children.retain(|c| *c != id);
    let idx = siblings
        .children
        .iter()
        .position(|c| *c == anchor_id)
        .ok_or_else(|| anyhow!("anchor not found among siblings"))?;

    let insert_at = if before.is_some() { idx } else { idx + 1 };
    if insert_at >= siblings.children.len() {
        siblings.children.push(id);
    } else {
        siblings.children.insert(insert_at, id);
    }

    Ok(())
}

pub fn resolve_id(data: &ProjectData, target: &ResolveTarget) -> Result<Uuid> {
    if let Some(id) = target.id {
        return if data.docs.contains_key(&id) {
            Ok(id)
        } else {
            Err(anyhow!("id not found"))
        };
    }

    if let Some(path) = &target.path {
        return resolve_id_by_path(data, path);
    }

    Err(anyhow!("id or path required"))
}

pub fn resolve_id_by_path(data: &ProjectData, path: &str) -> Result<Uuid> {
    let path = canonical_path(data, path);
    if path.is_empty() {
        return Ok(data.root_id);
    }

    let mut current = data.root_id;
    for segment in path.split('/') {
        let next = data
            .docs
            .values()
            .find(|d| d.parent == Some(current) && d.title == segment)
            .map(|d| d.id)
            .ok_or_else(|| anyhow!("path segment not found: {segment}"))?;
        current = next;
    }

    Ok(current)
}

fn canonical_path(data: &ProjectData, path: &str) -> String {
    let path = normalize_binder_path(path);
    if path.is_empty() {
        return path;
    }

    let root_title = data
        .docs
        .get(&data.root_id)
        .map(|n| n.title.as_str())
        .unwrap_or("Draft");

    let mut parts = path.split('/').collect::<Vec<_>>();
    if parts
        .first()
        .is_some_and(|p| p.eq_ignore_ascii_case(root_title))
    {
        parts.remove(0);
    }
    parts.join("/")
}

pub fn binder_path(data: &ProjectData, id: Uuid) -> Result<String> {
    let mut segments = Vec::new();
    let mut current = id;
    loop {
        let node = data
            .docs
            .get(&current)
            .ok_or_else(|| anyhow!("node not found"))?;
        segments.push(node.title.clone());
        if let Some(parent) = node.parent {
            current = parent;
        } else {
            break;
        }
    }
    segments.reverse();
    Ok(segments.join("/"))
}

fn ensure_text(node: &DocNode) -> Result<()> {
    if matches!(node.kind, DocKind::Folder) {
        return Err(anyhow!("target is folder; expected text document"));
    }
    Ok(())
}

pub fn ordered_nodes(data: &ProjectData) -> Result<Vec<Uuid>> {
    let mut out = Vec::new();
    walk_order(data, data.root_id, &mut out)?;
    Ok(out)
}

fn walk_order(data: &ProjectData, id: Uuid, out: &mut Vec<Uuid>) -> Result<()> {
    out.push(id);
    let node = data
        .docs
        .get(&id)
        .ok_or_else(|| anyhow!("node not found"))?;
    for child in &node.children {
        walk_order(data, *child, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::types::{DocMeta, ProjectData};

    #[test]
    fn resolves_path() {
        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let mut docs = BTreeMap::new();
        docs.insert(
            root_id,
            DocNode {
                id: root_id,
                parent: None,
                children: vec![child_id],
                title: "Draft".to_string(),
                path_hint: "Draft".to_string(),
                kind: DocKind::Folder,
                content: String::new(),
                meta: DocMeta::default(),
            },
        );
        docs.insert(
            child_id,
            DocNode {
                id: child_id,
                parent: Some(root_id),
                children: vec![],
                title: "Chapter 1".to_string(),
                path_hint: "Chapter 1".to_string(),
                kind: DocKind::Text,
                content: "hello".to_string(),
                meta: DocMeta::default(),
            },
        );

        let data = ProjectData {
            title: "Test".to_string(),
            template: "blank".to_string(),
            root_id,
            docs,
        };

        let id = resolve_id_by_path(&data, "Draft/Chapter 1").expect("path should resolve");
        assert_eq!(id, child_id);
    }
}
