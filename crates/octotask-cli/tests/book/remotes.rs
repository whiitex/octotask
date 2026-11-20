//! The fixture remotes of the book, reached as `git@github.com:acme/<name>.git`.
//! Their authors, dates and files are fixed, so their commits have the same hash on every run.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::sandbox::{NOW, Sandbox};

const AUTHOR_NAME: &str = "Alice";
const AUTHOR_EMAIL: &str = "alice@example.com";

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;

/// What happens in a repository, in order: commits on the current branch, new branches.
enum Step {
    /// A commit `age` seconds before `NOW`, writing each `(path, content)`.
    Commit(&'static str, u64, &'static [(&'static str, &'static str)]),
    /// A new branch from the current commit, which becomes the current branch.
    Branch(&'static str),
    Switch(&'static str),
}

struct Remote {
    name: &'static str,
    history: &'static [Step],
}

const REMOTES: [Remote; 3] = [
    Remote {
        name: "shop-frontend",
        history: &[
            Step::Commit(
                "Initial commit",
                21 * DAY,
                &[
                    ("README.md", "# shop-frontend\n\nThe web shop.\n"),
                    (
                        "package.json",
                        "{\n  \"name\": \"shop-frontend\",\n  \"scripts\": { \"dev\": \"vite\" }\n}\n",
                    ),
                ],
            ),
            Step::Commit(
                "Add product list",
                10 * DAY,
                &[("src/products.js", "export const products = [];\n")],
            ),
            Step::Branch("staging"),
            Step::Commit(
                "Add cart page",
                2 * DAY,
                &[("src/cart.js", "export const cart = [];\n")],
            ),
            Step::Branch("feat/login"),
            Step::Commit(
                "Add login form",
                4 * HOUR,
                &[("src/login.js", "export function login() {}\n")],
            ),
            Step::Switch("main"),
        ],
    },
    Remote {
        name: "shop-backend",
        history: &[
            Step::Commit(
                "Initial commit",
                21 * DAY,
                &[
                    ("README.md", "# shop-backend\n\nThe API of the shop.\n"),
                    ("main.py", "from shop import run\n\nrun(port=8000)\n"),
                ],
            ),
            Step::Commit(
                "Add orders API",
                10 * DAY,
                &[("orders.py", "def total(order):\n    return sum(order)\n")],
            ),
            Step::Branch("staging"),
            Step::Commit(
                "Fix order totals",
                5 * HOUR,
                &[(
                    "orders.py",
                    "def total(order):\n    return round(sum(order), 2)\n",
                )],
            ),
            Step::Branch("feat/login"),
            Step::Commit(
                "Add login endpoint",
                3 * HOUR,
                &[("auth.py", "def login(user):\n    return True\n")],
            ),
            Step::Switch("main"),
        ],
    },
    Remote {
        name: "shop-agent",
        history: &[
            Step::Commit(
                "Initial commit",
                21 * DAY,
                &[
                    ("README.md", "# shop-agent\n\nThe assistant of the shop.\n"),
                    ("agent.py", "PROMPT = open(\"prompt.txt\").read()\n"),
                ],
            ),
            Step::Branch("staging"),
            Step::Commit(
                "Tune the prompt",
                3 * DAY,
                &[(
                    "prompt.txt",
                    "You are the assistant of the shop. Answer every question about it.\n",
                )],
            ),
            Step::Branch("fix/prompt"),
            Step::Commit(
                "Shorten the prompt",
                6 * HOUR,
                &[("prompt.txt", "You are the assistant of the shop.\n")],
            ),
            Step::Switch("main"),
        ],
    },
];

/// The open pull requests, as the fake `gh` reads them: `(repo, number, title, branch)`.
const PULL_REQUESTS: [(&str, u64, &str, &str); 1] =
    [("shop-backend", 12, "Add login endpoint", "feat/login")];

/// Creates every remote, bare as on GitHub, and the pull requests of the fake `gh`.
pub fn create(sandbox: &Sandbox) {
    for remote in &REMOTES {
        let work = sandbox.work_dir().join(remote.name);
        fs::create_dir_all(&work).unwrap();
        git(sandbox, &work, &["init", "--quiet"]);
        for step in remote.history {
            apply(sandbox, &work, step);
        }
        let bare = sandbox.remote_dir(&format!("acme/{}.git", remote.name));
        git(
            sandbox,
            &work,
            &[
                "clone",
                "--quiet",
                "--bare",
                ".",
                &bare.display().to_string(),
            ],
        );
    }
    fs::remove_dir_all(sandbox.work_dir()).unwrap();
    for (repo, number, title, branch) in PULL_REQUESTS {
        write_pull_request(sandbox, repo, number, title, branch);
    }
}

fn apply(sandbox: &Sandbox, work: &Path, step: &Step) {
    match step {
        Step::Commit(message, age, files) => {
            for (path, content) in *files {
                let path = work.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, content).unwrap();
            }
            let date = format!("{} +0000", NOW - age);
            git(sandbox, work, &["add", "--all"]);
            run(sandbox_git(sandbox, work)
                .args(["commit", "--quiet", "--message", message])
                .envs([
                    ("GIT_AUTHOR_NAME", AUTHOR_NAME),
                    ("GIT_AUTHOR_EMAIL", AUTHOR_EMAIL),
                    ("GIT_COMMITTER_NAME", AUTHOR_NAME),
                    ("GIT_COMMITTER_EMAIL", AUTHOR_EMAIL),
                    ("GIT_AUTHOR_DATE", &date),
                    ("GIT_COMMITTER_DATE", &date),
                ]));
        }
        Step::Branch(branch) => git(sandbox, work, &["switch", "--quiet", "--create", branch]),
        Step::Switch(branch) => git(sandbox, work, &["switch", "--quiet", branch]),
    }
}

fn write_pull_request(sandbox: &Sandbox, repo: &str, number: u64, title: &str, branch: &str) {
    let dir = sandbox.pull_requests_dir().join(repo);
    fs::create_dir_all(&dir).unwrap();
    let pull_request = serde_json::json!({
        "number": number,
        "title": title,
        "headRefName": branch,
        "author": {"login": "alice"},
        "isCrossRepository": false,
    });
    fs::write(dir.join(format!("{number}.json")), pull_request.to_string()).unwrap();
}

fn git(sandbox: &Sandbox, dir: &Path, args: &[&str]) {
    run(sandbox_git(sandbox, dir).args(args));
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sandbox_git(sandbox: &Sandbox, dir: &Path) -> Command {
    let mut command = sandbox.command("git");
    command.current_dir(dir);
    command
}
