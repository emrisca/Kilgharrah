#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

const BASE: &str = "https://clawhub.ai";
const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 12 * 1024 * 1024;
const MAX_READ: u64 = 512 * 1024;
const PREVIEW_CHARS: usize = 20_000;
const MAX_SCAN_DEPTH: usize = 4;

/// The store's fixed category set. Extra folders the user adds are still read.
const CATEGORIES: [(&str, &str); 6] = [
    ("design", "Interfaces, type and motion"),
    ("development", "Code, tests and releases"),
    ("writing", "Copy, docs and tone"),
    ("research", "Search, sources and analysis"),
    ("productivity", "Workflow and notes"),
    ("other", "Imported and uncategorised"),
];

/// Local shelves for the ClawHub registry. The registry exposes topics, not a
/// category filter, so Kilgharrah groups skills by matching those topics.
const MARKET_CATEGORIES: [(&str, &str, &[&str]); 12] = [
    (
        "design",
        "Design",
        &["design", "ui", "ux", "css", "figma", "tailwind", "typography", "color", "motion", "animation", "icon", "layout", "theme", "visual", "shader"],
    ),
    (
        "development",
        "Development",
        &["code", "git", "github", "gitlab", "api", "sdk", "deploy", "build", "test", "debug", "rust", "python", "typescript", "javascript", "react", "node", "sql", "database", "devops", "docker", "kubernetes", "backend", "frontend", "compiler", "cli"],
    ),
    (
        "writing",
        "Writing",
        &["writ", "docs", "documentation", "copy", "blog", "prose", "edit", "translat", "markdown", "summary", "summarize", "humanize"],
    ),
    (
        "research",
        "Research",
        &["research", "search", "web search", "news", "rss", "arxiv", "source", "citation", "scrape", "crawl", "competitor"],
    ),
    (
        "data",
        "Data",
        &["data", "json", "csv", "analytics", "spreadsheet", "visualization", "chart", "excel", "ontology"],
    ),
    (
        "productivity",
        "Productivity",
        &["productivity", "calendar", "email", "gmail", "task", "note", "notion", "obsidian", "meeting", "booking", "reminder", "inbox"],
    ),
    (
        "media",
        "Media",
        &["media", "video", "audio", "music", "image", "photo", "screenshot", "youtube", "transcri", "pdf", "design asset"],
    ),
    (
        "marketing",
        "Marketing",
        &["marketing", "seo", "social", "ads", "growth", "commerce", "shopify", "sales", "crm", "pricing"],
    ),
    (
        "automation",
        "Automation",
        &["automation", "workflow", "n8n", "zapier", "cron", "orchestrat", "mcp", "pipeline", "agent-to-agent"],
    ),
    (
        "security",
        "Security",
        &["security", "privacy", "permission", "audit", "sandbox", "scan", "secret", "vet", "malware", "trust"],
    ),
    (
        "ai",
        "AI and agents",
        &["ai", "llm", "model", "prompt", "embedding", "openai", "anthropic", "agent memory", "self improv", "learning", "context"],
    ),
    ("other", "Other", &[]),
];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Category {
    id: String,
    name: String,
    blurb: String,
    count: usize,
    source: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Skill {
    id: String,
    source: String,
    slug: String,
    name: String,
    description: String,
    category: String,
    path: String,
    enabled: bool,
    files: usize,
    updated_at: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SkillFile {
    path: String,
    bytes: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McpServer {
    name: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppState {
    store: String,
    skills_root: String,
    hermes_root: String,
    mcp_path: String,
    categories: Vec<Category>,
    skills: Vec<Skill>,
    servers: Vec<McpServer>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogItem {
    id: String,
    kind: String,
    owner: String,
    slug: String,
    name: String,
    summary: String,
    version: String,
    downloads: u64,
    category: String,
    topics: Vec<String>,
    installable: bool,
    command: Option<String>,
}

// ---------------------------------------------------------------- locations

fn home_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_default();
    PathBuf::from(home)
}

fn agents_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("KILGHARRAH_HOME") {
        return PathBuf::from(custom);
    }
    home_dir().join(".agents")
}

fn hermes_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("KILGHARRAH_HERMES") {
        return PathBuf::from(custom);
    }
    home_dir().join(".hermes").join("skills")
}

fn store_root() -> PathBuf {
    agents_root().join("kilgharrah")
}

fn skills_store() -> PathBuf {
    store_root().join("skills")
}

fn links_root() -> PathBuf {
    agents_root().join("skills")
}

fn mcp_path() -> PathBuf {
    agents_root().join("mcp.json")
}

fn ensure_store() -> Result<(), String> {
    fs::create_dir_all(skills_store()).map_err(|e| format!("Could not create the store: {e}"))?;
    for (id, _) in CATEGORIES {
        fs::create_dir_all(skills_store().join(id)).ok();
    }
    let readme = store_root().join("README.md");
    if !readme.exists() {
        let text = "# Kilgharrah store\n\nEach skill lives in `skills/<category>/<slug>/SKILL.md`.\nA skill is live for the CLI when `~/.agents/skills/<slug>` links to its folder here.\nDisable a skill in Kilgharrah to remove the link. The files stay in the store.\n";
        fs::write(readme, text).ok();
    }
    fs::create_dir_all(links_root()).ok();
    Ok(())
}

fn titleize(id: &str) -> String {
    let spaced = id.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 64 {
        out.truncate(64);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

fn safe_segment(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 96
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

fn safe_relative(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => out.push(segment),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- skills

fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut description = None;
    let mut lines = text.lines();
    if lines.next().map(|line| line.trim()) != Some("---") {
        return (None, None);
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(value) = trimmed.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    (name, description)
}

fn count_files(dir: &Path, depth: usize) -> usize {
    if depth == 0 {
        return 0;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += count_files(&path, depth - 1);
        } else {
            total += 1;
        }
    }
    total.min(999)
}

fn link_path(slug: &str) -> PathBuf {
    links_root().join(slug)
}

fn is_enabled(slug: &str) -> bool {
    fs::symlink_metadata(link_path(slug)).is_ok()
}

fn link_skill(slug: &str, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(links_root()).map_err(|e| format!("Could not create {}: {e}", links_root().display()))?;
    let link = link_path(slug);
    if let Ok(meta) = fs::symlink_metadata(&link) {
        if meta.file_type().is_symlink() || meta.is_file() {
            fs::remove_file(&link).map_err(|e| format!("Could not replace the link: {e}"))?;
        } else {
            return Err(format!(
                "A real folder already sits at {}. Move it before enabling this skill.",
                link.display()
            ));
        }
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir, &link).map_err(|e| format!("Could not link the skill: {e}"))?;
    }
    #[cfg(not(unix))]
    {
        copy_dir(dir, &link).map_err(|e| format!("Could not copy the skill: {e}"))?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn unlink_skill(slug: &str) {
    let link = link_path(slug);
    if let Ok(meta) = fs::symlink_metadata(&link) {
        if meta.file_type().is_symlink() || meta.is_file() {
            let _ = fs::remove_file(&link);
        }
    }
}

fn collect_skills(root: PathBuf, source: &str, out: &mut Vec<(PathBuf, String, String)>) {
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.join("SKILL.md").is_file() {
            out.push((path, "other".to_string(), source.to_string()));
            continue;
        }
        let Ok(children) = fs::read_dir(&path) else {
            continue;
        };
        for child in children.flatten() {
            let dir = child.path();
            if dir.is_dir() && dir.join("SKILL.md").is_file() {
                out.push((dir, name.clone(), source.to_string()));
            }
        }
    }
}

fn scan_skills() -> Vec<Skill> {
    scan_roots(skills_store(), hermes_root())
}

fn scan_roots(store: PathBuf, hermes: PathBuf) -> Vec<Skill> {
    let mut candidates: Vec<(PathBuf, String, String)> = Vec::new();
    collect_skills(store, "store", &mut candidates);
    collect_skills(hermes, "hermes", &mut candidates);

    let mut skills = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (dir, category, source) in candidates {
        let manifest = dir.join("SKILL.md");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let slug = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if slug.is_empty() || seen.iter().any(|value| value == &slug) {
            continue;
        }
        seen.push(slug.clone());
        let (name, description) = parse_frontmatter(&text);
        skills.push(Skill {
            id: format!("{source}:{category}:{slug}"),
            source: source.clone(),
            name: name.filter(|n| !n.is_empty()).unwrap_or_else(|| titleize(&slug)),
            description: description.unwrap_or_default(),
            category,
            path: dir.display().to_string(),
            enabled: source == "store" && is_enabled(&slug),
            files: count_files(&dir, 3),
            updated_at: modified_secs(&manifest),
            slug,
        });
    }
    skills.sort_by(|a, b| {
        let rank = |id: &str| CATEGORIES.iter().position(|(c, _)| *c == id).unwrap_or(99);
        rank(&a.category)
            .cmp(&rank(&b.category))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    skills
}

fn list_categories(skills: &[Skill]) -> Vec<Category> {
    let mut order: Vec<(String, String, String)> = CATEGORIES
        .iter()
        .map(|(id, blurb)| (id.to_string(), blurb.to_string(), "store".to_string()))
        .collect();
    for skill in skills {
        if order.iter().any(|(id, _, _)| id == &skill.category) {
            continue;
        }
        let blurb = if skill.source == "hermes" {
            "Hermes library"
        } else {
            "Your own folder"
        };
        order.push((skill.category.clone(), blurb.to_string(), skill.source.clone()));
    }
    order
        .into_iter()
        .map(|(id, blurb, source)| Category {
            count: skills.iter().filter(|s| s.category == id).count(),
            name: titleize(&id),
            blurb,
            source,
            id,
        })
        .filter(|entry| entry.count > 0 || CATEGORIES.iter().any(|(id, _)| *id == entry.id))
        .collect()
}

// ---------------------------------------------------------------- MCP config

fn read_mcp() -> Value {
    fs::read_to_string(mcp_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }))
}

fn write_mcp(value: &Value) -> Result<(), String> {
    let path = mcp_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("Could not encode mcp.json: {e}"))?;
    let temp = path.with_extension("json.kilgharrah");
    fs::write(&temp, format!("{text}\n")).map_err(|e| format!("Could not write mcp.json: {e}"))?;
    fs::rename(&temp, &path).map_err(|e| format!("Could not replace mcp.json: {e}"))
}

fn read_servers() -> Vec<McpServer> {
    let value = read_mcp();
    let mut servers: Vec<McpServer> = Vec::new();
    if let Some(map) = value.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, entry) in map {
            let command = entry
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let args = entry
                .get("args")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let env = entry
                .get("env")
                .and_then(|v| v.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(key, value)| value.as_str().map(|v| (key.clone(), v.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            servers.push(McpServer {
                name: name.clone(),
                command,
                args,
                env,
                enabled: true,
            });
        }
    }
    servers
}

// ---------------------------------------------------------------- ClawHub

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(25))
        .user_agent("Kilgharrah/0.1 (skills and MCP manager for Freebuff)")
        .build()
}

fn get_json(url: &str) -> Result<Value, String> {
    match http_agent().get(url).call() {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|e| format!("ClawHub sent a response Kilgharrah could not read: {e}")),
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            let message: String = body.chars().take(180).collect();
            Err(format!("ClawHub returned {code}. {message}"))
        }
        Err(error) => Err(format!("Could not reach ClawHub: {error}")),
    }
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    match http_agent().get(url).call() {
        Ok(response) => {
            let mut buffer = Vec::new();
            response
                .into_reader()
                .take(MAX_BYTES)
                .read_to_end(&mut buffer)
                .map_err(|e| format!("Could not read the download: {e}"))?;
            Ok(buffer)
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("ClawHub returned {code} for that file.")),
        Err(error) => Err(format!("Could not reach ClawHub: {error}")),
    }
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn mentions(haystack: &str, needle: &str) -> bool {
    if needle.len() > 3 {
        return haystack.contains(needle);
    }
    haystack
        .split(|value: char| !value.is_alphanumeric())
        .any(|word| word == needle)
}

fn bucket_for(topics: &str, name: &str, summary: &str) -> String {
    let prose = format!("{} {}", name.to_lowercase(), summary.to_lowercase());
    let mut best = ("other", 0usize);
    for (id, _, words) in MARKET_CATEGORIES {
        let mut score = 0;
        for word in words {
            if mentions(topics, word) {
                score += 3;
            }
            if mentions(&prose, word) {
                score += 1;
            }
        }
        if score > best.1 {
            best = (id, score);
        }
    }
    best.0.to_string()
}

fn skill_topics(value: &Value) -> Vec<String> {
    value
        .get("topics")
        .and_then(|v| v.as_array())
        .or_else(|| {
            value
                .get("native")
                .and_then(|n| n.get("skill"))
                .and_then(|s| s.get("topics"))
                .and_then(|v| v.as_array())
        })
        .map(|rows| {
            rows.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn skill_item(value: &Value) -> Option<CatalogItem> {
    let slug = value.get("slug")?.as_str()?.to_string();
    let owner = value
        .get("ownerHandle")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("native").and_then(|n| n.get("ownerHandle")).and_then(|v| v.as_str()))
        .or_else(|| value.get("owner").and_then(|o| o.get("handle")).and_then(|v| v.as_str()))
        .unwrap_or("clawhub")
        .to_string();
    let name = value
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or(&slug)
        .to_string();
    let summary = value.get("summary").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("latestVersion").and_then(|v| v.as_str()))
        .or_else(|| {
            value
                .get("latestVersion")
                .and_then(|v| v.get("version"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or_default()
        .to_string();
    let downloads = value.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0);
    let topics = skill_topics(value);
    let category = bucket_for(&topics.join(" ").to_lowercase(), &name, &summary);
    Some(CatalogItem {
        id: format!("skill:{owner}:{slug}"),
        kind: "skill".into(),
        owner,
        slug,
        name,
        summary,
        version,
        downloads,
        category,
        topics,
        installable: true,
        command: None,
    })
}

fn package_item(value: &Value) -> Option<CatalogItem> {
    let package = value.get("package").unwrap_or(value);
    let name = package.get("name")?.as_str()?.to_string();
    let owner = package
        .get("ownerHandle")
        .and_then(|v| v.as_str())
        .unwrap_or("clawhub")
        .to_string();
    let family = package
        .get("family")
        .and_then(|v| v.as_str())
        .unwrap_or("plugin")
        .to_string();
    let summary = package.get("summary").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let version = package
        .get("latestVersion")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let downloads = package
        .get("stats")
        .and_then(|s| s.get("downloads"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    Some(CatalogItem {
        id: format!("{family}:{name}"),
        kind: if family == "bundle-plugin" { "bundle" } else { "plugin" }.into(),
        owner,
        slug: name.clone(),
        name: name.clone(),
        summary,
        version,
        downloads,
        category: "development".into(),
        topics: Vec::new(),
        installable: false,
        command: Some(format!("npm install {name}")),
    })
}

fn suggest_category(meta: &Value) -> String {
    let mut haystack = String::new();
    if let Some(topics) = meta.get("skill").and_then(|s| s.get("topics")).and_then(|t| t.as_array()) {
        for topic in topics {
            if let Some(text) = topic.as_str() {
                haystack.push_str(text);
                haystack.push(' ');
            }
        }
    }
    if let Some(summary) = meta.get("skill").and_then(|s| s.get("summary")).and_then(|v| v.as_str()) {
        haystack.push_str(summary);
    }
    let haystack = haystack.to_lowercase();
    let rules: [(&str, &[&str]); 5] = [
        ("design", &["design", "ui", "css", "type", "motion", "figma", "color"]),
        ("development", &["code", "dev", "test", "git", "build", "deploy", "api", "rust", "python", "react"]),
        ("writing", &["writ", "doc", "copy", "prose", "blog", "edit", "translat"]),
        ("research", &["research", "search", "data", "analy", "scrape", "source"]),
        ("productivity", &["productiv", "note", "task", "calendar", "workflow", "email"]),
    ];
    for (category, needles) in rules {
        if needles.iter().any(|needle| haystack.contains(needle)) {
            return category.to_string();
        }
    }
    "other".to_string()
}

// ---------------------------------------------------------------- commands

fn build_state() -> Result<AppState, String> {
    ensure_store()?;
    let skills = scan_skills();
    Ok(AppState {
        store: store_root().display().to_string(),
        skills_root: links_root().display().to_string(),
        hermes_root: hermes_root().display().to_string(),
        mcp_path: mcp_path().display().to_string(),
        categories: list_categories(&skills),
        skills,
        servers: read_servers(),
    })
}

#[tauri::command]
fn snapshot() -> Result<AppState, String> {
    build_state()
}

#[tauri::command]
fn add_skill(name: String, description: String, body: String, category: String) -> Result<AppState, String> {
    ensure_store()?;
    let display = name.trim().to_string();
    let slug = slugify(&display);
    if slug.is_empty() {
        return Err("Give the skill a name with letters or numbers.".into());
    }
    let category = category.trim().to_lowercase();
    if !safe_segment(&category) || !skills_store().join(&category).is_dir() {
        return Err("Pick one of the categories in the store.".into());
    }
    let dir = skills_store().join(&category).join(&slug);
    if dir.exists() {
        return Err(format!("{slug} already exists in {category}."));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create the skill folder: {e}"))?;
    let summary = if description.trim().is_empty() {
        "Instructions for when this skill applies.".to_string()
    } else {
        description.trim().to_string()
    };
    let instructions = if body.trim().is_empty() {
        format!("# {display}\n\nWrite the steps the agent should follow here.\n")
    } else {
        body.trim().to_string()
    };
    let text = format!("---\nname: {display}\ndescription: {summary}\n---\n\n{instructions}\n");
    fs::write(dir.join("SKILL.md"), text).map_err(|e| format!("Could not write SKILL.md: {e}"))?;
    link_skill(&slug, &dir)?;
    build_state()
}

#[tauri::command]
fn delete_skill(category: String, slug: String) -> Result<AppState, String> {
    if !safe_segment(&category) || !safe_segment(&slug) {
        return Err("That skill path is not valid.".into());
    }
    let dir = skills_store().join(&category).join(&slug);
    if !dir.is_dir() {
        return Err("That skill is not in the store.".into());
    }
    unlink_skill(&slug);
    fs::remove_dir_all(&dir).map_err(|e| format!("Could not remove the skill: {e}"))?;
    build_state()
}

#[tauri::command]
fn set_skill_enabled(category: String, slug: String, enabled: bool) -> Result<AppState, String> {
    if !safe_segment(&category) || !safe_segment(&slug) {
        return Err("That skill path is not valid.".into());
    }
    let dir = skills_store().join(&category).join(&slug);
    if !dir.is_dir() {
        return Err("That skill is not in the store.".into());
    }
    if enabled {
        link_skill(&slug, &dir)?;
    } else {
        unlink_skill(&slug);
    }
    build_state()
}

#[tauri::command]
fn move_skill(category: String, slug: String, target: String) -> Result<AppState, String> {
    if !safe_segment(&category) || !safe_segment(&slug) || !safe_segment(&target) {
        return Err("That category or skill path is not valid.".into());
    }
    if category == target {
        return build_state();
    }
    let from = skills_store().join(&category).join(&slug);
    let to_dir = skills_store().join(&target);
    if !from.is_dir() {
        return Err("That skill is not in the store.".into());
    }
    if !to_dir.is_dir() {
        return Err("That category does not exist in the store.".into());
    }
    let to = to_dir.join(&slug);
    if to.exists() {
        return Err(format!("{slug} already exists in {target}."));
    }
    let was_enabled = is_enabled(&slug);
    fs::rename(&from, &to).map_err(|e| format!("Could not move the skill: {e}"))?;
    if was_enabled {
        link_skill(&slug, &to)?;
    }
    build_state()
}

#[tauri::command]
fn save_server(name: String, command: String, args: Vec<String>, env: Vec<String>) -> Result<AppState, String> {
    let name = name.trim().to_string();
    if !safe_segment(&name) {
        return Err("Use a short name with letters, numbers, dots or hyphens.".into());
    }
    let command = command.trim().to_string();
    if command.is_empty() {
        return Err("A command is required, for example npx.".into());
    }
    let mut root = read_mcp();
    let Some(root) = root.as_object_mut() else {
        return Err("mcp.json does not hold a JSON object.".into());
    };
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    let Some(servers) = servers.as_object_mut() else {
        return Err("mcpServers in mcp.json is not an object.".into());
    };
    let mut entry = serde_json::Map::new();
    entry.insert("command".into(), Value::String(command));
    let args: Vec<Value> = args
        .into_iter()
        .map(|arg| arg.trim().to_string())
        .filter(|arg| !arg.is_empty())
        .map(Value::String)
        .collect();
    if !args.is_empty() {
        entry.insert("args".into(), Value::Array(args));
    }
    let env_map: BTreeMap<String, Value> = env
        .iter()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some((key.to_string(), Value::String(value.trim().to_string())))
        })
        .collect();
    if !env_map.is_empty() {
        entry.insert("env".into(), Value::Object(env_map.into_iter().collect()));
    }
    servers.insert(name.clone(), Value::Object(entry));
    write_mcp(&Value::Object(root.clone()))?;
    build_state()
}

#[tauri::command]
fn delete_server(name: String) -> Result<AppState, String> {
    let mut root = read_mcp();
    if let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove(&name);
    }
    write_mcp(&root)?;
    build_state()
}

#[tauri::command]
fn search_market(query: String, limit: Option<u32>) -> Result<Vec<CatalogItem>, String> {
    let limit = limit.unwrap_or(24).clamp(1, 300) as usize;
    let query = query.trim().to_string();
    let mut items: Vec<CatalogItem> = Vec::new();

    if query.is_empty() {
        items = skill_pool(limit)?;
    } else {
        let encoded = encode(&query);
        let skills = get_json(&format!(
            "{BASE}/api/v1/search?q={encoded}&limit={}",
            limit.min(60)
        ))?;
        if let Some(rows) = skills.get("results").and_then(|v| v.as_array()) {
            items.extend(rows.iter().filter_map(skill_item));
        }
        let packages = get_json(&format!("{BASE}/api/v1/packages/search?q={encoded}&limit=8"))?;
        if let Some(rows) = packages.get("results").and_then(|v| v.as_array()) {
            items.extend(rows.iter().filter_map(package_item));
        }
    }

    let mut unique: Vec<CatalogItem> = Vec::new();
    for item in items {
        if unique.iter().any(|existing| existing.id == item.id) {
            continue;
        }
        unique.push(item);
    }
    unique.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    Ok(unique)
}

fn skill_pool(limit: usize) -> Result<Vec<CatalogItem>, String> {
    let mut items: Vec<CatalogItem> = Vec::new();
    // ClawHub caps a page near 200 rows and repeats the same rows for any
    // cursor, so one request already holds the pool. Extra sorts only matter
    // when that first page came back thin.
    for sort in ["downloads", "newest", "trending"] {
        if items.len() >= limit.min(100) {
            break;
        }
        let url = format!("{BASE}/api/v1/skills?sort={sort}&limit={}", limit.min(200));
        let value = get_json(&url)?;
        let rows = value
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for row in &rows {
            if let Some(item) = skill_item(row) {
                if !items.iter().any(|existing| existing.id == item.id) {
                    items.push(item);
                }
            }
        }
    }
    items.truncate(limit);
    Ok(items)
}

#[tauri::command]
fn preview_market(slug: String, owner: Option<String>) -> Result<String, String> {
    if !safe_segment(&slug) {
        return Err("That skill name is not valid.".into());
    }
    let mut url = format!("{BASE}/api/v1/skills/{}/file?path=SKILL.md", encode(&slug));
    if let Some(owner) = owner.filter(|value| safe_segment(value)) {
        url.push_str(&format!("&owner={}", encode(&owner)));
    }
    let bytes = get_bytes(&url)?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(text.chars().take(PREVIEW_CHARS).collect())
}

#[tauri::command]
fn install_market(
    slug: String,
    owner: Option<String>,
    category: Option<String>,
) -> Result<AppState, String> {
    ensure_store()?;
    if !safe_segment(&slug) {
        return Err("That skill name is not valid.".into());
    }
    let owner = owner.unwrap_or_default().trim().to_string();
    let owner_query = if owner.is_empty() {
        String::new()
    } else {
        format!("?owner={}", encode(&owner))
    };
    let owner_suffix = if owner.is_empty() {
        String::new()
    } else {
        format!("&owner={}", encode(&owner))
    };
    let meta = get_json(&format!("{BASE}/api/v1/skills/{}{owner_query}", encode(&slug)))?;
    let version = meta
        .get("latestVersion")
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("ClawHub has no published version for that skill.")?
        .to_string();
    let detail = get_json(&format!(
        "{BASE}/api/v1/skills/{}/versions/{}{owner_query}",
        encode(&slug),
        encode(&version)
    ))?;
    let files = detail
        .get("version")
        .and_then(|v| v.get("files"))
        .and_then(|v| v.as_array())
        .ok_or("ClawHub returned no file list for that version.")?;
    if files.len() > MAX_FILES {
        return Err("That skill has too many files for Kilgharrah to install.".into());
    }
    let chosen = match category {
        Some(id) if safe_segment(&id) && skills_store().join(&id).is_dir() => id,
        _ => suggest_category(&meta),
    };
    let dir = skills_store().join(&chosen).join(&slug);
    if dir.exists() {
        return Err(format!("{slug} is already installed in {chosen}."));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create the skill folder: {e}"))?;

    let mut total: u64 = 0;
    for file in files {
        let Some(relative) = file.get("path").and_then(|v| v.as_str()).and_then(safe_relative) else {
            let _ = fs::remove_dir_all(&dir);
            return Err("ClawHub listed a file path Kilgharrah will not write.".into());
        };
        let url = format!(
            "{BASE}/api/v1/skills/{}/file?path={}&version={}{owner_suffix}",
            encode(&slug),
            encode(&relative.to_string_lossy()),
            encode(&version)
        );
        let bytes = match get_bytes(&url) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = fs::remove_dir_all(&dir);
                return Err(error);
            }
        };
        total += bytes.len() as u64;
        if total > MAX_BYTES {
            let _ = fs::remove_dir_all(&dir);
            return Err("That skill is larger than the 12 MB install limit.".into());
        }
        let target = dir.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create a folder: {e}"))?;
        }
        if let Err(error) = fs::write(&target, bytes) {
            let _ = fs::remove_dir_all(&dir);
            return Err(format!("Could not write a skill file: {error}"));
        }
    }
    if let Err(error) = link_skill(&slug, &dir) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    build_state()
}

fn skill_dir(source: &str, category: &str, slug: &str) -> Result<PathBuf, String> {
    if !safe_segment(category) || !safe_segment(slug) {
        return Err("That skill path is not valid.".into());
    }
    let root = match source {
        "store" => skills_store(),
        "hermes" => hermes_root(),
        _ => return Err("Unknown skill source.".into()),
    };
    let dir = root.join(category).join(slug);
    if !dir.join("SKILL.md").is_file() {
        return Err("That skill has no SKILL.md.".into());
    }
    Ok(dir)
}

fn walk_files(dir: &Path, base: &Path, depth: usize, out: &mut Vec<SkillFile>) {
    if depth == 0 || out.len() >= 400 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_files(&path, base, depth - 1, out);
        } else if let Ok(relative) = path.strip_prefix(base) {
            out.push(SkillFile {
                path: relative.to_string_lossy().to_string(),
                bytes: fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0),
            });
        }
    }
}

#[tauri::command]
fn list_skill_files(source: String, category: String, slug: String) -> Result<Vec<SkillFile>, String> {
    let dir = skill_dir(&source, &category, &slug)?;
    let mut files = Vec::new();
    walk_files(&dir, &dir, MAX_SCAN_DEPTH, &mut files);
    files.sort_by(|a, b| {
        let rank = |entry: &SkillFile| if entry.path == "SKILL.md" { 0 } else { 1 };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
    });
    Ok(files)
}

#[tauri::command]
fn read_skill_file(
    source: String,
    category: String,
    slug: String,
    path: String,
) -> Result<String, String> {
    let dir = skill_dir(&source, &category, &slug)?;
    let relative = safe_relative(&path).ok_or("That file path is not valid.")?;
    let target = dir.join(&relative);
    if !target.is_file() {
        return Err("That file is not part of this skill.".into());
    }
    let meta = fs::metadata(&target).map_err(|e| format!("Could not read the file: {e}"))?;
    if meta.len() > MAX_READ {
        return Err("That file is too large to open here.".into());
    }
    let bytes = fs::read(&target).map_err(|e| format!("Could not read the file: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "That file is not plain text.".to_string())
}

#[tauri::command]
fn write_skill_file(
    category: String,
    slug: String,
    path: String,
    content: String,
) -> Result<AppState, String> {
    let dir = skill_dir("store", &category, &slug)?;
    let relative = safe_relative(&path).ok_or("That file path is not valid.")?;
    let target = dir.join(&relative);
    if !target.is_file() {
        return Err("That file is not part of this skill.".into());
    }
    fs::write(&target, content).map_err(|e| format!("Could not save the file: {e}"))?;
    build_state()
}

#[tauri::command]
fn import_hermes(category: String, slug: String, target: String) -> Result<AppState, String> {
    ensure_store()?;
    if !safe_segment(&target) || !skills_store().join(&target).is_dir() {
        return Err("Pick one of the store categories.".into());
    }
    let from = skill_dir("hermes", &category, &slug)?;
    let to = skills_store().join(&target).join(&slug);
    if to.exists() {
        return Err(format!("{slug} is already in {target}."));
    }
    copy_dir(&from, &to).map_err(|e| format!("Could not copy the skill: {e}"))?;
    if let Err(error) = link_skill(&slug, &to) {
        let _ = fs::remove_dir_all(&to);
        return Err(error);
    }
    build_state()
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Kilgharrah only opens web links.".into());
    }
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(&url).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", &url]).status()
    } else {
        Command::new("xdg-open").arg(&url).status()
    };
    status.map_err(|e| format!("Could not open the browser: {e}"))?;
    Ok(())
}

#[tauri::command]
fn reveal_store() -> Result<(), String> {
    ensure_store()?;
    let target = store_root();
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(&target).status()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer").arg(&target).status()
    } else {
        Command::new("xdg-open").arg(&target).status()
    };
    status.map_err(|e| format!("Could not open the store: {e}"))?;
    Ok(())
}

