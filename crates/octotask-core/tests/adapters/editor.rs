use std::path::Path;

use octotask_core::{Editor, Location};

#[test]
fn every_editor_opens_a_local_folder_by_its_path() {
    for &editor in Editor::ALL {
        let args = editor.arguments(&Location::Local, Path::new("/ws/feat-login/backend"));
        assert_eq!(args, ["/ws/feat-login/backend"], "{editor:?}");
    }
}

#[test]
fn every_editor_opens_a_folder_of_another_machine_through_remote_ssh() {
    let location = Location::remote("vm", "/home/u/.octotask").unwrap();
    let path = Path::new("/home/u/remote user's home/feat-login/backend");
    for &editor in Editor::ALL {
        assert_eq!(
            editor.arguments(&location, path),
            [
                "--remote",
                "ssh-remote+vm",
                "/home/u/remote user's home/feat-login/backend"
            ],
            "{editor:?}"
        );
    }
}

#[test]
fn an_editor_opens_a_folder_of_a_user_at_another_machine_through_remote_ssh() {
    let location = Location::remote("user@host", "/home/user/.octotask").unwrap();
    let path = Path::new("/home/user/feat-login/backend");

    let args = Editor::named("code").unwrap().arguments(&location, path);

    assert_eq!(
        args,
        [
            "--remote",
            "ssh-remote+user@host",
            "/home/user/feat-login/backend"
        ]
    );
}

#[test]
fn an_editor_is_found_by_its_name_and_its_name_is_its_binary() {
    let names: Vec<&str> = Editor::ALL.iter().map(|editor| editor.name()).collect();
    assert_eq!(names, ["code", "cursor", "windsurf", "codium"]);
    assert_eq!(Editor::named("cursor").map(Editor::name), Some("cursor"));
    assert_eq!(Editor::named("zed"), None);
}
