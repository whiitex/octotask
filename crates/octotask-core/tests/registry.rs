use std::fs;

use octotask_core::{Error, Home, Registry};
use tempfile::TempDir;

fn registry() -> (TempDir, Registry) {
    let dir = TempDir::new().unwrap();
    let reg = Registry::new(Home::new(dir.path().join("octotask-home")));
    (dir, reg)
}

#[test]
fn init_writes_project_file_with_expected_yaml_shape() {
    let (dir, reg) = registry();
    let mut p = reg.init_project("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    p.add_repo("git@github.com:org/agent.git", None, Some("main"))
        .unwrap();
    reg.save_project(&p).unwrap();

    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    let raw = fs::read_to_string(&path).unwrap();
    // Parse independently of octotask-core to check the on-disk contract.
    let doc: serde_saphyr_check::Doc = serde_saphyr_check::parse(&raw);
    assert_eq!(doc.version, 1);
    assert_eq!(doc.name, "deeptree");
    assert_eq!(doc.base, "develop");
    assert_eq!(doc.repos["frontend"].url, "git@github.com:org/frontend.git");
    assert_eq!(doc.repos["frontend"].base, None);
    assert_eq!(doc.repos["agent"].base.as_deref(), Some("main"));
}

#[test]
fn saved_project_round_trips() {
    let (_dir, reg) = registry();
    let mut p = reg.init_project("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    p.add_repo("git@github.com:org/backend.git", None, None)
        .unwrap();
    reg.save_project(&p).unwrap();

    let loaded = reg.load_project("deeptree").unwrap();
    let keys: Vec<&str> = loaded.repos.keys().map(String::as_str).collect();
    assert_eq!(keys, ["frontend", "backend"]);
    assert_eq!(loaded.base, "develop");
}

#[test]
fn init_twice_fails_and_keeps_existing_file() {
    let (_dir, reg) = registry();
    let mut p = reg.init_project("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    reg.save_project(&p).unwrap();

    assert!(
        matches!(reg.init_project("deeptree", "main"), Err(Error::ProjectExists(n)) if n == "deeptree")
    );
    assert_eq!(reg.load_project("deeptree").unwrap().repos.len(), 1);
}

#[test]
fn init_makes_the_new_project_the_default() {
    let (_dir, reg) = registry();
    assert_eq!(reg.default_project().unwrap(), None);
    reg.init_project("deeptree", "develop").unwrap();
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("deeptree"));
    reg.init_project("side", "main").unwrap();
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("side"));
}

#[test]
fn failed_init_keeps_the_current_default() {
    let (_dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    reg.init_project("side", "main").unwrap();
    assert!(reg.init_project("deeptree", "main").is_err());
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("side"));
}

#[test]
fn remove_deletes_project_and_keeps_default_of_another_project() {
    let (_dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    reg.init_project("side", "main").unwrap();
    reg.remove_project("deeptree").unwrap();
    assert_eq!(reg.list_projects().unwrap(), ["side"]);
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("side"));
}

#[test]
fn remove_clears_default_when_it_pointed_to_the_removed_project() {
    let (_dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    reg.init_project("side", "main").unwrap();
    reg.remove_project("side").unwrap();
    assert_eq!(reg.list_projects().unwrap(), ["deeptree"]);
    assert_eq!(reg.default_project().unwrap(), None);
    assert!(matches!(
        reg.resolve_project(None),
        Err(Error::NoDefaultProject)
    ));
}

#[test]
fn remove_missing_project_is_not_found() {
    let (_dir, reg) = registry();
    assert!(matches!(reg.remove_project("nope"), Err(Error::ProjectNotFound(n)) if n == "nope"));
}

#[test]
fn remove_rejects_name_escaping_the_projects_dir() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    assert!(matches!(
        reg.remove_project("../config"),
        Err(Error::InvalidName { .. })
    ));
    assert!(dir.path().join("octotask-home/config.yaml").exists());
}

#[test]
fn remove_works_on_an_unparseable_project_file() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    fs::write(&path, "not: [valid").unwrap();
    reg.remove_project("deeptree").unwrap();
    assert!(!path.exists());
}

#[test]
fn global_config_is_written_with_version_1() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let raw = fs::read_to_string(dir.path().join("octotask-home/config.yaml")).unwrap();
    let doc: serde_saphyr_check::Global = serde_saphyr::from_str(&raw).unwrap();
    assert_eq!(doc.version, 1);
    assert_eq!(doc.default_project.as_deref(), Some("deeptree"));
}

