//! Tests for what runs on another machine, through the fake `ssh` of `tests/fixtures`.
//! Each test runs in a child process, the only one that has the fake `ssh` on its `PATH`.

use std::fs;
use std::path::PathBuf;

use octotask_core::{Error, Git, Location, Ssh, resolve_location};

use super::{FakeRemote, commit_empty, fake_remote_path, init_repo, is_child};

fn remote_home() -> PathBuf {
    fake_remote_path(FakeRemote::HOME)
}

fn vm() -> Location {
    Location::remote("vm", remote_home()).unwrap()
}

fn ssh_log() -> String {
    fs::read_to_string(fake_remote_path(FakeRemote::LOG)).unwrap_or_default()
}

#[test]
fn git_runs_on_the_remote_machine_with_its_arguments_intact() {
    if is_child() {
        let repo = remote_home().join("it's a \"repo\"");
        fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);
        commit_empty(&repo);
        let git = Git::at(&vm(), &repo);

        let top_level = git.run(&["rev-parse", "--show-toplevel"]).unwrap();
        assert_eq!(top_level.trim_end(), repo.display().to_string());
        let subject = git
            .run(&["log", "-1", "--format=%s: it's $HOME; \"x\" `echo` *"])
            .unwrap();
        assert_eq!(subject, "init: it's $HOME; \"x\" `echo` *\n");
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "git_runs_on_the_remote_machine_with_its_arguments_intact",
    );
}

#[test]
fn exit_codes_of_the_remote_command_come_back() {
    if is_child() {
        let repo = remote_home().join("repo");
        fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);
        let git = Git::at(&vm(), &repo);
        let missing_ref = ["show-ref", "--verify", "--quiet", "refs/heads/nope"];
        assert!(!git.check(&missing_ref).unwrap());

        let missing_dir = remote_home().join("missing");
        let err = Git::at(&vm(), &missing_dir)
            .check(&missing_ref)
            .unwrap_err();
        assert!(
            matches!(&err, Error::CommandFailed { status, .. } if status.code() == Some(128)),
            "{err:?}"
        );
        let shown = format!("failed in vm:{}", missing_dir.display());
        assert!(err.to_string().contains(&shown), "{err}");
        return;
    }
    FakeRemote::new().run_in_child(module_path!(), "exit_codes_of_the_remote_command_come_back");
}

#[test]
fn a_host_that_cannot_be_reached_is_reported_with_a_hint() {
    if is_child() {
        let unreachable = Location::remote("unreachable", "/srv/octotask").unwrap();
        let git = Git::at(&unreachable, "/srv/octotask/repo");

        for err in [
            git.run(&["status"]).unwrap_err(),
            git.check(&["status"]).unwrap_err(),
        ] {
            assert!(
                matches!(&err, Error::Ssh { host, .. } if host == "unreachable"),
                "{err:?}"
            );
            let message = err.to_string();
            assert!(message.contains("Connection refused"), "{message}");
            assert!(
                message.contains("check that `ssh unreachable` connects by itself"),
                "{message}"
            );
            assert!(
                message.contains("`octok set-ssh-passphrase unreachable`"),
                "{message}"
            );
        }
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "a_host_that_cannot_be_reached_is_reported_with_a_hint",
    );
}

#[test]
fn ssh_shares_one_connection_and_never_asks_without_a_terminal() {
    if is_child() {
        let _ = Git::at(&vm(), remote_home()).run(&["--version"]).unwrap();

        let log = ssh_log();
        for expected in [
            "-o ControlMaster=auto",
            "-o ControlPersist=60",
            "-o BatchMode=yes",
            "-- vm env GIT_TERMINAL_PROMPT=0 git -C ",
        ] {
            assert!(log.contains(expected), "missing {expected:?} in: {log}");
        }
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "ssh_shares_one_connection_and_never_asks_without_a_terminal",
    );
}

