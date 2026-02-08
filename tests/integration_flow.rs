use std::fs;

use scriv::{binder, mirror, project, sync};

#[test]
fn end_to_end_write_through_and_push() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");
    let mut data = project::load_project_data(&handle).expect("load");

    sync::pull(&handle, &data).expect("initial pull");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdoc(d, "Draft/Chapter1")?;
        Ok(())
    })
    .expect("create doc");

    let mut data = project::load_project_data(&handle).expect("reload");
    let id = binder::resolve_id_by_path(&data, "Draft/Chapter1").expect("resolve");
    let target = scriv::types::ResolveTarget::new(Some(id), None).expect("target");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::set_content(d, &target, "chapter text")?;
        binder::set_notes(d, &target, "note")?;
        binder::set_synopsis(d, &target, "syn")?;
        Ok(())
    })
    .expect("write content");

    let mirror_md = handle
        .mirror_dir
        .join("binder")
        .join(format!("{}.md", mirror::sanitize_path("Draft/Chapter1")));
    let mirror_meta = handle.mirror_dir.join("binder").join(format!(
        "{}.meta.yml",
        mirror::sanitize_path("Draft/Chapter1")
    ));
    assert_eq!(
        fs::read_to_string(&mirror_md).expect("md read"),
        "chapter text"
    );
    let meta_text = fs::read_to_string(&mirror_meta).expect("meta read");
    assert!(meta_text.contains("notes: note"));
    assert!(meta_text.contains("synopsis: syn"));

    fs::write(&mirror_md, "edited in mirror").expect("write mirror");
    let mut data = project::load_project_data(&handle).expect("reload2");
    sync::push(&handle, &mut data).expect("push mirror change");

    let data = project::load_project_data(&handle).expect("reload3");
    let doc = data.docs.get(&id).expect("doc exists");
    assert_eq!(doc.content, "edited in mirror");
}

#[test]
fn conflict_is_reported() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let handle =
        project::create_project("Novel", Some(tmp.path().to_path_buf()), "blank").expect("create");
    let mut data = project::load_project_data(&handle).expect("load");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::mkdoc(d, "Draft/Chapter2")?;
        Ok(())
    })
    .expect("mkdoc");

    let mut data = project::load_project_data(&handle).expect("reload");
    let id = binder::resolve_id_by_path(&data, "Draft/Chapter2").expect("resolve");
    let target = scriv::types::ResolveTarget::new(Some(id), None).expect("target");

    sync::with_write_through(&handle, &mut data, |d| {
        binder::set_content(d, &target, "base")?;
        Ok(())
    })
    .expect("set base");

    let mut data = project::load_project_data(&handle).expect("reload2");
    if let Some(doc) = data.docs.get_mut(&id) {
        doc.content = "project-changed".to_string();
    }
    project::save_project_data(&handle, &data).expect("save project changed");

    let mirror_md = handle
        .mirror_dir
        .join("binder")
        .join(format!("{}.md", mirror::sanitize_path("Draft/Chapter2")));
    fs::write(&mirror_md, "mirror-changed").expect("write mirror changed");

    let mut data = project::load_project_data(&handle).expect("reload3");
    let res = sync::push(&handle, &mut data);
    assert!(matches!(res, Err(sync::SyncError::Conflict(_))));
}
