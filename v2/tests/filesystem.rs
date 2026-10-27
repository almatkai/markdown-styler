use tezbar_markdown_styler::{self as core, temp};
#[test]
fn private_exclusive_documents_raii_cleanup_and_literal_urls() {
    let root = tempfile::tempdir().unwrap();
    let p = temp::create(root.path(), "a document", false).unwrap();
    let a = p.artifact.clone();
    assert_eq!(std::fs::read_to_string(&a.path).unwrap(), "a document");
    assert_eq!(std::fs::read_dir(&a.directory).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&a.directory)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&a.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    drop(p);
    assert!(!a.path.exists());
    let a = temp::create(root.path(), "second", false).unwrap().commit();
    let b = temp::create(root.path(), "second", false).unwrap().commit();
    assert_ne!(a.path, b.path);
    let u = temp::file_url(&a).unwrap();
    assert_eq!(url::Url::parse(&u).unwrap().to_file_path().unwrap(), a.path);
    // Foreign entries are never recursively removed by committed cleanup.
    let foreign = a.directory.join(".stage-recreated");
    std::fs::write(&foreign, "unrelated").unwrap();
    assert!(temp::cleanup(&a).is_err());
    assert_eq!(std::fs::read_to_string(foreign).unwrap(), "unrelated");
    temp::cleanup(&b).unwrap();
}
#[test]
fn missing_altered_symlink_and_identity_replacement_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let a = temp::create(root.path(), "original", false)
        .unwrap()
        .commit();
    std::fs::write(&a.path, "altered!").unwrap();
    assert!(temp::verify(&a).is_err());
    assert!(temp::cleanup(&a).is_err());
    assert!(a.path.exists());
    let a = temp::create(root.path(), "same", false).unwrap().commit();
    let old = a.directory.join("moved");
    std::fs::rename(&a.path, &old).unwrap();
    std::fs::write(&a.path, "same").unwrap();
    assert!(temp::verify(&a).is_err());
    assert!(temp::cleanup(&a).is_err());
    #[cfg(unix)]
    {
        let a = temp::create(root.path(), "link", false).unwrap().commit();
        let dest = root.path().join("unrelated");
        std::fs::write(&dest, "link").unwrap();
        std::fs::remove_file(&a.path).unwrap();
        std::os::unix::fs::symlink(&dest, &a.path).unwrap();
        assert!(temp::verify(&a).is_err());
        assert!(temp::cleanup(&a).is_err());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "link");
    }
    assert!(temp::create(&root.path().join("absent"), "x", false).is_err());
    assert!(temp::create(root.path(), &"x".repeat(core::MAX_OUTPUT_BYTES + 1), false).is_err());
}
#[test]
fn input_output_work_depth_and_utf16_budgets() {
    assert!(core::convert(&"x".repeat(core::MAX_INPUT_BYTES + 1)).is_err());
    assert!(core::render::styled(&format!("{}x", "> ".repeat(core::MAX_DEPTH + 1))).is_err());
    assert!(core::render::styled(&"**x** ".repeat(30_000)).is_err());
    let c = core::convert(&format!("{}😀tail", "x".repeat(299))).unwrap();
    assert_eq!(c.preview_utf16.len(), 300);
    assert_eq!(*c.preview_utf16.last().unwrap(), 0xd83d);
    assert!(c.display("t", "i").ends_with("\\uD83D..."));
    assert!(!c.display("t", "i").contains('\u{fffd}'));
    for ws in ["\u{feff}", "\u{a0}", "\u{2029}"] {
        assert_eq!(core::js_trim(&format!("{ws}abc{ws}")), "abc")
    }
    assert_eq!(core::js_trim("\u{85}abc\u{85}"), "\u{85}abc\u{85}");
}

#[test]
fn retired_staging_path_is_disarmed_after_publish_even_when_pending_is_dropped() {
    let root = tempfile::tempdir().unwrap();
    for commit in [true, false] {
        let p = temp::create(root.path(), "owned", false).unwrap();
        let retired = p.retired_staging_path.clone();
        let output = p.artifact.clone();
        std::fs::write(&retired, "new unrelated file").unwrap();
        if commit {
            let a = p.commit();
            assert!(temp::cleanup(&a).is_err());
        } else {
            drop(p);
        }
        assert_eq!(
            std::fs::read_to_string(retired).unwrap(),
            "new unrelated file"
        );
        assert!(!output.path.exists());
    }
}

#[test]
fn cleanup_rejects_replaced_directory_when_document_was_removed() {
    let root = tempfile::tempdir().unwrap();
    let a = temp::create(root.path(), "owned", false).unwrap().commit();
    let moved = root.path().join("moved-namespace");
    std::fs::rename(&a.directory, &moved).unwrap();
    std::fs::create_dir(&a.directory).unwrap();
    assert!(temp::cleanup(&a).is_err());
    assert!(a.directory.is_dir());
    assert!(moved.join("md-styler.html").is_file());
}