fn main() {
    let _ = ensure_store();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            snapshot,
            add_skill,
            delete_skill,
            set_skill_enabled,
            move_skill,
            save_server,
            delete_server,
            search_market,
            preview_market,
            install_market,
            list_skill_files,
            read_skill_file,
            write_skill_file,
            import_hermes,
            open_url,
            reveal_store
        ])
        .run(tauri::generate_context!())
        .expect("Kilgharrah failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_is_stable() {
        assert_eq!(slugify("Postgres Backups!"), "postgres-backups");
        assert_eq!(slugify("  --Weird  name--  "), "weird-name");
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn relative_paths_reject_escapes() {
        assert!(safe_relative("SKILL.md").is_some());
        assert!(safe_relative("agents/openai.yaml").is_some());
        assert!(safe_relative("../etc/passwd").is_none());
        assert!(safe_relative("/tmp/file").is_none());
        assert!(safe_relative("a/../../b").is_none());
    }

    #[test]
    fn frontmatter_reads_name_and_description() {
        let text = "---\nname: \"Demo\"\ndescription: A small test skill\n---\n\nBody\n";
        let (name, description) = parse_frontmatter(text);
        assert_eq!(name.as_deref(), Some("Demo"));
        assert_eq!(description.as_deref(), Some("A small test skill"));
    }

    #[test]
    fn categories_are_titled() {
        assert_eq!(titleize("design"), "Design");
        assert_eq!(titleize("data-ops"), "Data ops");
    }

    #[test]
    fn market_buckets_follow_topics() {
        assert_eq!(bucket_for("figma tailwind", "Layout kit", "UI work"), "design");
        assert_eq!(bucket_for("", "News digest", "Search the web for news"), "research");
        assert_eq!(bucket_for("", "Quiet helper", "Nothing in particular"), "other");
    }

    #[test]
    fn short_words_need_boundaries() {
        assert!(!mentions("email detail", "ai"));
        assert!(mentions("ai tools", "ai"));
        assert!(mentions("codebase work", "code"));
    }

    #[test]
    fn hermes_skills_are_scanned_and_deduped() {
        let root = std::env::temp_dir().join(format!("kilgharrah-scan-{}", std::process::id()));
        let store = root.join("store");
        let hermes = root.join("hermes");
        let write = |path: &Path, name: &str| {
            fs::create_dir_all(path).unwrap();
            fs::write(path.join("SKILL.md"), format!("---\nname: {name}\n---\n\nBody\n")).unwrap();
        };
        write(&store.join("design/apple-design"), "Apple");
        write(&hermes.join("creative/p5js"), "p5js");
        write(&hermes.join("kilgharrah/apple-design"), "Apple copy");

        let skills = scan_roots(store, hermes);
        let apple: Vec<&Skill> = skills.iter().filter(|s| s.slug == "apple-design").collect();
        assert_eq!(apple.len(), 1, "the store copy wins the duplicate slug");
        assert_eq!(apple[0].source, "store");
        assert_eq!(apple[0].category, "design");
        let p5 = skills.iter().find(|s| s.slug == "p5js").expect("hermes skill listed");
        assert_eq!(p5.source, "hermes");
        assert_eq!(p5.category, "creative");
        assert_eq!(p5.id, "hermes:creative:p5js");

        let _ = fs::remove_dir_all(&root);
    }
}