#[test]
fn set_default_requires_existing_project() {
    let (_dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    reg.init_project("side", "main").unwrap();
    reg.set_default_project("deeptree").unwrap();
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("deeptree"));
    assert!(
        matches!(reg.set_default_project("nope"), Err(Error::ProjectNotFound(n)) if n == "nope")
    );
    assert_eq!(reg.default_project().unwrap().as_deref(), Some("deeptree"));
}

#[test]
fn resolve_prefers_explicit_then_default() {
    let (_dir, reg) = registry();
    assert!(matches!(
        reg.resolve_project(None),
        Err(Error::NoDefaultProject)
    ));
    reg.init_project("deeptree", "develop").unwrap();
    reg.init_project("side", "main").unwrap();
    assert_eq!(reg.resolve_project(None).unwrap(), "side");
    assert_eq!(reg.resolve_project(Some("deeptree")).unwrap(), "deeptree");
    assert!(matches!(
        reg.resolve_project(Some("nope")),
        Err(Error::ProjectNotFound(_))
    ));
}

#[test]
fn list_projects_is_sorted_and_ignores_other_files() {
    let (dir, reg) = registry();
    assert!(reg.list_projects().unwrap().is_empty());
    reg.init_project("zeta", "main").unwrap();
    reg.init_project("alpha", "main").unwrap();
    fs::write(dir.path().join("octotask-home/projects/notes.txt"), "x").unwrap();
    assert_eq!(reg.list_projects().unwrap(), ["alpha", "zeta"]);
}

#[test]
fn load_missing_project_is_not_found() {
    let (_dir, reg) = registry();
    assert!(matches!(reg.load_project("nope"), Err(Error::ProjectNotFound(n)) if n == "nope"));
}

#[test]
fn load_rejects_unknown_fields_instead_of_dropping_them() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    fs::write(
        &path,
        "version: 1\nname: deeptree\nbase: develop\nrepos: {}\nservices: {}\n",
    )
    .unwrap();
    assert!(matches!(
        reg.load_project("deeptree"),
        Err(Error::Parse { .. })
    ));
}

#[test]
fn load_rejects_unsupported_version() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    fs::write(&path, "version: 2\nname: deeptree\nbase: develop\n").unwrap();
    assert!(matches!(
        reg.load_project("deeptree"),
        Err(Error::UnsupportedVersion { version: 2, .. })
    ));
}

#[test]
fn load_rejects_name_that_does_not_match_file() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    fs::write(&path, "version: 1\nname: other\nbase: develop\n").unwrap();
    assert!(matches!(
        reg.load_project("deeptree"),
        Err(Error::NameMismatch { .. })
    ));
}

#[test]
fn load_rejects_hand_edited_invalid_repo_url() {
    let (dir, reg) = registry();
    reg.init_project("deeptree", "develop").unwrap();
    let path = dir.path().join("octotask-home/projects/deeptree.yaml");
    fs::write(
        &path,
        "version: 1\nname: deeptree\nbase: develop\nrepos:\n  fe:\n    url: not-a-url\n",
    )
    .unwrap();
    assert!(matches!(
        reg.load_project("deeptree"),
        Err(Error::InvalidUrl(_))
    ));
}

#[test]
fn save_leaves_no_temp_files_behind() {
    let (dir, reg) = registry();
    let p = reg.init_project("deeptree", "develop").unwrap();
    reg.save_project(&p).unwrap();
    let names: Vec<String> = fs::read_dir(dir.path().join("octotask-home/projects"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["deeptree.yaml"]);
}

/// Minimal independent view of the on-disk format, so the shape test
/// does not go through octotask-core's own (de)serializer types.
mod serde_saphyr_check {
    use std::collections::HashMap;

    #[derive(serde::Deserialize)]
    pub struct Doc {
        pub version: u32,
        pub name: String,
        pub base: String,
        pub repos: HashMap<String, Repo>,
    }

    #[derive(serde::Deserialize)]
    pub struct Repo {
        pub url: String,
        pub base: Option<String>,
    }

    #[derive(serde::Deserialize)]
    pub struct Global {
        pub version: u32,
        pub default_project: Option<String>,
    }

    pub fn parse(raw: &str) -> Doc {
        serde_saphyr::from_str(raw).unwrap()
    }
}