#[test]
fn a_location_without_a_path_is_the_octok_home_of_the_remote_user() {
    if is_child() {
        let home = remote_home();
        assert_eq!(resolve_location("local").unwrap(), Location::Local);

        let cases = [
            ("vm".to_string(), home.join(".octotask")),
            ("vm:".to_string(), home.join(".octotask")),
            ("vm:~/custom/octok".to_string(), home.join("custom/octok")),
            ("vm:custom two".to_string(), home.join("custom two")),
            (
                format!("vm:{}", home.join("absolute").display()),
                home.join("absolute"),
            ),
        ];
        for (input, root) in cases {
            let location = resolve_location(&input).unwrap();
            assert_eq!(location, Location::remote("vm", &root).unwrap(), "{input}");
            assert!(root.is_dir(), "{input} did not create {}", root.display());
        }
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "a_location_without_a_path_is_the_octok_home_of_the_remote_user",
    );
}

#[test]
fn a_location_that_cannot_be_used_is_refused() {
    if is_child() {
        let err = resolve_location("-oProxyCommand=evil").unwrap_err();
        assert!(matches!(err, Error::InvalidLocation { .. }), "{err:?}");
        assert_eq!(ssh_log(), "", "ssh must not run for an invalid host");

        let err = resolve_location("unreachable").unwrap_err();
        assert!(matches!(err, Error::Ssh { .. }), "{err:?}");
        return;
    }
    FakeRemote::new().run_in_child(module_path!(), "a_location_that_cannot_be_used_is_refused");
}

#[test]
fn remembering_the_passphrase_adds_the_keys_of_the_host_to_the_agent() {
    if is_child() {
        let local_home = fake_remote_path("HOME");
        let in_home = local_home.join(".ssh/id_test");
        let elsewhere = local_home.join("keys/second key");
        for key in [&in_home, &elsewhere] {
            fs::create_dir_all(key.parent().unwrap()).unwrap();
            fs::write(key, "a private key").unwrap();
        }
        let config = format!(
            "hostname 10.0.0.5\nuser ubuntu\nidentityfile ~/.ssh/id_test\nidentityfile {}\nidentityfile ~/.ssh/missing\n",
            elsewhere.display()
        );
        fs::write(fake_remote_path(FakeRemote::CONFIG), config).unwrap();

        let keys = Ssh::new("vm").unwrap().remember_passphrase().unwrap();

        assert_eq!(keys, [in_home.clone(), elsewhere.clone()]);
        let added = fs::read_to_string(fake_remote_path(FakeRemote::ADD_LOG)).unwrap();
        let keychain = if cfg!(target_os = "macos") {
            "--apple-use-keychain "
        } else {
            ""
        };
        assert_eq!(
            added,
            format!("{keychain}{} {}\n", in_home.display(), elsewhere.display())
        );
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "remembering_the_passphrase_adds_the_keys_of_the_host_to_the_agent",
    );
}

#[test]
fn a_host_without_key_files_has_no_passphrase_to_remember() {
    if is_child() {
        fs::write(
            fake_remote_path(FakeRemote::CONFIG),
            "hostname 10.0.0.5\nidentityfile ~/.ssh/missing\n",
        )
        .unwrap();

        let keys = Ssh::new("vm").unwrap().remember_passphrase().unwrap();

        assert!(keys.is_empty(), "{keys:?}");
        assert!(!fake_remote_path(FakeRemote::ADD_LOG).exists());
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "a_host_without_key_files_has_no_passphrase_to_remember",
    );
}

#[test]
fn settings_are_what_ssh_resolves_for_the_host() {
    if is_child() {
        let key = fake_remote_path("HOME").join(".ssh/id_test");
        fs::create_dir_all(key.parent().unwrap()).unwrap();
        fs::write(&key, "a private key").unwrap();
        fs::write(
            fake_remote_path(FakeRemote::CONFIG),
            "hostname 10.0.0.5\nuser ubuntu\nport 2222\nidentityfile ~/.ssh/id_test\nidentityfile ~/.ssh/missing\n",
        )
        .unwrap();

        let settings = Ssh::new("vm").unwrap().settings().unwrap();

        assert_eq!(settings.hostname, "10.0.0.5");
        assert_eq!(settings.user, "ubuntu");
        assert_eq!(settings.port, "2222");
        assert_eq!(settings.keys, [key]);
        return;
    }
    FakeRemote::new().run_in_child(
        module_path!(),
        "settings_are_what_ssh_resolves_for_the_host",
    );
}
