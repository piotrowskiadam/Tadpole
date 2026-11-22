use std::path::{Path, PathBuf};
use tadpole::state::{CrawlResult, CrawlState};

fn sample_page(url: &str) -> CrawlResult {
    CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), 0, 0)
}

#[test]
fn project_save_defaults_use_tadpole_and_open_accepts_legacy_seocrawl() {
    assert_eq!(
        tadpole::project::DEFAULT_PROJECT_FILENAME,
        "project.tadpole"
    );
    assert_eq!(tadpole::project::SAVE_FILTER_PATTERNS, &["*.tadpole"]);
    assert_eq!(
        tadpole::project::OPEN_FILTER_PATTERNS,
        &["*.tadpole", "*.seocrawl"]
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("/tmp/site-audit")),
        PathBuf::from("/tmp/site-audit.tadpole")
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("/tmp/site-audit.tadpole")),
        PathBuf::from("/tmp/site-audit.tadpole")
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("/tmp/legacy.seocrawl")),
        PathBuf::from("/tmp/legacy.seocrawl")
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("/tmp/example.com")),
        PathBuf::from("/tmp/example.com.tadpole")
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("C:\\Audits\\client.co.uk")),
        PathBuf::from("C:\\Audits\\client.co.uk.tadpole")
    );
    assert_eq!(
        tadpole::project::ensure_project_save_path(Path::new("C:\\Audits\\SITE.TADPOLE")),
        PathBuf::from("C:\\Audits\\SITE.TADPOLE")
    );
}

#[test]
fn tadpole_and_legacy_seocrawl_projects_round_trip_and_validate() {
    let dir = tempfile::tempdir().unwrap();
    let state = CrawlState::new();
    state.insert_result(sample_page("https://example.com/"));
    let project = state.get_project("https://example.com/".into());

    let tadpole_path = dir.path().join("audit.tadpole");
    tadpole::project::save(&tadpole_path, &project).unwrap();
    let loaded_tadpole = tadpole::project::load(&tadpole_path).unwrap();
    assert_eq!(loaded_tadpole.seed_url, "https://example.com/");
    assert_eq!(loaded_tadpole.results.len(), 1);

    let legacy_path = dir.path().join("legacy.seocrawl");
    tadpole::project::save(&legacy_path, &project).unwrap();
    let loaded_legacy = tadpole::project::load(&legacy_path).unwrap();
    assert_eq!(loaded_legacy.seed_url, "https://example.com/");
    assert_eq!(loaded_legacy.results.len(), 1);

    let mut invalid = project;
    invalid.config.include_regex = Some("[invalid".into());
    let invalid_path = dir.path().join("invalid.tadpole");
    tadpole::project::save(&invalid_path, &invalid).unwrap();
    assert!(tadpole::project::load(&invalid_path).is_err());
}

#[test]
fn open_file_path_resolution_accepts_tadpole_and_seocrawl_case_insensitively() {
    assert!(tadpole::project::is_supported_project_path(Path::new(
        "/tmp/site.tadpole"
    )));
    assert!(tadpole::project::is_supported_project_path(Path::new(
        "C:\\Audits\\SITE.TADPOLE"
    )));
    assert!(tadpole::project::is_supported_project_path(Path::new(
        "/tmp/legacy.seocrawl"
    )));
    assert!(tadpole::project::is_supported_project_path(Path::new(
        "/tmp/legacy.SeoCrawl"
    )));
    assert!(!tadpole::project::is_supported_project_path(Path::new(
        "/tmp/notes.json"
    )));
    assert!(!tadpole::project::is_supported_project_path(Path::new(
        "/tmp/no_extension"
    )));

    let picked = tadpole::project::resolve_open_project_path([
        PathBuf::from("/tmp/readme.md"),
        PathBuf::from("/tmp/audit.tadpole"),
        PathBuf::from("/tmp/older.seocrawl"),
    ]);
    assert_eq!(picked, Some(PathBuf::from("/tmp/audit.tadpole")));

    let legacy_picked = tadpole::project::resolve_open_project_path([
        PathBuf::from("/tmp/notes.txt"),
        PathBuf::from("/tmp/older.seocrawl"),
    ]);
    assert_eq!(legacy_picked, Some(PathBuf::from("/tmp/older.seocrawl")));

    let none_picked = tadpole::project::resolve_open_project_path([
        PathBuf::from("/tmp/notes.txt"),
        PathBuf::from("/tmp/data.json"),
    ]);
    assert_eq!(none_picked, None);
}

