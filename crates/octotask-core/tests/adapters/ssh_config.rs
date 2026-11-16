//! Each test runs in a child process, the only one whose `HOME` holds the ssh config under test.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use octotask_core::ssh_config_hosts;
use tempfile::TempDir;

use super::{is_child, run_in_child};

fn ssh_dir() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap()).join(".ssh")
}

fn run_with_home(test_name: &str) {
    let home = TempDir::new().unwrap();
    let vars: [(&str, &OsStr); 1] = [("HOME", home.path().as_os_str())];
    run_in_child(module_path!(), test_name, &vars);
}

#[test]
fn hosts_are_the_names_the_ssh_config_lists_one_by_one() {
    if is_child() {
        fs::create_dir_all(ssh_dir()).unwrap();
        fs::write(
            ssh_dir().join("config"),
            "# Host commented-out\n\
             Host api-dev-1\n\
             \x20   HostName 34.154.66.2\n\
             \x20   User pasquale\n\
             host bastion \"db 1\" db-2\n\
             Host=equals-sign\n\
             Host *.internal !secret git?hub\n\
             Host *\n\
             \x20   ServerAliveInterval 60\n\
             Match host other\n\
             Host api-dev-1\n",
        )
        .unwrap();

        assert_eq!(
            ssh_config_hosts(),
            ["api-dev-1", "bastion", "db-2", "equals-sign"]
        );
        return;
    }
    run_with_home("hosts_are_the_names_the_ssh_config_lists_one_by_one");
}

#[test]
fn hosts_of_included_files_are_listed_too() {
    if is_child() {
        let elsewhere = ssh_dir().parent().unwrap().join("tools/ssh");
        fs::create_dir_all(ssh_dir().join("config.d")).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(
            ssh_dir().join("config"),
            "Include ~/tools/ssh/config config.d/*.conf missing\nHost last\n",
        )
        .unwrap();
        fs::write(elsewhere.join("config"), "Host from-tool\n").unwrap();
        fs::write(ssh_dir().join("config.d/b.conf"), "Host second\n").unwrap();
        fs::write(ssh_dir().join("config.d/a.conf"), "Host first\n").unwrap();
        fs::write(ssh_dir().join("config.d/notes.txt"), "Host ignored\n").unwrap();

        assert_eq!(ssh_config_hosts(), ["from-tool", "first", "second", "last"]);
        return;
    }
    run_with_home("hosts_of_included_files_are_listed_too");
}

#[test]
fn no_ssh_config_means_no_host() {
    if is_child() {
        assert!(ssh_config_hosts().is_empty());
        return;
    }
    run_with_home("no_ssh_config_means_no_host");
}
