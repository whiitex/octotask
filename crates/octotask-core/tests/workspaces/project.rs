use octotask_core::{Error, create_workspace, remove_project, remove_workspace};

use super::{Fixture, PROJECT};

#[test]
fn remove_project_deletes_the_project_file_and_its_caches() {
    let fx = Fixture::new();
    fx.register(&fx.remote("frontend"));
    create_workspace(&fx.registry(), PROJECT, "feat/login", None).unwrap();
    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    remove_project(&fx.registry(), PROJECT, false).unwrap();

    assert!(fx.registry().list_projects().unwrap().is_empty());
    assert!(!fx.home_dir().join("cache").join(PROJECT).exists());
    assert!(!fx.home_dir().join("workspaces").join(PROJECT).exists());
}

#[test]
fn remove_project_is_refused_while_it_has_workspaces() {
    let fx = Fixture::new();
    fx.register(&fx.remote("frontend"));
    create_workspace(&fx.registry(), PROJECT, "feat/login", None).unwrap();

    let err = remove_project(&fx.registry(), PROJECT, false).unwrap_err();

    assert!(
        matches!(&err, Error::ProjectInUse { project, workspaces }
            if project == PROJECT && workspaces == &["feat-login"]),
        "{err:?}"
    );
    assert!(fx.cache_dir("frontend").is_dir());
    assert_eq!(fx.registry().list_projects().unwrap(), [PROJECT]);
}

#[test]
fn remove_project_with_force_deletes_its_workspaces_too() {
    let fx = Fixture::new();
    fx.register(&fx.remote("frontend"));
    create_workspace(&fx.registry(), PROJECT, "feat/login", None).unwrap();

    remove_project(&fx.registry(), PROJECT, true).unwrap();

    assert!(fx.registry().list_projects().unwrap().is_empty());
    assert!(!fx.home_dir().join("cache").join(PROJECT).exists());
    assert!(!fx.home_dir().join("workspaces").join(PROJECT).exists());
}

#[test]
fn remove_project_that_does_not_exist_fails() {
    let fx = Fixture::new();

    let err = remove_project(&fx.registry(), "missing", false).unwrap_err();

    assert!(matches!(err, Error::ProjectNotFound(_)), "{err:?}");
}
