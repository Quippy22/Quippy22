//! Refresh `assets/stats.txt` with live data from the GitHub API.
//!
//!     cargo run --release -- refresh            # everything, including lines of code
//!     cargo run --release -- refresh --no-loc   # skip the line count (no repo downloads)
//!
//! Set `GH_TOKEN` to raise the API rate limit (60 requests/hour otherwise).
//! Anything that fails to fetch keeps its previous value.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};
use std::time::Duration;

use chrono::Local;
use flate2::read::GzDecoder;
use serde_json::Value;

const API: &str = "https://api.github.com";
const STATS: &str = "assets/stats.txt";
const UA: &str = "quippy22-card";

/// Files counted as code. Anything else (assets, configs, docs) is ignored.
const CODE_EXT: &[&str] = &[
    ".rs", ".py", ".lua", ".sh", ".bash", ".zsh", ".fish", ".html", ".css", ".js", ".ts", ".jsx",
    ".tsx", ".c", ".h", ".cpp", ".go", ".java", ".sql", ".vim", ".nix",
];

/// Directories whose contents never count as code.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    "third_party",
];

pub fn run(args: &[String]) -> Result<(), String> {
    let no_loc = args.iter().any(|arg| arg == "--no-loc");
    if let Some(other) = args.iter().find(|arg| arg.as_str() != "--no-loc") {
        return Err(format!("unknown argument: {other}"));
    }

    let mut kv = read_stats();
    let user = kv
        .get("user")
        .filter(|user| !user.is_empty())
        .cloned()
        .unwrap_or_else(|| "Quippy22".into());
    let old = kv.clone();

    let client = Client::new();

    // Fatal: without the profile and repo list there is nothing worth writing.
    let profile = client.api(&format!("/users/{user}"))?;
    let login = string(&profile, "login")?;
    let joined = string(&profile, "created_at")?;
    kv.insert("user".into(), login.into());
    kv.insert("joined".into(), joined.chars().take(10).collect());
    kv.insert(
        "followers".into(),
        number(&profile, "followers")?.to_string(),
    );

    let mut repos = Vec::new();
    let mut page = 1;
    loop {
        let batch = client.api(&format!(
            "/users/{user}/repos?per_page=100&page={page}&sort=updated"
        ))?;
        let batch = batch.as_array().ok_or("repos: expected an array")?.clone();
        let count = batch.len();
        repos.extend(batch);
        if count < 100 {
            break;
        }
        page += 1;
    }
    kv.insert("repos".into(), repos.len().to_string());
    let stars: u64 = repos
        .iter()
        .filter_map(|r| r["stargazers_count"].as_u64())
        .sum();
    kv.insert("stars".into(), stars.to_string());

    // Best-effort: keep whatever was there before when one of these fails.
    match sum_languages(&client, &user, &repos) {
        Ok(languages) if !languages.is_empty() => {
            kv.insert("languages".into(), format_languages(&languages));
        }
        Ok(_) => keep(&kv, "languages"),
        Err(err) => {
            println!("! languages: {err}");
            keep(&kv, "languages");
        }
    }

    match count_commits(&client, &user) {
        Ok(total) => {
            kv.insert("commits".into(), total.to_string());
        }
        Err(err) => {
            println!("! commits: {err}");
            keep(&kv, "commits");
        }
    }

    if no_loc {
        println!("  lines of code: skipped (--no-loc)");
    } else if repos.is_empty() {
        println!("! lines of code: no repos found");
    } else {
        let mut total = 0;
        let mut failed = 0;
        for repo in &repos {
            let name = repo["name"].as_str().unwrap_or("?");
            let branch = repo["default_branch"].as_str().unwrap_or("main");
            let url =
                format!("https://codeload.github.com/{user}/{name}/tar.gz/refs/heads/{branch}");
            match client.count_lines(&url) {
                Ok(lines) => {
                    total += lines;
                    println!("  {name}: {total} lines so far");
                }
                Err(err) => {
                    println!("! {name}: {err} (skipped)");
                    failed += 1;
                }
            }
        }
        if failed > 0 {
            println!("! {failed} repo(s) unreadable — keeping previous lines of code");
        } else {
            kv.insert("loc".into(), total.to_string());
        }
    }

    write_stats(&kv)?;
    for key in ["repos", "followers", "stars", "commits", "loc"] {
        let before = old.get(key).map(String::as_str).unwrap_or("?");
        let after = kv.get(key).map(String::as_str).unwrap_or("?");
        let mark = if before == after { ' ' } else { '*' };
        println!("{mark} {key:<10} {before} -> {after}");
    }
    println!("wrote {STATS}");
    Ok(())
}

fn keep(kv: &BTreeMap<String, String>, key: &str) {
    println!(
        "  keeping {key} = {}",
        kv.get(key).map(String::as_str).unwrap_or("?")
    );
}

/// Minimal GitHub client: one agent, optional bearer token.
struct Client {
    agent: ureq::Agent,
    token: Option<String>,
}

