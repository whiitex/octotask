use std::path::{Component, Path, PathBuf};

use super::repos::dev_store;
use super::workspace::{chosen_keys, members_of_existing};
use crate::adapters::{Files, run_script};
use crate::{Error, Home, Location, Project, Result};

/// Keeps the dev changes of one repo in the folder `$2`, on the machine of the repo.
/// `$1` is what to do, and the rest depends on it.
///
/// The folder holds `dev/changed/<path>` and `base/<path>`, each tracked file with dev
/// changes and the version git had of it, and `dev/new/<path>`, the files git does not
/// track.
///
/// git itself keeps the dev changes of the tracked files in every checkout and out of
/// every commit: a filter puts them in when it writes a file, and takes them out when it
/// reads one. The filter is the three-way merge of `filter.sh`, so a file can change in
/// git and still carry its dev changes, as long as the two do not touch the same lines.
const SCRIPT: &str = r##"
action=$1 store=$2
shift 2

# Paths of the files saved under `$store/$1`.
stored() {
    [ ! -d "$store/$1" ] || (cd "$store/$1" && find . -type f | sed 's|^\./||' | LC_ALL=C sort)
}

# Replaces the block octok keeps in the file `$1` with the lines on stdin.
rewrite_block() {
    {
        [ ! -f "$1" ] || sed '/^# octok dev changes: begin$/,/^# octok dev changes: end$/d' "$1"
        lines=$(cat)
        [ -z "$lines" ] || printf '# octok dev changes: begin\n%s\n# octok dev changes: end\n' "$lines"
    } >"$1.octok" && mv "$1.octok" "$1"
}

# Tells the repo of the current directory about what is saved, for all its checkouts:
# which files go through the filter, and which new files git must ignore.
install() {
    common=$(git rev-parse --git-common-dir) && mkdir -p "$common/info" || exit
    stored dev/changed | sed 's/[\\"]/\\&/g; s/.*/"&" filter=octok-dev/' |
        rewrite_block "$common/info/attributes" || exit
    stored dev/new | sed 's|^|/|' | rewrite_block "$common/info/exclude" || exit
    if [ -d "$store/dev/changed" ]; then
        filter=$(printf '%s' "$store/filter.sh" | sed "s/'/'\\\\''/g")
        git config filter.octok-dev.clean "sh '$filter' clean %f" &&
            git config filter.octok-dev.smudge "sh '$filter' smudge %f" &&
            git config filter.octok-dev.required true || exit
    else
        git config --remove-section filter.octok-dev 2>/dev/null
    fi
}

case $action in
save)
    # Saves what the checkout `$1` has and git does not: `$2` is the text of `filter.sh`,
    # the rest are files git ignores, to save all the same.
    checkout=$1 filter_script=$2
    shift 2
    cd "$checkout" || exit
    next=$store.next
    rm -rf "$next"
    mkdir -p "$next/base" "$next/dev/changed" "$next/dev/new" || exit
    # The tracked files git sees as changed, and the ones saved before: the filter hides
    # their dev changes from git, so it is left out here.
    {
        git -c core.quotePath=false -c filter.octok-dev.clean= -c filter.octok-dev.smudge= \
            -c filter.octok-dev.required=false diff --name-only HEAD
        stored dev/changed
    } | LC_ALL=C sort -u | while IFS= read -r path; do
        [ -f "$path" ] && git cat-file -e "HEAD:$path" 2>/dev/null || continue
        base=$next/base/$path changed=$next/dev/changed/$path
        mkdir -p "$(dirname "$base")" "$(dirname "$changed")" &&
            git cat-file blob "HEAD:$path" >"$base" &&
            cp -p "$path" "$changed" || exit
        ! cmp -s "$base" "$changed" || rm "$base" "$changed"
    done || exit
    # The files git does not track: the ones it does not ignore, the ones saved before
    # that are still there, and the ones named.
    {
        git -c core.quotePath=false ls-files --others --exclude-standard
        stored dev/new
        for file in "$@"; do printf '%s\n' "$file"; done
    } | LC_ALL=C sort -u | while IFS= read -r path; do
        [ -f "$path" ] || continue
        ! git ls-files --error-unmatch -- "$path" >/dev/null 2>&1 || continue
        mkdir -p "$(dirname "$next/dev/new/$path")" && cp -p "$path" "$next/dev/new/$path" || exit
    done || exit
    for file in "$@"; do
        [ -f "$file" ] || printf 'missing %s\n' "$file"
    done
    printf '%s\n' "$filter_script" >"$next/filter.sh" || exit
    changed_before=$(stored dev/changed)
    rm -rf "$store"
    if [ -n "$(find "$next/dev" -type f | head -n 1)" ]; then
        mv "$next" "$store" || exit
    else
        rm -rf "$next"
    fi
    install
    # git takes a file whose size is not the one in its index as changed, without asking
    # the filter: staging the files, which the filter leaves as git has them, fixes the size.
    # A file that lost its dev changes is as git has it, with the size it had with them.
    {
        printf '%s\n' "$changed_before"
        stored dev/changed
    } | LC_ALL=C sort -u | while IFS= read -r path; do
        [ -f "$path" ] && git cat-file -e "HEAD:$path" 2>/dev/null || continue
        git add -- "$path" || exit
    done
    ;;
