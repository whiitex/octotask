//! Tests for `src/error.rs`: what an error is for a program, next to its message for people.

use std::path::PathBuf;

use octotask_core::{Error, ErrorKind};
use serde_json::json;

#[test]
fn an_error_is_a_document_with_its_code_its_message_and_what_it_carries() {
    let error = Error::WorkspaceHasWork {
        name: "feat-login".to_string(),
        reasons: vec!["backend: uncommitted changes".to_string()],
    };

    assert_eq!(error.code(), "workspace_has_work");
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({
            "code": "workspace_has_work",
            "message": error.to_string(),
            "workspace": "feat-login",
            "reasons": ["backend: uncommitted changes"],
        })
    );
}

#[test]
fn an_error_with_one_value_names_it() {
    let error = Error::RepoNotFound("backend".to_string());

    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({
            "code": "repo_not_found",
            "message": "repo `backend` is not registered",
            "repo": "backend",
        })
    );
}

#[test]
fn an_error_that_carries_nothing_has_only_its_code_and_message() {
    let error = serde_json::to_value(Error::NoDefaultProject).unwrap();

    let names: Vec<&String> = error.as_object().unwrap().keys().collect();
    assert_eq!(names, ["code", "message"]);
    assert_eq!(error["code"], "no_default_project");
}

#[test]
fn a_path_is_written_as_text() {
    let error = Error::WorkspaceExists {
        name: "feat-login".to_string(),
        path: PathBuf::from("/home/workspaces/deeptree/feat-login"),
    };

    let document = serde_json::to_value(&error).unwrap();

    assert_eq!(document["path"], "/home/workspaces/deeptree/feat-login");
}

#[test]
fn every_class_of_failure_has_its_errors() {
    let text = |value: &str| value.to_string();
    let classified = [
        (Error::InvalidUrl(text("x")), ErrorKind::Usage),
        (Error::NoDefaultProject, ErrorKind::Usage),
        (Error::RepoNotFound(text("x")), ErrorKind::NotFound),
        (Error::WorkspaceNotFound(text("x")), ErrorKind::NotFound),
        (
            Error::WorkspaceHasWork {
                name: text("x"),
                reasons: Vec::new(),
            },
            ErrorKind::WorkAtRisk,
        ),
        (Error::ProjectExists(text("x")), ErrorKind::Conflict),
        (
            Error::RepoInUse {
                repo: text("x"),
                workspaces: Vec::new(),
            },
            ErrorKind::Conflict,
        ),
        (Error::ToolNotFound("git"), ErrorKind::Unreachable),
        (
            Error::Ssh {
                host: text("vm"),
                stderr: String::new(),
            },
            ErrorKind::Unreachable,
        ),
        (Error::NoHome, ErrorKind::Other),
    ];

    for (error, kind) in classified {
        assert_eq!(error.kind(), kind, "{error}");
    }
}