#[test]
fn open_guard_blocks_active_crawl_confirms_replacement_and_allows_empty_workspace() {
    use tadpole::project::{OpenGuard, open_guard};

    assert_eq!(open_guard(false, false, 0), OpenGuard::Proceed);
    assert_eq!(
        open_guard(false, false, 12),
        OpenGuard::ConfirmReplace { existing_urls: 12 }
    );
    assert_eq!(open_guard(true, false, 0), OpenGuard::BlockedByActiveCrawl);
    assert_eq!(open_guard(true, false, 5), OpenGuard::BlockedByActiveCrawl);
    assert_eq!(open_guard(false, true, 5), OpenGuard::BlockedByActiveCrawl);
}

#[test]
fn slug_filenames_avoid_windows_reserved_device_names() {
    for reserved in [
        "con", "CON", "prn", "aux", "nul", "com1", "COM9", "lpt1", "LPT9",
    ] {
        let url = format!("https://example.com/{reserved}");
        let slug = tadpole::crawler::get_slug_filename(&url);
        assert_ne!(
            slug.to_ascii_lowercase(),
            reserved.to_ascii_lowercase(),
            "slug for {url} must not be Windows reserved device name {reserved}"
        );
        assert!(
            slug.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
    assert_eq!(
        tadpole::crawler::get_slug_filename("https://example.com/blog/seo-guide.html"),
        "seo-guide"
    );
}

#[test]
fn saving_over_existing_tadpole_project_replaces_atomically_without_stray_temp_files() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("Client Audits");
    std::fs::create_dir_all(&nested).unwrap();
    let target = nested.join("example.com.tadpole");

    let first_state = CrawlState::new();
    first_state.insert_result(sample_page("https://example.com/first"));
    let first_project = first_state.get_project("https://example.com/".into());
    tadpole::project::save(&target, &first_project).unwrap();

    let second_state = CrawlState::new();
    second_state.insert_result(sample_page("https://example.com/second"));
    second_state.insert_result(sample_page("https://example.com/third"));
    let second_project = second_state.get_project("https://example.com/updated".into());
    tadpole::project::save(&target, &second_project).unwrap();

    let loaded = tadpole::project::load(&target).unwrap();
    assert_eq!(loaded.seed_url, "https://example.com/updated");
    assert_eq!(loaded.results.len(), 2);

    let entries: Vec<_> = std::fs::read_dir(&nested)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        entries,
        vec![std::ffi::OsString::from("example.com.tadpole")]
    );
}

#[test]
fn document_portal_save_paths_keep_exact_portal_filename() {
    let portal_without_ext = Path::new("/run/user/1000/doc/1a2b3c4d/site-audit");
    assert_eq!(
        tadpole::project::ensure_project_save_path(portal_without_ext),
        PathBuf::from("/run/user/1000/doc/1a2b3c4d/site-audit")
    );
    let portal_with_ext = Path::new("/run/user/1000/doc/1a2b3c4d/site-audit.tadpole");
    assert_eq!(
        tadpole::project::ensure_project_save_path(portal_with_ext),
        PathBuf::from("/run/user/1000/doc/1a2b3c4d/site-audit.tadpole")
    );
}

#[cfg(unix)]
#[test]
fn save_falls_back_to_direct_write_when_parent_forbids_sibling_temp_files() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("portal.tadpole");
    std::fs::write(&target, b"{}").unwrap();

    // Lock the parent directory against sibling temp-file creation (like a single-file portal mount)
    // while keeping the target file itself writable.
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

    let state = CrawlState::new();
    state.insert_result(sample_page("https://example.com/portal"));
    let project = state.get_project("https://example.com/".into());
    let save_result = tadpole::project::save(&target, &project);

    // Restore permissions before asserting so tempdir cleanup always succeeds.
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

    save_result.expect("direct write fallback should succeed when target file is writable");
    let loaded = tadpole::project::load(&target).unwrap();
    assert_eq!(loaded.seed_url, "https://example.com/");
    assert_eq!(loaded.results.len(), 1);
}