prepare)
    # Before a checkout of the commit `$2` is added to the repo in `$1`: tells the repo
    # what is saved, and prints the files whose dev changes do not apply to that commit.
    cd "$1" || exit
    rev=$2
    install
    tmp=$(mktemp -d) || exit
    trap 'rm -rf "$tmp"' EXIT
    stored dev/changed | while IFS= read -r path; do
        git cat-file blob "$rev:$path" >"$tmp/file" 2>/dev/null &&
            git merge-file -p "$tmp/file" "$store/base/$path" "$store/dev/changed/$path" >/dev/null 2>&1 ||
            printf '%s\n' "$path"
    done
    ;;
copy)
    # Copies the new files into the checkout `$1`, where they are not there already.
    stored dev/new | while IFS= read -r path; do
        [ ! -e "$1/$path" ] || continue
        mkdir -p "$(dirname "$1/$path")" && cp -p "$store/dev/new/$path" "$1/$path" || exit
    done
    ;;
show)
    stored dev/changed | sed 's/^/changed /'
    stored dev/new | sed 's/^/new /'
    echo diff
    [ ! -d "$store/dev/changed" ] ||
        (cd "$store" && git diff --no-index --no-color --no-ext-diff base dev/changed)
    exit 0
    ;;
esac
"##;

/// `filter.sh clean|smudge <path>`: git gives it a file on stdin and takes the result
/// from stdout. `smudge` puts the dev changes in, `clean` takes them out.
///
/// A file that changed in git where its dev changes are cannot get them: `smudge` then
/// leaves it as it is, so a checkout never fails, and `clean` leaves alone a file of a
/// checkout that never got them. Otherwise `clean` refuses a file it cannot take the dev
/// changes out of, so nothing is staged with dev changes in it.
const FILTER: &str = r##"
store=$(dirname "$0") mode=$1 path=$2
base=$store/base/$path dev=$store/dev/changed/$path
[ -f "$dev" ] || exec cat
tmp=$(mktemp -d) || exit
trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/file"
case $mode in
clean) from=$dev to=$base ;;
smudge) from=$base to=$dev ;;
esac
# Prints the file `$1` with the changes from `$2` to `$3`, or fails if they do not apply.
with_changes() {
    git merge-file -p "$1" "$2" "$3" 2>/dev/null
}
if with_changes "$tmp/file" "$from" "$to" >"$tmp/merged"; then
    cat "$tmp/merged"
elif [ "$mode" = smudge ]; then
    echo "octok: the dev changes of $path no longer apply: it is left as it is in git" >&2
    cat "$tmp/file"
elif git cat-file blob "HEAD:$path" >"$tmp/head" 2>/dev/null &&
    ! with_changes "$tmp/head" "$base" "$dev" >/dev/null; then
    # The commit of this checkout has other lines there: the file never got the dev changes.
    cat "$tmp/file"
else
    echo "octok: $path changed where its dev changes are, so they cannot be taken out of it." >&2
    echo "octok: if this is a dev change too, save it with \`octok dev save\`." >&2
    exit 1
fi
"##;

/// What is saved of the dev changes of one repo: changes that every new checkout gets
/// and no commit ever contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevChanges {
    pub repo: String,
    /// Tracked files with dev changes, by their path in the repo.
    pub changed: Vec<String>,
    /// Files git does not track, copied as they are.
    pub new: Vec<String>,
    /// The dev changes of the tracked files, as a diff.
    pub diff: String,
}

