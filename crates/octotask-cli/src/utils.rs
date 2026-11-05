use octotask_core::Project;
use serde_json::json;

pub(crate) fn print_projects(projects: &[String], default: Option<&str>, json: bool) {
    if json {
        println!(
            "{}",
            json!({"schema": 1, "default": default, "projects": projects})
        );
    } else if projects.is_empty() {
        println!("no projects yet: create one with `octok project init <name>`");
    } else {
        for name in projects {
            let marker = if default == Some(name.as_str()) {
                "*"
            } else {
                " "
            };
            println!("{marker} {name}");
        }
    }
}

pub(crate) fn print_repos(project: &Project, json: bool) {
    let rows: Vec<RepoRow> = project
        .repos
        .iter()
        .map(|(key, repo)| RepoRow {
            key,
            url: &repo.url,
            base: project.effective_base(repo),
        })
        .collect();
    if json {
        print_repos_json(&project.name, &rows);
    } else if rows.is_empty() {
        println!(
            "no repos in {}: add one with `octok repo add <url>`",
            project.name
        );
    } else {
        print_repos_table(&rows);
    }
}

struct RepoRow<'a> {
    key: &'a str,
    url: &'a str,
    base: &'a str,
}

fn print_repos_json(project_name: &str, rows: &[RepoRow]) {
    let repos: Vec<_> = rows
        .iter()
        .map(|row| json!({"key": row.key, "url": row.url, "base": row.base}))
        .collect();
    println!(
        "{}",
        json!({"schema": 1, "project": project_name, "repos": repos})
    );
}

fn print_repos_table(rows: &[RepoRow]) {
    let key_width = rows.iter().map(|row| row.key.len()).max().unwrap_or(0);
    let base_width = rows.iter().map(|row| row.base.len()).max().unwrap_or(0);
    for RepoRow { key, url, base } in rows {
        println!("{key:key_width$}  {base:base_width$}  {url}");
    }
}