impl Client {
    fn new() -> Client {
        let agent = ureq::AgentBuilder::new()
            .user_agent(UA)
            .timeout(Duration::from_secs(120))
            .build();
        let token = ["GH_TOKEN", "GITHUB_TOKEN"]
            .iter()
            .find_map(|key| std::env::var(key).ok())
            .filter(|token| !token.is_empty());
        Client { agent, token }
    }

    fn api(&self, path: &str) -> Result<Value, String> {
        let url = format!("{API}{path}");
        let mut request = self
            .agent
            .get(&url)
            .set("Accept", "application/vnd.github+json");
        if let Some(token) = &self.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        let response = request.call().map_err(|err| format!("{path}: {err}"))?;
        let mut body = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|err| format!("{path}: {err}"))?;
        serde_json::from_slice(&body).map_err(|err| format!("{path}: {err}"))
    }

    /// Code lines in a repo tarball: no blanks, no whole-line comments.
    /// The tarball is streamed entry by entry, never unpacked to disk.
    fn count_lines(&self, url: &str) -> Result<u64, String> {
        let response = self.agent.get(url).call().map_err(|err| err.to_string())?;
        let mut archive = tar::Archive::new(GzDecoder::new(response.into_reader()));
        let mut total = 0;

        for entry in archive.entries().map_err(|err| err.to_string())? {
            let mut entry = entry.map_err(|err| err.to_string())?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().map_err(|err| err.to_string())?.into_owned();
            if skipped_dir(&path) {
                continue;
            }
            let ext = extension(&path);
            if !CODE_EXT.contains(&ext.as_str()) {
                continue;
            }

            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|err| err.to_string())?;
            let text = String::from_utf8_lossy(&bytes);
            let mark = comment_mark(&ext);
            let block = has_block_comments(&ext);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if mark.is_some_and(|mark| line.starts_with(mark)) {
                    continue;
                }
                if block && (line.starts_with("/*") || line.starts_with('*')) {
                    continue;
                }
                total += 1;
            }
        }
        Ok(total)
    }
}

fn skipped_dir(path: &Path) -> bool {
    path.components().any(|part| match part {
        Component::Normal(name) => SKIP_DIRS.contains(&name.to_string_lossy().as_ref()),
        _ => false,
    })
}

/// ".RS" -> ".rs"; paths without an extension give "".
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_lowercase()))
        .unwrap_or_default()
}

fn comment_mark(ext: &str) -> Option<&'static str> {
    Some(match ext {
        ".rs" | ".c" | ".h" | ".cpp" | ".go" | ".js" | ".ts" | ".jsx" | ".tsx" | ".java" => "//",
        ".py" | ".sh" | ".bash" | ".zsh" | ".fish" | ".nix" => "#",
        ".lua" | ".sql" => "--",
        ".vim" => "\"",
        _ => return None,
    })
}

/// Extensions whose `/* ... */` blocks are worth stripping line by line.
fn has_block_comments(ext: &str) -> bool {
    matches!(
        ext,
        ".rs" | ".c" | ".h" | ".cpp" | ".go" | ".js" | ".ts" | ".jsx" | ".tsx" | ".java" | ".css"
    )
}

fn sum_languages(
    client: &Client,
    user: &str,
    repos: &[Value],
) -> Result<BTreeMap<String, u64>, String> {
    let mut languages = BTreeMap::new();
    for repo in repos {
        let name = repo["name"].as_str().ok_or("repo without a name")?;
        let bytes = client.api(&format!("/repos/{user}/{name}/languages"))?;
        let bytes = bytes.as_object().ok_or("languages: expected an object")?;
        for (language, size) in bytes {
            *languages.entry(language.clone()).or_insert(0) += size.as_u64().unwrap_or(0);
        }
    }
    Ok(languages)
}

fn count_commits(client: &Client, user: &str) -> Result<u64, String> {
    let found = client.api(&format!("/search/commits?q=author:{user}&per_page=1"))?;
    number(&found, "total_count")
}

fn format_languages(languages: &BTreeMap<String, u64>) -> String {
    let mut items: Vec<_> = languages.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));
    items
        .iter()
        .map(|(name, bytes)| format!("{name}:{bytes}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn read_stats() -> BTreeMap<String, String> {
    let mut kv = BTreeMap::new();
    let Ok(raw) = fs::read_to_string(STATS) else {
        return kv;
    };
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            kv.insert(key.trim().into(), value.trim().into());
        }
    }
    kv
}

fn write_stats(kv: &BTreeMap<String, String>) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# GitHub data for card.svg — refresh with `cargo run --release -- refresh`\n");
    let _ = writeln!(out, "# generated: {}", Local::now().date_naive());
    for key in [
        "user",
        "joined",
        "repos",
        "followers",
        "stars",
        "commits",
        "loc",
    ] {
        let _ = writeln!(
            out,
            "{key}={}",
            kv.get(key).map(String::as_str).unwrap_or("0")
        );
    }
    let _ = writeln!(
        out,
        "languages={}",
        kv.get("languages").map(String::as_str).unwrap_or("")
    );
    fs::write(STATS, out).map_err(|err| format!("write {STATS}: {err}"))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} missing from the API response"))
}

fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("{key} missing from the API response"))
}