/// Saves the dev changes of the repos of a workspace from their checkouts: every change
/// that is not committed, every file git does not track, and among the files git ignores
/// the ones in `files`. What was saved before for a repo is replaced.
///
/// `repos` are keys or aliases: none means every repo with a checkout. Each of `files`
/// must be in a checkout at least.
pub fn save_dev_changes(
    home: &Home,
    project: &Project,
    workspace: &str,
    repos: &[&str],
    files: &[&str],
) -> Result<Vec<DevChanges>> {
    files.iter().try_for_each(|file| validate_file(file))?;
    let chosen = chosen_keys(project, repos)?;
    let mut not_found: Vec<&str> = files.to_vec();
    let mut saved = Vec::new();
    for member in members_of_existing(home, project, workspace)? {
        if chosen
            .as_ref()
            .is_some_and(|keys| !keys.contains(&member.key))
        {
            continue;
        }
        if !member.has_worktree()? {
            if chosen.is_none() {
                continue;
            }
            return Err(Error::NoWorktree {
                repo: member.key.to_string(),
                workspace: workspace.to_string(),
            });
        }
        let missing = member.dev.save(&member.worktree, files)?;
        not_found.retain(|file| missing.iter().any(|missing| missing == file));
        saved.push(member.dev.read(member.key)?);
    }
    match not_found.first() {
        Some(file) => Err(Error::DevFileNotFound(file.to_string())),
        None => Ok(saved),
    }
}

/// What is saved of the dev changes of each repo, where the project keeps the repo.
/// `repos` are keys or aliases: none means every repo.
pub fn dev_changes(home: &Home, project: &Project, repos: &[&str]) -> Result<Vec<DevChanges>> {
    let chosen = chosen_keys(project, repos)?;
    project
        .repos
        .iter()
        .filter(|(key, _)| {
            chosen
                .as_ref()
                .is_none_or(|keys| keys.contains(&key.as_str()))
        })
        .map(|(key, _)| dev_store(home, project, key).read(key))
        .collect()
}

/// The folder that holds the dev changes of one repo, on the machine of the repo.
pub(super) struct Store {
    location: Location,
    dir: PathBuf,
}

impl Store {
    pub(super) fn new(home: &Home, location: &Location, project: &str, repo: &str) -> Self {
        Self {
            location: location.clone(),
            dir: home.at(location).dev_changes_dir(project, repo),
        }
    }

    /// Makes git keep the dev changes in the checkouts of the repo in `repo_dir`.
    /// Returns the files whose dev changes do not apply to the commit `rev`.
    pub(super) fn prepare(&self, repo_dir: &Path, rev: &str) -> Result<Vec<String>> {
        let stale = self.run("prepare", &[&repo_dir.to_string_lossy(), rev])?;
        Ok(stale.lines().map(str::to_string).collect())
    }

    /// Copies the files git does not track into a new checkout.
    pub(super) fn copy_new_files(&self, checkout: &Path) -> Result<()> {
        self.run("copy", &[&checkout.to_string_lossy()]).map(drop)
    }

    pub(super) fn remove(&self) -> Result<()> {
        Files::new(&self.location).remove_dir_all(&self.dir)
    }

    /// Returns the ones of `files` that the checkout does not have.
    fn save(&self, checkout: &Path, files: &[&str]) -> Result<Vec<String>> {
        let checkout = checkout.to_string_lossy();
        let mut args = vec![&*checkout, FILTER];
        args.extend(files);
        let output = self.run("save", &args)?;
        let missing = output
            .lines()
            .filter_map(|line| line.strip_prefix("missing "));
        Ok(missing.map(str::to_string).collect())
    }

    fn read(&self, repo: &str) -> Result<DevChanges> {
        let output = self.run("show", &[])?;
        let mut lines = output.lines();
        let mut changes = DevChanges {
            repo: repo.to_string(),
            changed: Vec::new(),
            new: Vec::new(),
            diff: String::new(),
        };
        for line in lines.by_ref().take_while(|line| *line != "diff") {
            if let Some(path) = line.strip_prefix("changed ") {
                changes.changed.push(path.to_string());
            } else if let Some(path) = line.strip_prefix("new ") {
                changes.new.push(path.to_string());
            }
        }
        changes.diff = lines.map(repo_path_in_diff).collect::<Vec<_>>().join("\n");
        Ok(changes)
    }

    fn run(&self, action: &'static str, args: &[&str]) -> Result<String> {
        let dir = self.dir.to_string_lossy();
        let mut all = vec![action, &*dir];
        all.extend(args);
        run_script(&self.location, action, SCRIPT, &all)
    }
}

/// A file given to save: a path inside the checkout.
fn validate_file(file: &str) -> Result<()> {
    let path = Path::new(file);
    let inside = !file.is_empty()
        && !file.contains('\n')
        && path.is_relative()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
    if inside {
        Ok(())
    } else {
        Err(Error::InvalidName {
            kind: "file",
            value: file.to_string(),
            reason: "must be a path inside the checkout",
        })
    }
}

/// The diff compares the two folders of the store: its headers get the path in the repo.
fn repo_path_in_diff(line: &str) -> String {
    let is_header = ["diff --git ", "--- ", "+++ "]
        .iter()
        .any(|start| line.starts_with(start));
    if is_header {
        line.replacen("a/base/", "a/", 1)
            .replacen("b/dev/changed/", "b/", 1)
    } else {
        line.to_string()
    }
}
