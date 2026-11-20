use std::path::{Path, PathBuf};

use octotask_core::{Error, Location};

fn remote(host: &str, root: &str) -> Location {
    Location::Remote {
        host: host.to_string(),
        root: PathBuf::from(root),
    }
}

#[test]
fn local_and_remote_locations_are_parsed() {
    assert_eq!("local".parse::<Location>().unwrap(), Location::Local);
    assert_eq!(
        "api-dev-1:/home/ubuntu/.octotask"
            .parse::<Location>()
            .unwrap(),
        remote("api-dev-1", "/home/ubuntu/.octotask")
    );
    assert_eq!(
        "ubuntu@10.0.0.5:/data/my octotask"
            .parse::<Location>()
            .unwrap(),
        remote("ubuntu@10.0.0.5", "/data/my octotask")
    );
}

#[test]
fn a_location_is_written_the_way_it_is_parsed() {
    for text in ["local", "api-dev-1:/home/ubuntu/.octotask"] {
        assert_eq!(text.parse::<Location>().unwrap().to_string(), text);
    }
}

#[test]
fn malformed_locations_are_rejected() {
    for text in [
        "",
        "api-dev-1",
        "api-dev-1:",
        "api-dev-1:relative/path",
        "api-dev-1:~/.octotask",
        ":/home/ubuntu",
        "-oProxyCommand=evil:/home/ubuntu",
        "two words:/home/ubuntu",
        "host;rm:/home/ubuntu",
        "api-dev-1:/home/ubuntu\n/.octotask",
    ] {
        let result = text.parse::<Location>();
        assert!(
            matches!(result, Err(Error::InvalidLocation { .. })),
            "should reject {text:?}: {result:?}"
        );
    }
}

#[test]
fn remote_paths_are_shown_with_their_host() {
    let path = Path::new("/home/ubuntu/.octotask/cache");
    assert_eq!(Location::Local.shown(path), path);
    assert_eq!(
        remote("api-dev-1", "/home/ubuntu/.octotask").shown(path),
        Path::new("api-dev-1:/home/ubuntu/.octotask/cache")
    );
}
