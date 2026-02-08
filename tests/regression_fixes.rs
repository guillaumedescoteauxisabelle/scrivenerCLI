use std::fs;

use scriv::{binder, conflict, mirror, project, sync};
use scriv::types::ResolveTarget;

#[test]
fn draft_prefixed_paths_work_consistently() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");
    let mut data = project::load_project_data(&handle).expect("load");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdir(d, "Draft/CLI/F1")?;
        binder::mkdoc(d, "Draft/CLI/F1/Doc")?;
        Ok(())
    })
    .expect("mkdir/mkdoc with Draft prefix");

    let data = project::load_project_data(&handle).expect("reload");
    let id_a = binder::resolve_id_by_path(&data, "Draft/CLI/F1/Doc").expect("resolve with Draft");
    let id_b = binder::resolve_id_by_path(&data, "CLI/F1/Doc").expect("resolve without Draft");
    assert_eq!(id_a, id_b);
}

#[test]
fn conflict_resolve_allows_next_push() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");
    let mut data = project::load_project_data(&handle).expect("load");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdoc(d, "Draft/CResolve")?;
        Ok(())
    })
    .expect("mkdoc");

    let mut data = project::load_project_data(&handle).expect("reload");
    let id = binder::resolve_id_by_path(&data, "Draft/CResolve").expect("resolve");
    let target = ResolveTarget::new(Some(id), None).expect("target");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::set_content(d, &target, "base")?;
        Ok(())
    })
    .expect("set base");

    sync::pull(&handle, &data).expect("pull");

    let mirror_md = handle
        .mirror_dir
        .join("binder")
        .join(format!("{}.md", mirror::sanitize_path("Draft/CResolve")));

    let mut data = project::load_project_data(&handle).expect("reload2");
    if let Some(doc) = data.docs.get_mut(&id) {
        doc.content = "project-side-change".to_string();
    }
    project::save_project_data(&handle, &data).expect("save changed project");
    fs::write(&mirror_md, "mirror-side-change").expect("write mirror side");

    let mut data = project::load_project_data(&handle).expect("reload3");
    let first = sync::push(&handle, &mut data);
    assert!(matches!(first, Err(sync::SyncError::Conflict(_))));

    let target = ResolveTarget::new(Some(id), None).expect("target2");
    conflict::resolve(&handle, &target, "mirror", None).expect("resolve conflict");

    let mut data = project::load_project_data(&handle).expect("reload4");
    sync::push(&handle, &mut data).expect("second push should succeed");
}

#[test]
fn orphan_data_dir_removed_and_not_in_checksum() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");
    let mut data = project::load_project_data(&handle).expect("load");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdoc(d, "Draft/Check")?;
        Ok(())
    })
    .expect("mkdoc");

    let orphan = "AAAAAAAA-BBBB-4CCC-8DDD-EEEEEEEEEEEE";
    let orphan_dir = handle.scriv_dir.join("Files/Data").join(orphan);
    fs::create_dir_all(&orphan_dir).expect("mkdir orphan");
    fs::write(orphan_dir.join("content.rtf"), "{\\rtf1\\ansi orphan}").expect("write orphan");
    assert!(orphan_dir.exists());

    let mut data = project::load_project_data(&handle).expect("reload");
    let id = binder::resolve_id_by_path(&data, "Draft/Check").expect("resolve");
    let target = ResolveTarget::new(Some(id), None).expect("target");
    sync::with_write_through(&handle, &mut data, |d| {
        binder::set_synopsis(d, &target, "checksum trigger")?;
        Ok(())
    })
    .expect("trigger save");

    assert!(!orphan_dir.exists());
    let checksum = fs::read_to_string(handle.scriv_dir.join("Files/Data/docs.checksum")).expect("read checksum");
    assert!(!checksum.contains(&orphan.to_lowercase()));
}

#[test]
fn rich_text_content_is_preserved_on_metadata_only_saves() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");

    let mut data = project::load_project_data(&handle).expect("load");
    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdoc(d, "Draft/RichDoc")?;
        Ok(())
    })
    .expect("mkdoc");

    let data = project::load_project_data(&handle).expect("reload");
    let id = binder::resolve_id_by_path(&data, "Draft/RichDoc").expect("resolve rich doc");
    let data_dir = handle
        .scriv_dir
        .join("Files/Data")
        .join(id.to_string().to_uppercase());

    // Seed non-basic rich text content that should not be overwritten by metadata-only saves.
    fs::write(
        data_dir.join("content.rtf"),
        "{\\rtf1\\ansi\\ansicpg1252\\cocoartf2865\\n\\f0 some styled content}",
    )
    .expect("write rich rtf");

    let before = fs::read_to_string(data_dir.join("content.rtf")).expect("before read");
    let mut data = project::load_project_data(&handle).expect("load data");
    let target = ResolveTarget::new(Some(id), None).expect("target");
    sync::with_write_through(&handle, &mut data, |d| {
        binder::set_synopsis(d, &target, "metadata update only")?;
        Ok(())
    })
    .expect("metadata save");
    let after = fs::read_to_string(data_dir.join("content.rtf")).expect("after read");
    assert_eq!(before, after);
}
