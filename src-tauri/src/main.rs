#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use tauri::Emitter;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

const BASE: &str = "https://clawhub.ai";
const AGENTIC_BASE: &str = "https://agenticskills.io";
const AGENTIC_TTL: Duration = Duration::from_secs(600);
const MARKET_TTL: Duration = Duration::from_secs(300);
const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 12 * 1024 * 1024;
const MAX_READ: u64 = 512 * 1024;
const PREVIEW_CHARS: usize = 20_000;
const MAX_SCAN_DEPTH: usize = 4;
const META_FILE: &str = ".kilgharrah.json";

/// Agent skill folders Kilgharrah can mirror into. The main agent's folder is
/// always linked; these are opt-in from Settings.
const AGENT_TARGETS: [(&str, &str, &str); 3] = [
    ("claude", "Claude Code", ".claude/skills"),
    ("codex", "Codex CLI", ".codex/skills"),
    ("gemini", "Gemini CLI", ".gemini/skills"),
];

/// Files that run code if an agent follows the skill. They are reported, not
/// executed or scanned for behaviour.
const SCRIPT_EXTENSIONS: [&str; 10] = [
    "sh", "bash", "zsh", "ps1", "bat", "cmd", "py", "js", "ts", "rb",
];

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
    version: Option<String>,
    origin: String,
    installed_at: u64,
    modified: bool,
    scripts: usize,
    profile: Option<String>,
}

/// Written next to SKILL.md when Kilgharrah installs a skill. It answers what
/// was installed, from where, at which version, and whether the files changed.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstalledMeta {
    name: String,
    origin: String,
    upstream: Option<String>,
    version: Option<String>,
    installed_at: u64,
    hash: String,
    #[serde(default)]
    profile: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AgentTarget {
    id: String,
    name: String,
    path: String,
    enabled: bool,
    linked: usize,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    agent_targets: Vec<String>,
    #[serde(default)]
    onboarded: bool,
    #[serde(default = "default_scope")]
    scope: String,
    #[serde(default)]
    project_root: Option<String>,
    #[serde(default = "default_agent")]
    agent: String,
    #[serde(default)]
    profile: Option<String>,
    /// Names Kilgharrah wrote into each mirrored config, so a later change can
    /// remove its own entries without touching servers the user added by hand.
    #[serde(default)]
    mirror_state: BTreeMap<String, Vec<String>>,
}

fn default_scope() -> String {
    "home".into()
}

fn default_agent() -> String {
    "freebuff".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            agent_targets: Vec::new(),
            onboarded: false,
            scope: default_scope(),
            project_root: None,
            agent: default_agent(),
            profile: None,
            mirror_state: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SetupView {
    onboarded: bool,
    scope: String,
    project_root: Option<String>,
    agent: String,
    agent_name: String,
    profile: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SkillUpdate {
    category: String,
    slug: String,
    name: String,
    origin: String,
    current: Option<String>,
    latest: Option<String>,
    status: String,
    detail: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Check {
    id: String,
    label: String,
    status: String,
    detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostics {
    ok: bool,
    checks: Vec<Check>,
    checked_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileFile {
    path: String,
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileSkill {
    name: String,
    category: String,
    slug: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    files: Vec<ProfileFile>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileServer {
    name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    skills: Vec<ProfileSkill>,
    #[serde(default)]
    servers: Vec<ProfileServer>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileInfo {
    name: String,
    path: Option<String>,
    skills: usize,
    servers: usize,
    server_names: Vec<String>,
    skill_names: Vec<String>,
    excluded_names: Vec<String>,
    location: String,
    builtin: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateReport {
    state: AppState,
    updated: usize,
    failed: Vec<String>,
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
    agent_targets: Vec<AgentTarget>,
    setup: SetupView,
    profiles: Vec<ProfileInfo>,
}

#[derive(Serialize, Clone)]
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
    stars: u64,
    rank: u32,
    grade: String,
    market: String,
    category: String,
    topics: Vec<String>,
    installable: bool,
    command: Option<String>,
}

// ---------------------------------------------------------------- locations

fn home_dir() -> PathBuf {
    // Some Windows shells export HOME as a POSIX path (`/c/Users/me`) that no
    // Windows file API accepts, so prefer the native variable there.
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
    } else {
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    };
    PathBuf::from(home.unwrap_or_default())
}

fn agents_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("KILGHARRAH_HOME") {
        return PathBuf::from(custom);
    }
    home_dir().join(".agents")
}

/// The first Hermes skills folder that exists. Hermes itself honours
/// HERMES_HOME, and its home may sit next to the skills folder or point at it.
fn hermes_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("KILGHARRAH_HERMES") {
        return PathBuf::from(custom);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HERMES_HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("skills"));
        candidates.push(home);
    }
    candidates.push(home_dir().join(".hermes").join("skills"));
    candidates.push(home_dir().join("hermes").join("skills"));
    candidates
        .iter()
        .find(|path| path.is_dir())
        .cloned()
        .unwrap_or_else(|| home_dir().join(".hermes").join("skills"))
}

fn store_root() -> PathBuf {
    agents_root().join("kilgharrah")
}

fn skills_store() -> PathBuf {
    store_root().join("skills")
}

fn expand_path(value: &str) -> PathBuf {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if trimmed == "~" {
        return home_dir();
    }
    PathBuf::from(trimmed)
}

fn base_dir(settings: &Settings) -> PathBuf {
    if settings.scope == "project" {
        settings
            .project_root
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(expand_path)
            .unwrap_or_else(home_dir)
    } else {
        home_dir()
    }
}

fn skills_relative(agent: &str) -> &'static str {
    match agent {
        "claude" => ".claude/skills",
        "codex" => ".codex/skills",
        "gemini" => ".gemini/skills",
        _ => ".agents/skills",
    }
}

fn agent_name(agent: &str) -> &'static str {
    match agent {
        "claude" => "Claude Code",
        "codex" => "Codex CLI",
        "gemini" => "Gemini CLI",
        _ => "Freebuff CLI",
    }
}

fn primary_skills_root(settings: &Settings) -> PathBuf {
    if settings.scope == "home" && settings.agent == "freebuff" {
        agents_root().join("skills")
    } else {
        base_dir(settings).join(skills_relative(&settings.agent))
    }
}

fn links_root() -> PathBuf {
    primary_skills_root(&read_settings())
}

fn mcp_path() -> PathBuf {
    let settings = read_settings();
    if settings.scope == "project" {
        base_dir(&settings).join(".agents").join("mcp.json")
    } else {
        agents_root().join("mcp.json")
    }
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- install records

fn meta_path(dir: &Path) -> PathBuf {
    dir.join(META_FILE)
}

fn read_meta(dir: &Path) -> Option<InstalledMeta> {
    let text = fs::read_to_string(meta_path(dir)).ok()?;
    serde_json::from_str::<InstalledMeta>(&text).ok()
}

fn write_meta(dir: &Path, meta: &InstalledMeta) -> Result<(), String> {
    let text = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Could not encode the install record: {e}"))?;
    fs::write(meta_path(dir), format!("{text}\n"))
        .map_err(|e| format!("Could not write the install record: {e}"))
}

fn collect_hash_input(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    if out.len() >= 4000 {
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
        if name == META_FILE || name == ".DS_Store" {
            continue;
        }
        if path.is_dir() {
            collect_hash_input(&path, base, out);
        } else if let Ok(relative) = path.strip_prefix(base) {
            out.push((relative.to_string_lossy().to_string(), path));
        }
    }
}

/// A stable digest over every installed file and its relative path. The install
/// record itself is excluded so writing it does not change the digest.
fn hash_skill(dir: &Path) -> String {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_hash_input(dir, dir, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0u8]);
        if let Ok(bytes) = fs::read(path) {
            hasher.update(&bytes);
        }
        hasher.update([0u8]);
    }
    format!("{:x}", hasher.finalize())
}

fn count_scripts(dir: &Path, depth: usize) -> usize {
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
            total += count_scripts(&path, depth - 1);
        } else if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
            if SCRIPT_EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(ext)) {
                total += 1;
            }
        }
    }
    total
}

// ---------------------------------------------------------------- settings

fn settings_path() -> PathBuf {
    store_root().join("settings.json")
}

fn read_settings() -> Settings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default()
}

fn write_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create the store: {e}"))?;
    }
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Could not encode the settings: {e}"))?;
    fs::write(&path, format!("{text}\n")).map_err(|e| format!("Could not save the settings: {e}"))
}

// ---------------------------------------------------------------- dragon

fn dragon_path() -> PathBuf {
    store_root().join("dragon.txt")
}

fn build_dragon_text(frames: &[String]) -> Result<String, String> {
    if frames.is_empty() {
        return Err("Keep at least one frame.".into());
    }
    if frames.len() > 200 {
        return Err("That is more than 200 frames.".into());
    }
    let mut clean = Vec::with_capacity(frames.len());
    for (index, frame) in frames.iter().enumerate() {
        let frame = frame.replace("\r\n", "\n").replace('\r', "\n");
        // Keep blank rows at the end of a frame. Several frames rest the dragon
        // near the top, so their bottom rows are empty on purpose.
        let rows: Vec<&str> = frame.split('\n').collect();
        if rows.len() != 23 {
            return Err(format!(
                "Frame {} needs 23 lines, not {}.",
                index + 1,
                rows.len()
            ));
        }
        if rows.iter().any(|row| row.chars().count() > 70) {
            return Err(format!("Frame {} has a line wider than 70 columns.", index + 1));
        }
        if rows.iter().any(|row| row.contains('\t')) {
            return Err(format!("Frame {} contains a tab. Use spaces.", index + 1));
        }
        clean.push(rows.join("\n"));
    }
    Ok(clean.join("\n~~~~~~~~\n"))
}

fn read_dragon_at(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not read the dragon frames: {error}")),
    }
}

fn save_dragon_at(path: &Path, frames: &[String]) -> Result<(), String> {
    let text = build_dragon_text(frames)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create the store: {e}"))?;
    }
    fs::write(path, format!("{text}\n"))
        .map_err(|e| format!("Could not save the dragon frames: {e}"))
}

#[tauri::command]
fn read_dragon() -> Result<Option<String>, String> {
    read_dragon_at(&dragon_path())
}

#[tauri::command]
fn save_dragon(frames: Vec<String>) -> Result<(), String> {
    ensure_store()?;
    save_dragon_at(&dragon_path(), &frames)
}

// ---------------------------------------------------------------- profiles

fn profiles_dir() -> PathBuf {
    store_root().join("profiles")
}

fn hermes_mcp_path() -> PathBuf {
    hermes_root()
        .parent()
        .map(|parent| parent.join("mcp.json"))
        .unwrap_or_else(|| home_dir().join(".hermes").join("mcp.json"))
}

fn hermes_config_path() -> PathBuf {
    hermes_root()
        .parent()
        .map(|parent| parent.join("config.yaml"))
        .unwrap_or_else(|| home_dir().join(".hermes").join("config.yaml"))
}

fn yaml_text(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(text) => Some(text.clone()),
        serde_yaml::Value::Number(number) => Some(number.to_string()),
        serde_yaml::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// Hermes keeps its servers under `mcp_servers` in `config.yaml`. Only enabled
/// entries with a command can be copied into the Kilgharrah library.
fn servers_from_hermes_yaml(text: &str) -> Option<Vec<ProfileServer>> {
    let doc: serde_yaml::Value = serde_yaml::from_str(text).ok()?;
    let entry = doc
        .as_mapping()?
        .get(&serde_yaml::Value::String("mcp_servers".into()))?;
    let map = entry.as_mapping()?;
    let mut out = Vec::new();
    for (name, config) in map {
        let (Some(name), Some(config)) = (name.as_str(), config.as_mapping()) else {
            continue;
        };
        let field = |key: &str| config.get(&serde_yaml::Value::String(key.into()));
        if field("enabled").and_then(|value| value.as_bool()) == Some(false) {
            continue;
        }
        let command = field("command").and_then(yaml_text).unwrap_or_default();
        if command.is_empty() {
            continue;
        }
        let args = field("args")
            .and_then(|value| value.as_sequence())
            .map(|items| items.iter().filter_map(yaml_text).collect())
            .unwrap_or_default();
        let env = field("env")
            .and_then(|value| value.as_mapping())
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| {
                        Some((key.as_str()?.to_string(), yaml_text(value)?))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.push(ProfileServer {
            name: name.to_string(),
            command,
            args,
            env,
        });
    }
    Some(out)
}

fn servers_from_hermes_json(text: &str) -> Vec<ProfileServer> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(map) = value
        .get("mcpServers")
        .or_else(|| value.get("mcp_servers"))
        .and_then(|entry| entry.as_object())
    else {
        return Vec::new();
    };
    map.iter()
        .filter(|(_, entry)| entry.get("enabled").and_then(|value| value.as_bool()) != Some(false))
        .map(|(name, entry)| ProfileServer {
            name: name.clone(),
            command: entry
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            args: entry
                .get("args")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|value| value.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            env: entry
                .get("env")
                .and_then(|value| value.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|text| (key.clone(), text.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .filter(|server| !server.command.is_empty())
        .collect()
}

fn hermes_servers() -> Vec<ProfileServer> {
    if let Ok(text) = fs::read_to_string(hermes_config_path()) {
        if let Some(servers) = servers_from_hermes_yaml(&text) {
            return servers;
        }
    }
    fs::read_to_string(hermes_mcp_path())
        .map(|text| servers_from_hermes_json(&text))
        .unwrap_or_default()
}

fn walk_profile_files(dir: &Path, base: &Path, depth: usize, out: &mut Vec<ProfileFile>) {
    if depth > 4 || out.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name == META_FILE || name == ".DS_Store" {
            continue;
        }
        if path.is_dir() {
            walk_profile_files(&path, base, depth + 1, out);
        } else if let Ok(relative) = path.strip_prefix(base) {
            if let Ok(content) = fs::read_to_string(&path) {
                out.push(ProfileFile {
                    path: relative.to_string_lossy().to_string(),
                    content,
                });
            }
        }
    }
}

fn snapshot_profile(
    name: &str,
    candidates: Vec<(PathBuf, String, String)>,
    servers: Vec<ProfileServer>,
) -> Result<Profile, String> {
    let mut skills = Vec::new();
    let mut total = 0usize;
    for (dir, category, _) in candidates {
        let slug = dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
        let (display, description) = parse_frontmatter(&text);
        let mut files = Vec::new();
        walk_profile_files(&dir, &dir, 0, &mut files);
        total += files.iter().map(|file| file.content.len()).sum::<usize>();
        if total > MAX_BYTES as usize * 2 {
            return Err("That profile is larger than the export limit.".into());
        }
        skills.push(ProfileSkill {
            name: display
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| titleize(&slug)),
            category,
            slug,
            description: description.unwrap_or_default(),
            files,
        });
    }
    skills.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.name.cmp(&b.name)));
    Ok(Profile {
        name: name.to_string(),
        created_at: now_secs(),
        skills,
        servers,
    })
}

fn profile_snapshot(name: &str) -> Result<Profile, String> {
    let mut candidates = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    let servers = read_servers()
        .into_iter()
        .map(|server| ProfileServer {
            name: server.name,
            command: server.command,
            args: server.args,
            env: server.env,
        })
        .collect();
    snapshot_profile(name, candidates, servers)
}

/// The built-in profile is read from the Hermes install itself, so its file can
/// be exported before anything has been copied into the Kilgharrah library.
fn emrys_profile_snapshot() -> Result<Profile, String> {
    let (candidates, _) = portable_hermes_candidates();
    snapshot_profile("Emrys", candidates, hermes_servers())
}

/// The Hermes install split into the skills that run on macOS and Windows and
/// the ones that do not. Emrys only ever carries the first group; the second
/// is reported so the profile can say what it left out.
fn portable_hermes_candidates() -> (Vec<(PathBuf, String, String)>, Vec<String>) {
    let mut all = Vec::new();
    collect_skills(hermes_root(), "hermes", &mut all);
    let mut keep = Vec::new();
    let mut skipped = Vec::new();
    for (dir, category, source) in all {
        let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
        let (display, _) = parse_frontmatter(&text);
        let slug = dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let name = display
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| titleize(&slug));
        if is_cross_platform(&frontmatter_platforms(&text)) {
            keep.push((dir, category, source));
        } else {
            skipped.push(name);
        }
    }
    skipped.sort_by_key(|value| value.to_lowercase());
    (keep, skipped)
}

fn profile_destination(name: &str, path: Option<String>) -> PathBuf {
    let stem = {
        let value = slugify(name);
        if value.is_empty() { "profile".to_string() } else { value }
    };
    match path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(value) => {
            let mut target = expand_path(&value);
            if target.extension().and_then(|value| value.to_str()) != Some("json") {
                target.set_extension("json");
            }
            target
        }
        None => profiles_dir().join(format!("{stem}.json")),
    }
}

/// Write a profile as JSON. With no path it lands in the profiles folder; with
/// a path it can go anywhere, so it can travel to another machine on a stick or
/// a sync folder.
#[tauri::command]
fn export_profile(name: String, path: Option<String>) -> Result<String, String> {
    ensure_store()?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the profile a name.".into());
    }
    let profile = if name.eq_ignore_ascii_case("emrys") || name.eq_ignore_ascii_case("hermes") {
        emrys_profile_snapshot()?
    } else {
        profile_snapshot(name)?
    };
    if profile.skills.is_empty() && profile.servers.is_empty() {
        return Err(
            "That profile has no skills or MCP servers to export. Check the folder shown on the profile."
                .into(),
        );
    }
    let target = profile_destination(name, path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create the profiles folder: {e}"))?;
    }
    let text = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("Could not encode the profile: {e}"))?;
    fs::write(&target, format!("{text}\n"))
        .map_err(|e| format!("Could not save the profile: {e}"))?;
    Ok(target.display().to_string())
}

/// Read a profile file from anywhere, keep a copy in the profiles folder, then
/// install its skills and MCP servers. This is the one-step path for a clean
/// machine: pick the exported file and the library is ready.
#[tauri::command]
fn import_profile(path: String) -> Result<AppState, String> {
    ensure_store()?;
    let source = expand_path(path.trim());
    let text =
        fs::read_to_string(&source).map_err(|e| format!("Could not open that profile: {e}"))?;
    let profile: Profile = serde_json::from_str(&text)
        .map_err(|e| format!("That file is not a Kilgharrah profile: {e}"))?;
    if profile.name.trim().is_empty() {
        return Err("That profile has no name.".into());
    }
    if profile.skills.is_empty() && profile.servers.is_empty() {
        return Err("That profile is empty.".into());
    }
    let dir = profiles_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create the profiles folder: {e}"))?;
    let stem = {
        let value = slugify(&profile.name);
        if value.is_empty() { "profile".to_string() } else { value }
    };
    let text = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("Could not encode the profile: {e}"))?;
    fs::write(dir.join(format!("{stem}.json")), format!("{text}\n"))
        .map_err(|e| format!("Could not save the imported profile: {e}"))?;
    install_profile(&profile)?;
    mirror_mcp()?;
    build_state()
}

fn profile_skill_names(candidates: &[(PathBuf, String, String)]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (dir, _, _) in candidates {
        let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
        let (display, _) = parse_frontmatter(&text);
        let slug = dir.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        let name = display
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| titleize(slug));
        if !names.iter().any(|existing| existing.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    names.sort_by_key(|value| value.to_lowercase());
    names
}

fn list_profiles() -> Vec<ProfileInfo> {
    let (candidates, excluded) = portable_hermes_candidates();
    let servers = hermes_servers();
    let skill_names = profile_skill_names(&candidates);
    let mut out = vec![ProfileInfo {
        name: "Emrys".into(),
        path: None,
        skills: candidates.len(),
        servers: servers.len(),
        server_names: servers.iter().map(|server| server.name.clone()).collect(),
        skill_names,
        excluded_names: excluded,
        location: hermes_root().display().to_string(),
        builtin: true,
    }];
    if let Ok(entries) = fs::read_dir(profiles_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(profile) = serde_json::from_str::<Profile>(&text) else {
                continue;
            };
            out.push(ProfileInfo {
                name: profile.name,
                path: Some(path.display().to_string()),
                skills: profile.skills.len(),
                servers: profile.servers.len(),
                server_names: profile
                    .servers
                    .iter()
                    .map(|server| server.name.clone())
                    .collect(),
                skill_names: profile.skills.iter().map(|skill| skill.name.clone()).collect(),
                excluded_names: Vec::new(),
                location: path.display().to_string(),
                builtin: false,
            });
        }
    }
    out.sort_by(|a, b| b.builtin.cmp(&a.builtin).then_with(|| a.name.cmp(&b.name)));
    out
}

fn merge_profile_servers(servers: &[ProfileServer]) -> Result<(), String> {
    if servers.is_empty() {
        return Ok(());
    }
    let mut value = read_mcp();
    if !value.get("mcpServers").map(|entry| entry.is_object()).unwrap_or(false) {
        value["mcpServers"] = json!({});
    }
    if let Some(map) = value.get_mut("mcpServers").and_then(|entry| entry.as_object_mut()) {
        for server in servers {
            if map.contains_key(&server.name) {
                continue;
            }
            map.insert(
                server.name.clone(),
                json!({
                    "command": server.command,
                    "args": server.args,
                    "env": server.env,
                }),
            );
        }
    }
    write_mcp(&value)
}

fn install_profile(profile: &Profile) -> Result<usize, String> {
    ensure_store()?;
    let mut installed = 0usize;
    for skill in &profile.skills {
        if !safe_segment(&skill.category) || !safe_segment(&skill.slug) {
            continue;
        }
        let dir = skills_store().join(&skill.category).join(&skill.slug);
        if dir.exists() || skill.files.is_empty() {
            continue;
        }
        fs::create_dir_all(&dir).map_err(|e| format!("Could not create the skill folder: {e}"))?;
        let mut wrote = false;
        for file in &skill.files {
            let Some(relative) = safe_relative(&file.path) else {
                continue;
            };
            let target = dir.join(&relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Could not create a folder: {e}"))?;
            }
            fs::write(&target, &file.content)
                .map_err(|e| format!("Could not write a profile file: {e}"))?;
            wrote = true;
        }
        if !wrote {
            let _ = fs::remove_dir_all(&dir);
            continue;
        }
        write_meta(
            &dir,
            &InstalledMeta {
                name: skill.name.clone(),
                origin: "profile".into(),
                upstream: None,
                version: None,
                installed_at: now_secs(),
                hash: hash_skill(&dir),
                profile: Some(profile.name.clone()),
            },
        )?;
        link_skill(&skill.slug, &dir)?;
        installed += 1;
    }
    merge_profile_servers(&profile.servers)?;
    let mut settings = read_settings();
    settings.profile = Some(profile.name.clone());
    write_settings(&settings)?;
    Ok(installed)
}

fn apply_emrys() -> Result<usize, String> {
    ensure_store()?;
    let (candidates, _) = portable_hermes_candidates();
    let mut installed = 0usize;
    for (from, category, _) in candidates {
        let Some(slug) = from.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let category = if safe_segment(&category) && skills_store().join(&category).is_dir() {
            category
        } else {
            "other".to_string()
        };
        let to = skills_store().join(&category).join(slug);
        if to.exists() {
            continue;
        }
        copy_dir(&from, &to).map_err(|e| format!("Could not copy a Hermes skill: {e}"))?;
        write_meta(
            &to,
            &InstalledMeta {
                name: titleize(slug),
                origin: "profile".into(),
                upstream: None,
                version: None,
                installed_at: now_secs(),
                hash: hash_skill(&to),
                profile: Some("Emrys".into()),
            },
        )?;
        link_skill(slug, &to)?;
        installed += 1;
    }
    let servers = hermes_servers();
    merge_profile_servers(&servers)?;
    let mut settings = read_settings();
    settings.profile = Some("Emrys".into());
    write_settings(&settings)?;
    Ok(installed)
}

fn apply_profile_named(name: &str) -> Result<usize, String> {
    if name.eq_ignore_ascii_case("emrys") || name.eq_ignore_ascii_case("hermes") {
        return apply_emrys();
    }
    let wanted = name.trim().to_string();
    let mut found: Option<Profile> = None;
    if let Ok(entries) = fs::read_dir(profiles_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(profile) = serde_json::from_str::<Profile>(&text) else {
                continue;
            };
            if profile.name == wanted || slugify(&profile.name) == slugify(&wanted) {
                found = Some(profile);
                break;
            }
        }
    }
    let profile = found.ok_or("That profile is not in the profiles folder.")?;
    install_profile(&profile)
}

#[tauri::command(async)]
fn apply_profile(name: String) -> Result<AppState, String> {
    apply_profile_named(&name)?;
    mirror_mcp()?;
    build_state()
}

/// Find the profile file whose stored name matches, then remove it. Returns the
/// deleted path so the caller can report it.
fn remove_profile_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let wanted = name.trim();
    if wanted.is_empty() {
        return Err("Pick a profile to delete.".into());
    }
    let mut target: Option<PathBuf> = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(profile) = serde_json::from_str::<Profile>(&text) else {
                continue;
            };
            if profile.name == wanted || slugify(&profile.name) == slugify(wanted) {
                target = Some(path);
                break;
            }
        }
    }
    let path = target.ok_or("That profile is not in the profiles folder.")?;
    fs::remove_file(&path).map_err(|e| format!("Could not delete the profile: {e}"))?;
    Ok(path)
}

/// Remove an exported profile file. The built-in Emrys profile is derived from
/// the Hermes install, so it has no file to delete and is refused. Skills already
/// copied into the library stay there.
#[tauri::command]
fn delete_profile(name: String) -> Result<AppState, String> {
    let wanted = name.trim().to_string();
    if wanted.is_empty() {
        return Err("Pick a profile to delete.".into());
    }
    if wanted.eq_ignore_ascii_case("emrys") || wanted.eq_ignore_ascii_case("hermes") {
        return Err("The built-in Emrys profile cannot be deleted.".into());
    }
    remove_profile_file(&profiles_dir(), &wanted)?;
    let mut settings = read_settings();
    let active = settings
        .profile
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case(&wanted))
        .unwrap_or(false);
    if active {
        settings.profile = None;
        write_settings(&settings)?;
    }
    build_state()
}

// ------------------------------------------------------------------- scan

/// A skill that exists on this machine but is not in the library yet.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiscoveredSkill {
    id: String,
    name: String,
    slug: String,
    description: String,
    category: String,
    source: String,
    path: String,
    files: usize,
    platforms: Vec<String>,
    portable: bool,
    in_library: bool,
}

/// Folders another tool may already keep skills in, with the label to show.
fn installed_skill_roots() -> Vec<(PathBuf, String)> {
    let mut roots: Vec<(PathBuf, String)> = Vec::new();
    let mut add = |path: PathBuf, source: &str| {
        if path.is_dir() && !roots.iter().any(|(seen, _)| seen == &path) {
            roots.push((path, source.to_string()));
        }
    };
    add(agents_root().join("skills"), "Freebuff CLI");
    add(home_dir().join(".claude").join("skills"), "Claude Code");
    add(home_dir().join(".codex").join("skills"), "Codex CLI");
    add(home_dir().join(".gemini").join("skills"), "Gemini CLI");
    add(hermes_root(), "Hermes");
    let settings = read_settings();
    if let Some(root) = settings
        .project_root
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let base = expand_path(root);
        add(base.join(".agents").join("skills"), "Project");
        add(base.join(".claude").join("skills"), "Project");
    }
    roots
}

/// Every skill already in the library, as slug and category.
fn library_lookup() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let Ok(categories) = fs::read_dir(skills_store()) else {
        return out;
    };
    for category in categories.flatten() {
        let name = category.file_name().to_string_lossy().to_string();
        let Ok(children) = fs::read_dir(category.path()) else {
            continue;
        };
        for child in children.flatten() {
            if child.path().join("SKILL.md").is_file() {
                out.push((child.file_name().to_string_lossy().to_string(), name.clone()));
            }
        }
    }
    out
}

fn library_has(slug: &str) -> bool {
    library_lookup().iter().any(|(known, _)| known == slug)
}

/// Look through the folders other tools keep skills in. A symlink is skipped:
/// on macOS and Linux the mirrors point back into the library, so following
/// them would report the library as newly found.
#[tauri::command]
fn scan_installed_skills() -> Result<Vec<DiscoveredSkill>, String> {
    ensure_store()?;
    let library = library_lookup();
    let mut out: Vec<DiscoveredSkill> = Vec::new();
    for (root, source) in installed_skill_roots() {
        let mut candidates = Vec::new();
        collect_skills(root, "scan", &mut candidates);
        for (dir, category, _) in candidates {
            if fs::symlink_metadata(&dir)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false)
            {
                continue;
            }
            let slug = dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            if slug.is_empty() {
                continue;
            }
            let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
            let (display, description) = parse_frontmatter(&text);
            let platforms = frontmatter_platforms(&text);
            let known = library.iter().find(|(existing, _)| existing == &slug);
            out.push(DiscoveredSkill {
                id: format!("{source}:{}", dir.display()),
                name: display
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| titleize(&slug)),
                slug,
                description: description.unwrap_or_default(),
                category: known
                    .map(|(_, existing)| existing.clone())
                    .unwrap_or(category),
                source: source.clone(),
                path: dir.display().to_string(),
                files: count_files(&dir, 3),
                portable: is_cross_platform(&platforms),
                platforms,
                in_library: known.is_some(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.in_library
            .cmp(&b.in_library)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out.dedup_by(|a, b| a.slug == b.slug && a.source == b.source);
    Ok(out)
}

/// Copy scanned skills into the library, keeping their category folder when the
/// store already has one by that name. Only folders the scan would have offered
/// are accepted, so a crafted path cannot pull in arbitrary files.
#[tauri::command]
fn import_installed_skills(paths: Vec<String>) -> Result<AppState, String> {
    ensure_store()?;
    let roots: Vec<PathBuf> = installed_skill_roots()
        .into_iter()
        .map(|(root, _)| root)
        .collect();
    let mut imported = 0usize;
    for raw in paths {
        let dir = PathBuf::from(&raw);
        if !roots.iter().any(|root| dir.starts_with(root)) {
            continue;
        }
        let Some(slug) = dir
            .file_name()
            .and_then(|value| value.to_str())
            .map(String::from)
        else {
            continue;
        };
        if !safe_segment(&slug) || !dir.join("SKILL.md").is_file() || library_has(&slug) {
            continue;
        }
        let parent = dir
            .parent()
            .and_then(|value| value.file_name())
            .and_then(|value| value.to_str())
            .unwrap_or("other")
            .to_string();
        let category = if safe_segment(&parent) && skills_store().join(&parent).is_dir() {
            parent
        } else {
            "other".to_string()
        };
        let to = skills_store().join(&category).join(&slug);
        if to.exists() {
            continue;
        }
        copy_dir(&dir, &to).map_err(|e| format!("Could not copy {slug}: {e}"))?;
        write_meta(
            &to,
            &InstalledMeta {
                name: titleize(&slug),
                origin: "scan".into(),
                upstream: Some(dir.display().to_string()),
                version: None,
                installed_at: now_secs(),
                hash: hash_skill(&to),
                profile: None,
            },
        )?;
        link_skill(&slug, &to)?;
        imported += 1;
    }
    if imported == 0 {
        return Err("Nothing new to import: those skills are already in the library.".into());
    }
    build_state()
}

/// Finish the first-run setup. The chosen project folder and agent decide where
/// skills and MCP configs live, so existing links move to the new root.
#[tauri::command]
fn complete_setup(
    project_root: Option<String>,
    agent: String,
    profile: Option<String>,
) -> Result<AppState, String> {
    ensure_store()?;
    let agent = agent.trim().to_lowercase();
    if !["freebuff", "claude", "codex", "gemini"].contains(&agent.as_str()) {
        return Err("Pick one of the agents in the list.".into());
    }
    let root_text = project_root.unwrap_or_default().trim().to_string();
    let (scope, project) = if root_text.is_empty() {
        ("home".to_string(), None)
    } else {
        let path = expand_path(&root_text);
        if !path.is_dir() {
            return Err("That project folder does not exist.".into());
        }
        ("project".to_string(), Some(path.display().to_string()))
    };

    let before = read_settings();
    let old_root = primary_skills_root(&before);
    let enabled: Vec<String> = scan_store_slugs()
        .into_iter()
        .filter(|slug| fs::symlink_metadata(old_root.join(slug)).is_ok())
        .collect();

    let mut settings = before;
    settings.onboarded = true;
    settings.scope = scope;
    settings.project_root = project;
    settings.agent = agent.clone();
    settings.agent_targets.retain(|id| *id != agent);
    write_settings(&settings)?;

    let new_root = primary_skills_root(&settings);
    fs::create_dir_all(&new_root)
        .map_err(|e| format!("Could not create {}: {e}", new_root.display()))?;
    if new_root != old_root {
        for slug in &enabled {
            unlink_from(&old_root, slug);
        }
    }
    let mut candidates = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    for (dir, _, _) in candidates {
        let Some(slug) = dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if enabled.iter().any(|value| value == slug) {
            let _ = link_into(&new_root, slug, &dir);
        }
    }

    if let Some(name) = profile {
        let name = name.trim();
        if !name.is_empty() && !name.eq_ignore_ascii_case("none") {
            apply_profile_named(name)?;
        }
    }
    mirror_mcp()?;
    build_state()
}

fn target_root(relative: &str) -> PathBuf {
    base_dir(&read_settings()).join(relative)
}

fn enabled_targets() -> Vec<(&'static str, &'static str, PathBuf)> {
    let settings = read_settings();
    AGENT_TARGETS
        .iter()
        .filter(|(id, _, _)| *id != settings.agent)
        .filter(|(id, _, _)| settings.agent_targets.iter().any(|value| value == id))
        .map(|(id, name, relative)| (*id, *name, target_root(relative)))
        .collect()
}

fn agent_target_state() -> Vec<AgentTarget> {
    let settings = read_settings();
    let enabled_slugs: Vec<String> = scan_store_slugs()
        .into_iter()
        .filter(|slug| is_enabled(slug))
        .collect();
    AGENT_TARGETS
        .iter()
        .map(|(id, name, relative)| {
            let root = target_root(relative);
            let enabled = settings.agent != *id
                && settings.agent_targets.iter().any(|value| value == id);
            AgentTarget {
                id: (*id).to_string(),
                name: (*name).to_string(),
                path: root.display().to_string(),
                enabled,
                linked: if enabled {
                    enabled_slugs
                        .iter()
                        .filter(|slug| fs::symlink_metadata(root.join(slug)).is_ok())
                        .count()
                } else {
                    0
                },
            }
        })
        .collect()
}

/// Slugs that exist in the store, whether enabled or not.
fn scan_store_slugs() -> Vec<String> {
    let mut candidates = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    candidates
        .into_iter()
        .filter_map(|(dir, _, _)| dir.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect()
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

/// The `platforms:` list a skill declares in its frontmatter, lowercased.
/// Skills that declare none are treated as portable, which is true of the ones
/// written for this repository.
fn frontmatter_platforms(text: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next().map(|line| line.trim()) != Some("---") {
        return Vec::new();
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        let Some(value) = trimmed.strip_prefix("platforms:") else {
            continue;
        };
        return value
            .trim()
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|part| {
                part.trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_lowercase()
            })
            .filter(|part| !part.is_empty())
            .collect();
    }
    Vec::new()
}

/// A profile that travels between two machines has to work on both, so a skill
/// locked to a single platform is left out: an iMessage skill is no use on
/// Windows, and a Linux-only one is no use here.
fn is_cross_platform(platforms: &[String]) -> bool {
    if platforms.is_empty() {
        return true;
    }
    let has = |wanted: &str| platforms.iter().any(|entry| entry == wanted);
    has("macos") && has("windows")
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
        if path.file_name().and_then(|n| n.to_str()) == Some(META_FILE) {
            continue;
        }
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

fn link_into(root: &Path, slug: &str, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| format!("Could not create {}: {e}", root.display()))?;
    let link = root.join(slug);
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

/// Links a skill into the main agent's folder and every enabled agent folder.
fn link_skill(slug: &str, dir: &Path) -> Result<(), String> {
    link_into(&links_root(), slug, dir)?;
    for (_, _, root) in enabled_targets() {
        // A mirror is helpful but not worth failing the main link over.
        let _ = link_into(&root, slug, dir);
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

fn unlink_from(root: &Path, slug: &str) {
    let link = root.join(slug);
    if let Ok(meta) = fs::symlink_metadata(&link) {
        if meta.file_type().is_symlink() || meta.is_file() {
            let _ = fs::remove_file(&link);
        } else if meta.is_dir() {
            // Windows enables by copying, so disabling must remove the folder.
            let _ = fs::remove_dir_all(&link);
        }
    }
}

fn unlink_skill(slug: &str) {
    unlink_from(&links_root(), slug);
    for (_, _, root) in enabled_targets() {
        unlink_from(&root, slug);
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
    let mut candidates: Vec<(PathBuf, String, String)> = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    build_skills(candidates)
}

#[cfg(test)]
fn scan_roots(store: PathBuf, hermes: PathBuf) -> Vec<Skill> {
    let mut candidates: Vec<(PathBuf, String, String)> = Vec::new();
    collect_skills(store, "store", &mut candidates);
    collect_skills(hermes, "hermes", &mut candidates);
    build_skills(candidates)
}

fn build_skills(candidates: Vec<(PathBuf, String, String)>) -> Vec<Skill> {
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
        let record = if source == "store" { read_meta(&dir) } else { None };
        let modified = record
            .as_ref()
            .map(|meta| !meta.hash.is_empty() && hash_skill(&dir) != meta.hash)
            .unwrap_or(false);
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
            version: record.as_ref().and_then(|meta| meta.version.clone()),
            origin: record
                .as_ref()
                .map(|meta| meta.origin.clone())
                .unwrap_or_else(|| if source == "hermes" { "hermes".into() } else { "unknown".into() }),
            installed_at: record.as_ref().map(|meta| meta.installed_at).unwrap_or(0),
            modified,
            scripts: count_scripts(&dir, 3),
            profile: record.as_ref().and_then(|meta| meta.profile.clone()),
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
        let blurb = "Your own folder";
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
        .user_agent("Kilgharrah/0.1 (skills and MCP manager)")
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
        stars: 0,
        rank: 0,
        grade: String::new(),
        market: "clawhub".into(),
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
        stars: 0,
        rank: 0,
        grade: String::new(),
        market: "clawhub".into(),
        category: "development".into(),
        topics: Vec::new(),
        installable: false,
        command: Some(format!("npm install {name}")),
    })
}

fn get_text(url: &str) -> Result<String, String> {
    match http_agent().get(url).call() {
        Ok(response) => response
            .into_string()
            .map_err(|e| format!("agenticskills.io sent a response Kilgharrah could not read: {e}")),
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            let message: String = body.chars().take(180).collect();
            Err(format!("agenticskills.io returned {code}. {message}"))
        }
        Err(error) => Err(format!("Could not reach agenticskills.io: {error}")),
    }
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn parse_count(text: &str) -> u64 {
    let trimmed = text.trim();
    let factor = match trimmed.chars().last() {
        Some('K') | Some('k') => 1_000f64,
        Some('M') | Some('m') => 1_000_000f64,
        _ => 1f64,
    };
    let digits: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    (digits.parse::<f64>().unwrap_or(0.0) * factor).round() as u64
}

/// agenticskills.io is server rendered and its API is private, so Kilgharrah
/// reads the published cards. The site lists each skill once, already ranked.
fn parse_rank_map(page: &str) -> HashMap<String, u32> {
    let Ok(ranks_re) = Regex::new(r#""position":(\d+),"url":"https://agenticskills\.io/skills/([^"]+)""#)
    else {
        return HashMap::new();
    };
    ranks_re
        .captures_iter(page)
        .filter_map(|caps| {
            Some((
                caps.get(2)?.as_str().to_string(),
                caps.get(1)?.as_str().parse().ok()?,
            ))
        })
        .collect()
}

fn parse_agentic(page: &str, ranks: &HashMap<String, u32>) -> Vec<CatalogItem> {
    let name_re = Regex::new(r#"<h3 class="[^"]*">([^<]+)</h3>"#).ok();
    let author_re = Regex::new(r#"by<!-- -->\s*<span[^>]*>([^<]+)</span>"#).ok();
    let grade_re = Regex::new(r#"([SABC])<!-- -->-rank"#).ok();
    let summary_re = Regex::new(r#"line-clamp-2 mb-4">([^<]*)</p>"#).ok();
    let category_re = Regex::new(
        r#"<span class="text-\[10px\] font-mono uppercase tracking-\[0\.08em\][^"]*">([^<]+)</span>"#,
    )
    .ok();
    let stars_re = Regex::new(
        r#"text-\[11px\] text-\[var\(--mute-2\)\]"><svg[\s\S]*?</svg>([^<]+)</span>"#,
    )
    .ok();

    let mut items = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for chunk in page
        .split("<a class=\"group block h-full\" href=\"/skills/")
        .skip(1)
    {
        let Some(slug_end) = chunk.find('"') else {
            continue;
        };
        let slug = chunk[..slug_end].trim().to_string();
        if slug.is_empty() || !safe_segment(&slug) || seen.iter().any(|value| value == &slug) {
            continue;
        }
        seen.push(slug.clone());
        let Some(name_re) = name_re.as_ref() else {
            break;
        };
        let Some(name) = name_re.captures(chunk).and_then(|caps| caps.get(1)) else {
            continue;
        };
        let name = decode_entities(name.as_str());
        if name.is_empty() {
            continue;
        }
        let author = author_re
            .as_ref()
            .and_then(|re| re.captures(chunk))
            .and_then(|caps| caps.get(1).map(|m| decode_entities(m.as_str())))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "agenticskills".into());
        let grade = grade_re
            .as_ref()
            .and_then(|re| re.captures(chunk))
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_default();
        let summary = summary_re
            .as_ref()
            .and_then(|re| re.captures(chunk))
            .and_then(|caps| caps.get(1).map(|m| decode_entities(m.as_str())))
            .unwrap_or_default();
        let site_category = category_re
            .as_ref()
            .and_then(|re| re.captures(chunk))
            .and_then(|caps| caps.get(1).map(|m| decode_entities(m.as_str())))
            .unwrap_or_default();
        let stars = stars_re
            .as_ref()
            .and_then(|re| re.captures(chunk))
            .and_then(|caps| caps.get(1).map(|m| parse_count(m.as_str())))
            .unwrap_or(0);
        let rank = ranks.get(&slug).copied().unwrap_or(0);
        let category = bucket_for(&site_category.to_lowercase(), &name, &summary);
        items.push(CatalogItem {
            id: format!("agentic:{slug}"),
            kind: "skill".into(),
            owner: author,
            slug,
            name,
            summary,
            version: String::new(),
            downloads: 0,
            stars,
            rank,
            grade,
            market: "agentic".into(),
            category,
            topics: Vec::new(),
            installable: true,
            command: None,
        });
    }
    items
}

const AGENTIC_CATEGORIES: [&str; 16] = [
    "agents",
    "ai-ml",
    "backend",
    "data-science",
    "database",
    "design",
    "devops",
    "documents",
    "marketing",
    "mobile",
    "official",
    "productivity",
    "security",
    "seo",
    "testing",
    "web-development",
];

fn agentic_pages() -> Result<Vec<String>, String> {
    let mut urls = vec![format!("{AGENTIC_BASE}/skills")];
    for category in AGENTIC_CATEGORIES {
        urls.push(format!("{AGENTIC_BASE}/category/{category}"));
    }
    let mut pages = Vec::new();
    for group in urls.chunks(6) {
        let fetched: Vec<Result<String, String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = group
                .iter()
                .map(|url| scope.spawn(move || get_text(url)))
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err("Could not read agenticskills.io.".into()))
                })
                .collect()
        });
        for result in fetched {
            if let Ok(page) = result {
                pages.push(page);
            }
        }
    }
    if pages.is_empty() {
        return Err("Could not reach agenticskills.io.".into());
    }
    Ok(pages)
}

fn agentic_pool() -> Result<Vec<CatalogItem>, String> {
    static CACHE: OnceLock<Mutex<Option<(Instant, Vec<CatalogItem>)>>> = OnceLock::new();
    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock() {
        if let Some((at, items)) = guard.as_ref() {
            if at.elapsed() < AGENTIC_TTL {
                return Ok(items.clone());
            }
        }
    }
    match agentic_pages() {
        Ok(pages) => {
            let ranks = parse_rank_map(&pages[0]);
            let mut seen: Vec<String> = Vec::new();
            let mut items: Vec<CatalogItem> = Vec::new();
            for page in &pages {
                for item in parse_agentic(page, &ranks) {
                    if seen.iter().any(|slug| slug.eq_ignore_ascii_case(&item.slug)) {
                        continue;
                    }
                    seen.push(item.slug.clone());
                    items.push(item);
                }
            }
            if items.is_empty() {
                return Err("agenticskills.io returned no skills.".into());
            }
            if let Ok(mut guard) = lock.lock() {
                *guard = Some((Instant::now(), items.clone()));
            }
            Ok(items)
        }
        Err(error) => {
            if let Ok(guard) = lock.lock() {
                if let Some((_, items)) = guard.as_ref() {
                    return Ok(items.clone());
                }
            }
            Err(error)
        }
    }
}

struct AgenticSource {
    owner: String,
    repo: String,
    git_ref: String,
    path: String,
    raw: String,
}

/// Splits a GitHub repo or tree URL into owner, repo, and an optional branch
/// plus folder. A plain repo URL has no folder, so the caller has to find one.
fn github_parts(url: &str) -> Option<(String, String, Option<(String, String)>)> {
    let path = url.trim_start_matches("https://github.com/");
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0].to_string();
    let repo = parts[1].trim_end_matches(".git").to_string();
    let nested = if parts.len() >= 4 && parts[2] == "tree" {
        Some((parts[3].to_string(), parts[4..].join("/")))
    } else {
        None
    };
    Some((owner, repo, nested))
}

/// Reads the GitHub source for one AgenticSkills page. The page embeds the
/// skill's own `githubUrl` next to other related skills, so the lookup starts
/// at this skill's slug instead of taking the first link on the page.
fn agentic_source(slug: &str) -> Result<AgenticSource, String> {
    let html = get_text(&format!("{AGENTIC_BASE}/skills/{}", encode(slug)))?;
    let page = html.replace("\\\"", "\"");
    let own = Regex::new(&format!(
        r#"(?s)"slug":"{}".*?"githubUrl":"(https://github\.com/[^"]+)""#,
        regex::escape(slug)
    ))
    .map_err(|e| format!("Could not read the skill page: {e}"))?;
    if let Some(caps) = own.captures(&page) {
        let url = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if let Some((owner, repo, nested)) = github_parts(url) {
            let (git_ref, dir) = if let Some((git_ref, dir)) = nested {
                (git_ref, dir)
            } else {
                let meta = get_json(&format!(
                    "https://api.github.com/repos/{}/{}",
                    encode(&owner),
                    encode(&repo)
                ))?;
                let branch = meta
                    .get("default_branch")
                    .and_then(|v| v.as_str())
                    .ok_or("GitHub lists no default branch for that skill.")?
                    .to_string();
                let tree = get_json(&format!(
                    "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
                    encode(&owner),
                    encode(&repo),
                    encode(&branch)
                ))?;
                if tree.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
                    return Err("That GitHub repo is too large for Kilgharrah to scan.".into());
                }
                let skills: Vec<String> = tree
                    .get("tree")
                    .and_then(|v| v.as_array())
                    .map(|rows| {
                        rows.iter()
                            .filter_map(|row| row.get("path").and_then(|v| v.as_str()))
                            .filter(|path| *path == "SKILL.md" || path.ends_with("/SKILL.md"))
                            .map(|path| path.trim_end_matches("SKILL.md").trim_end_matches('/').to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                match skills.len() {
                    0 => return Err("That GitHub repo has no SKILL.md.".into()),
                    1 => (branch, skills[0].clone()),
                    _ => {
                        return Err(format!(
                            "That GitHub repo holds {} skills, so Kilgharrah cannot pick one. Open the repo and install the skill you want.",
                            skills.len()
                        ))
                    }
                }
            };
            let raw = if dir.is_empty() {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/SKILL.md")
            } else {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{dir}/SKILL.md")
            };
            return Ok(AgenticSource { owner, repo, git_ref, path: dir, raw });
        }
    }
    let re = Regex::new(
        r#"https://raw\.githubusercontent\.com/([^/"\s]+)/([^/"\s]+)/([^/"\s]+)/([^"\s<]+?)/SKILL\.md"#,
    )
    .map_err(|e| format!("Could not read the install command: {e}"))?;
    let caps = re
        .captures(&page)
        .ok_or("agenticskills.io lists no GitHub source for that skill.")?;
    let owner = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    let repo = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
    let git_ref = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
    let path = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();
    if owner.is_empty() || repo.is_empty() || git_ref.is_empty() || path.is_empty() {
        return Err("agenticskills.io lists an incomplete GitHub source for that skill.".into());
    }
    let raw = format!("https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{path}/SKILL.md");
    Ok(AgenticSource { owner, repo, git_ref, path, raw })
}

fn merge_items(items: Vec<CatalogItem>, limit: usize) -> Vec<CatalogItem> {
    let mut out: Vec<CatalogItem> = Vec::new();
    for item in items {
        if let Some(existing) = out
            .iter_mut()
            .find(|entry| entry.slug.eq_ignore_ascii_case(&item.slug))
        {
            existing.downloads = existing.downloads.max(item.downloads);
            existing.stars = existing.stars.max(item.stars);
            if existing.grade.is_empty() {
                existing.grade = item.grade.clone();
            }
            if existing.rank == 0 {
                existing.rank = item.rank;
            }
            if item.market == "agentic" && existing.market != "agentic" {
                existing.market = "both".into();
            }
            continue;
        }
        out.push(item);
    }
    out.sort_by(|a, b| b.downloads.cmp(&a.downloads).then(a.rank.cmp(&b.rank)));
    out.truncate(limit);
    out
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

/// Skills installed before records existed get a baseline record with an
/// unknown source, so edits from now on are still visible.
fn backfill_records() {
    let mut candidates = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    for (dir, _, _) in candidates {
        if read_meta(&dir).is_some() {
            continue;
        }
        let slug = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let manifest = dir.join("SKILL.md");
        let name = fs::read_to_string(&manifest)
            .ok()
            .and_then(|text| parse_frontmatter(&text).0)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| titleize(&slug));
        let _ = write_meta(
            &dir,
            &InstalledMeta {
                name,
                origin: "unknown".into(),
                upstream: None,
                version: None,
                installed_at: modified_secs(&manifest),
                hash: hash_skill(&dir),
                profile: None,
            },
        );
    }
}

fn setup_view() -> SetupView {
    let settings = read_settings();
    SetupView {
        onboarded: settings.onboarded,
        scope: settings.scope,
        project_root: settings.project_root,
        agent_name: agent_name(&settings.agent).to_string(),
        agent: settings.agent,
        profile: settings.profile,
    }
}

fn build_state() -> Result<AppState, String> {
    ensure_store()?;
    backfill_records();
    let skills = scan_skills();
    Ok(AppState {
        store: store_root().display().to_string(),
        skills_root: links_root().display().to_string(),
        hermes_root: hermes_root().display().to_string(),
        mcp_path: mcp_path().display().to_string(),
        categories: list_categories(&skills),
        skills,
        servers: read_servers(),
        agent_targets: agent_target_state(),
        setup: setup_view(),
        profiles: list_profiles(),
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
    write_meta(
        &dir,
        &InstalledMeta {
            name: display.clone(),
            origin: "local".into(),
            upstream: None,
            version: None,
            installed_at: now_secs(),
            hash: hash_skill(&dir),
            profile: None,
        },
    )?;
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
    mirror_mcp()?;
    build_state()
}

#[tauri::command]
fn delete_server(name: String) -> Result<AppState, String> {
    let mut root = read_mcp();
    if let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove(&name);
    }
    write_mcp(&root)?;
    mirror_mcp()?;
    build_state()
}

// ---------------------------------------------------------------- MCP mirror

/// The config file each agent reads. The main agent reads the canonical file,
/// so it has no separate target.
fn agent_mcp_path(id: &str, settings: &Settings) -> Option<PathBuf> {
    let project = settings.scope == "project";
    match id {
        "claude" if project => Some(base_dir(settings).join(".mcp.json")),
        "claude" => Some(home_dir().join(".claude.json")),
        "gemini" if project => Some(base_dir(settings).join(".gemini").join("settings.json")),
        "gemini" => Some(home_dir().join(".gemini").join("settings.json")),
        "codex" if project => Some(base_dir(settings).join(".codex").join("config.toml")),
        "codex" => Some(home_dir().join(".codex").join("config.toml")),
        _ => None,
    }
}

fn mirror_targets(settings: &Settings) -> Vec<(String, PathBuf)> {
    let mut ids = vec![settings.agent.clone()];
    for id in &settings.agent_targets {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for id in ids {
        if let Some(path) = agent_mcp_path(&id, settings) {
            if out.iter().any(|(_, known)| known == &path) {
                continue;
            }
            out.push((id, path));
        }
    }
    out
}

fn mirror_json(
    path: &Path,
    servers: &serde_json::Map<String, Value>,
    previous: &[String],
) -> Result<Vec<String>, String> {
    let mut root = fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| json!({}));
    let Some(object) = root.as_object_mut() else {
        return Err(format!("{} does not hold a JSON object.", path.display()));
    };
    let entry = object.entry("mcpServers").or_insert_with(|| json!({}));
    let Some(map) = entry.as_object_mut() else {
        return Err(format!(
            "mcpServers in {} is not an object.",
            path.display()
        ));
    };
    for name in previous {
        map.remove(name);
    }
    for (name, value) in servers {
        map.insert(name.clone(), value.clone());
    }
    let mut written: Vec<String> = servers.keys().cloned().collect();
    written.sort();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Could not encode {}: {e}", path.display()))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(written)
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn toml_key(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        value.to_string()
    } else {
        toml_string(value)
    }
}

const TOML_BEGIN: &str = "# kilgharrah:begin";
const TOML_END: &str = "# kilgharrah:end";

fn mirror_toml(
    path: &Path,
    servers: &serde_json::Map<String, Value>,
) -> Result<Vec<String>, String> {
    let mut block = String::from(TOML_BEGIN);
    block.push_str("\n# Written by Kilgharrah. Change servers in the app, not here.\n");
    let mut names: Vec<&String> = servers.keys().collect();
    names.sort();
    for name in &names {
        let Some(entry) = servers.get(*name) else {
            continue;
        };
        let command = entry
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        block.push_str(&format!(
            "\n[mcp_servers.{}]\ncommand = {}\n",
            toml_key(name),
            toml_string(command)
        ));
        if let Some(args) = entry.get("args").and_then(|value| value.as_array()) {
            let values: Vec<String> = args
                .iter()
                .filter_map(|value| value.as_str())
                .map(toml_string)
                .collect();
            block.push_str(&format!("args = [{}]\n", values.join(", ")));
        }
        if let Some(env) = entry.get("env").and_then(|value| value.as_object()) {
            if !env.is_empty() {
                block.push_str(&format!("[mcp_servers.{}.env]\n", toml_key(name)));
                for (key, value) in env {
                    block.push_str(&format!(
                        "{} = {}\n",
                        toml_key(key),
                        toml_string(value.as_str().unwrap_or_default())
                    ));
                }
            }
        }
    }
    block.push_str(TOML_END);

    let existing = fs::read_to_string(path).unwrap_or_default();
    let next = match (existing.find(TOML_BEGIN), existing.find(TOML_END)) {
        (Some(start), Some(end)) if end > start => {
            let after = end + TOML_END.len();
            format!("{}{}{}", &existing[..start], block, &existing[after..])
        }
        _ => {
            let mut text = existing.trim_end().to_string();
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&block);
            text.push('\n');
            text
        }
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::write(path, next).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(names.into_iter().cloned().collect())
}

fn strip_toml_block(path: &Path) -> Result<(), String> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if let (Some(start), Some(end)) = (existing.find(TOML_BEGIN), existing.find(TOML_END)) {
        if end > start {
            let after = end + TOML_END.len();
            let next = format!("{}{}", existing[..start].trim_end(), &existing[after..]);
            fs::write(path, format!("{}\n", next.trim()))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Copy the canonical servers into every agent config Kilgharrah serves. The
/// canonical file stays the one place a server is defined.
fn mirror_mcp() -> Result<(), String> {
    let canonical = read_mcp();
    let Some(servers) = canonical
        .get("mcpServers")
        .and_then(|value| value.as_object())
        .cloned()
    else {
        return Ok(());
    };
    let mut settings = read_settings();
    let previous_state = settings.mirror_state.clone();
    let mut next_state = BTreeMap::new();
    for (id, path) in mirror_targets(&settings) {
        if path == mcp_path() {
            continue;
        }
        let key = path.display().to_string();
        let previous = previous_state.get(&key).cloned().unwrap_or_default();
        let written = if id == "codex" {
            mirror_toml(&path, &servers)?
        } else {
            mirror_json(&path, &servers, &previous)?
        };
        next_state.insert(key, written);
    }
    // An agent that was turned off keeps its config file, but not our entries.
    let empty = serde_json::Map::new();
    for (key, names) in &previous_state {
        if next_state.contains_key(key) {
            continue;
        }
        let path = PathBuf::from(key);
        if !path.exists() {
            continue;
        }
        if key.ends_with(".toml") {
            strip_toml_block(&path)?;
        } else {
            mirror_json(&path, &empty, names)?;
        }
    }
    if next_state != previous_state {
        settings.mirror_state = next_state;
        write_settings(&settings)?;
    }
    Ok(())
}

// The marketplace lives behind the network, so the command runs off the webview
// thread. A synchronous command would hold the window still for the whole call.
#[tauri::command]
async fn search_market(query: String, limit: Option<u32>) -> Result<Vec<CatalogItem>, String> {
    tauri::async_runtime::spawn_blocking(move || search_market_now(query, limit))
        .await
        .map_err(|error| format!("The marketplace search stopped: {error}"))?
}

fn search_market_now(query: String, limit: Option<u32>) -> Result<Vec<CatalogItem>, String> {
    let limit = limit.unwrap_or(24).clamp(1, 300) as usize;
    let query = query.trim().to_string();
    let agentic = agentic_pool().unwrap_or_default();
    let mut items: Vec<CatalogItem> = Vec::new();

    if query.is_empty() {
        items = market_pool()?;
        items.extend(agentic);
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
        let needle = query.to_lowercase();
        items.extend(agentic.into_iter().filter(|item| {
            [&item.name, &item.summary, &item.owner, &item.category]
                .iter()
                .any(|field| field.to_lowercase().contains(&needle))
        }));
    }

    Ok(merge_items(items, limit))
}

fn market_pool() -> Result<Vec<CatalogItem>, String> {
    static CACHE: OnceLock<Mutex<Option<(Instant, Vec<CatalogItem>)>>> = OnceLock::new();
    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock() {
        if let Some((at, items)) = guard.as_ref() {
            if at.elapsed() < MARKET_TTL {
                return Ok(items.clone());
            }
        }
    }
    match skill_pool(200) {
        Ok(items) => {
            let pool = merge_items(items, 200);
            if let Ok(mut guard) = lock.lock() {
                *guard = Some((Instant::now(), pool.clone()));
            }
            Ok(pool)
        }
        Err(error) => {
            if let Ok(guard) = lock.lock() {
                if let Some((_, items)) = guard.as_ref() {
                    return Ok(items.clone());
                }
            }
            Err(error)
        }
    }
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
fn preview_market(
    slug: String,
    owner: Option<String>,
    market: Option<String>,
) -> Result<String, String> {
    if !safe_segment(&slug) {
        return Err("That skill name is not valid.".into());
    }
    if market.as_deref() == Some("agentic") {
        let source = agentic_source(&slug)?;
        let bytes = get_bytes(&source.raw)?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        return Ok(text.chars().take(PREVIEW_CHARS).collect());
    }
    let mut url = format!("{BASE}/api/v1/skills/{}/file?path=SKILL.md", encode(&slug));
    if let Some(owner) = owner.filter(|value| safe_segment(value)) {
        url.push_str(&format!("&owner={}", encode(&owner)));
    }
    let bytes = get_bytes(&url)?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(text.chars().take(PREVIEW_CHARS).collect())
}

#[derive(Clone, Serialize)]
struct InstallProgress {
    id: String,
    phase: String,
    percent: u8,
}

/// Installs run on their own thread so the window stays usable. Progress and
/// the finished state come back as events keyed by the returned job id.
#[tauri::command]
fn install_market(
    app: tauri::AppHandle,
    slug: String,
    owner: Option<String>,
    category: Option<String>,
    market: Option<String>,
) -> Result<String, String> {
    if !safe_segment(&slug) {
        return Err("That skill name is not valid.".into());
    }
    let id = format!("install:{slug}");
    let task_id = id.clone();
    std::thread::spawn(move || {
        let progress = |percent: u8, phase: &str| {
            let _ = app.emit(
                "install-progress",
                InstallProgress {
                    id: task_id.clone(),
                    phase: phase.to_string(),
                    percent,
                },
            );
        };
        match install_market_task(&slug, owner, category, market, &progress) {
            Ok(state) => {
                let _ = app.emit("install-finished", json!({ "id": task_id, "state": state }));
            }
            Err(error) => {
                let _ = app.emit("install-failed", json!({ "id": task_id, "error": error }));
            }
        }
    });
    Ok(id)
}

/// Downloads one published ClawHub version into `dir` and returns the version.
/// The caller owns the folder and removes it if this fails.
fn download_clawhub(
    slug: &str,
    owner: &str,
    dir: &Path,
    progress: &dyn Fn(u8, &str),
) -> Result<String, String> {
    let owner_query = if owner.is_empty() {
        String::new()
    } else {
        format!("?owner={}", encode(owner))
    };
    let owner_suffix = if owner.is_empty() {
        String::new()
    } else {
        format!("&owner={}", encode(owner))
    };
    let meta = get_json(&format!("{BASE}/api/v1/skills/{}{owner_query}", encode(slug)))?;
    let version = meta
        .get("latestVersion")
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("ClawHub has no published version for that skill.")?
        .to_string();
    let detail = get_json(&format!(
        "{BASE}/api/v1/skills/{}/versions/{}{owner_query}",
        encode(slug),
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
    let file_count = files.len().max(1) as u8;
    let mut total: u64 = 0;
    for (index, file) in files.iter().enumerate() {
        let Some(relative) = file.get("path").and_then(|v| v.as_str()).and_then(safe_relative) else {
            return Err("ClawHub listed a file path Kilgharrah will not write.".into());
        };
        let done = (index as u32 + 1).min(file_count as u32);
        let percent = 14 + (done * 68 / file_count as u32).min(68) as u8;
        progress(percent, "Fetching skill files");
        let url = format!(
            "{BASE}/api/v1/skills/{}/file?path={}&version={}{owner_suffix}",
            encode(slug),
            encode(&relative.to_string_lossy()),
            encode(&version)
        );
        let bytes = get_bytes(&url)?;
        total += bytes.len() as u64;
        if total > MAX_BYTES {
            return Err("That skill is larger than the 12 MB install limit.".into());
        }
        let target = dir.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create a folder: {e}"))?;
        }
        fs::write(&target, bytes).map_err(|e| format!("Could not write a skill file: {e}"))?;
    }
    Ok(version)
}

fn install_market_task(
    slug: &str,
    owner: Option<String>,
    category: Option<String>,
    market: Option<String>,
    progress: &dyn Fn(u8, &str),
) -> Result<AppState, String> {
    ensure_store()?;
    if market.as_deref() == Some("agentic") {
        return install_agentic(slug, category, progress);
    }
    progress(6, "Reading the skill");
    let owner = owner.unwrap_or_default().trim().to_string();
    let chosen = {
        let meta = get_json(&format!(
            "{BASE}/api/v1/skills/{}{}",
            encode(slug),
            if owner.is_empty() { String::new() } else { format!("?owner={}", encode(&owner)) }
        ))?;
        match category {
            Some(id) if safe_segment(&id) && skills_store().join(&id).is_dir() => id,
            _ => suggest_category(&meta),
        }
    };
    let dir = skills_store().join(&chosen).join(slug);
    if dir.exists() {
        return Err(format!("{slug} is already installed in {chosen}."));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create the skill folder: {e}"))?;
    let version = match download_clawhub(slug, &owner, &dir, progress) {
        Ok(version) => version,
        Err(error) => {
            let _ = fs::remove_dir_all(&dir);
            return Err(error);
        }
    };
    if let Err(error) = write_meta(
        &dir,
        &InstalledMeta {
            name: slug.to_string(),
            origin: "clawhub".into(),
            upstream: if owner.is_empty() { None } else { Some(owner.clone()) },
            version: Some(version),
            installed_at: now_secs(),
            hash: hash_skill(&dir),
            profile: None,
        },
    ) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    progress(92, "Linking the skill");
    if let Err(error) = link_skill(slug, &dir) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    let state = build_state()?;
    progress(100, "Installed");
    Ok(state)
}

fn collect_github(
    source: &AgenticSource,
    path: &str,
    base: &str,
    dir: &Path,
    total: &mut u64,
    files: &mut usize,
    progress: &dyn Fn(u8, &str),
) -> Result<(), String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        encode(&source.owner),
        encode(&source.repo),
        encode(path),
        encode(&source.git_ref)
    );
    let value = get_json(&url)?;
    let rows = value
        .as_array()
        .ok_or("GitHub returned no contents for that skill folder.")?;
    for row in rows {
        let kind = row.get("type").and_then(|v| v.as_str()).unwrap_or_default();
        let child = row
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if kind == "dir" {
            collect_github(source, &child, base, dir, total, files, progress)?;
            continue;
        }
        if kind != "file" {
            continue;
        }
        let Some(relative) = child.strip_prefix(&format!("{base}/")) else {
            continue;
        };
        let Some(relative) = safe_relative(relative) else {
            return Err("GitHub listed a file path Kilgharrah will not write.".into());
        };
        *files += 1;
        if *files > MAX_FILES {
            return Err("That skill has too many files for Kilgharrah to install.".into());
        }
        let percent = (22 + *files as u8 * 6).min(88);
        progress(percent, &format!("Fetching files ({})", *files));
        let download = row
            .get("download_url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| {
                format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    source.owner, source.repo, source.git_ref, child
                )
            });
        let bytes = get_bytes(&download)?;
        *total += bytes.len() as u64;
        if *total > MAX_BYTES {
            return Err("That skill is larger than the 12 MB install limit.".into());
        }
        let target = dir.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create a folder: {e}"))?;
        }
        fs::write(&target, bytes).map_err(|e| format!("Could not write a skill file: {e}"))?;
    }
    Ok(())
}

fn install_agentic(
    slug: &str,
    category: Option<String>,
    progress: &dyn Fn(u8, &str),
) -> Result<AppState, String> {
    progress(8, "Finding the GitHub source");
    let source = agentic_source(slug)?;
    let chosen = match category {
        Some(id) if safe_segment(&id) && skills_store().join(&id).is_dir() => id,
        _ => "other".to_string(),
    };
    let dir = skills_store().join(&chosen).join(slug);
    if dir.exists() {
        return Err(format!("{slug} is already installed in {chosen}."));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create the skill folder: {e}"))?;
    let base = source.path.clone();
    let mut total = 0u64;
    let mut files = 0usize;
    if let Err(error) = collect_github(&source, &base, &base, &dir, &mut total, &mut files, progress) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    if files == 0 {
        let _ = fs::remove_dir_all(&dir);
        return Err("That skill has no files to install.".into());
    }
    if !dir.join("SKILL.md").is_file() {
        let _ = fs::remove_dir_all(&dir);
        return Err("That skill has no SKILL.md at its GitHub source.".into());
    }
    if let Err(error) = write_meta(
        &dir,
        &InstalledMeta {
            name: slug.to_string(),
            origin: "agentic".into(),
            upstream: Some(format!("{}/{}/{}", source.owner, source.repo, source.path)),
            version: None,
            installed_at: now_secs(),
            hash: hash_skill(&dir),
            profile: None,
        },
    ) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    progress(94, "Linking the skill");
    if let Err(error) = link_skill(slug, &dir) {
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }
    let state = build_state()?;
    progress(100, "Installed");
    Ok(state)
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

fn enabled_store_dirs() -> Vec<(String, PathBuf)> {
    let mut candidates = Vec::new();
    collect_skills(skills_store(), "store", &mut candidates);
    candidates
        .into_iter()
        .filter_map(|(dir, _, _)| {
            let slug = dir.file_name().and_then(|n| n.to_str())?.to_string();
            if is_enabled(&slug) {
                Some((slug, dir))
            } else {
                None
            }
        })
        .collect()
}

#[tauri::command]
fn set_agent_targets(targets: Vec<String>) -> Result<AppState, String> {
    ensure_store()?;
    let before = read_settings();
    let mut chosen: Vec<String> = Vec::new();
    for (id, _, _) in AGENT_TARGETS {
        if targets.iter().any(|value| value == id) && !chosen.iter().any(|value| value == id) {
            chosen.push(id.to_string());
        }
    }
    let mut next = before.clone();
    next.agent_targets = chosen.clone();
    write_settings(&next)?;

    let enabled = enabled_store_dirs();
    for (id, _, relative) in AGENT_TARGETS {
        let root = target_root(relative);
        if !chosen.iter().any(|value| value == id) {
            if before.agent_targets.iter().any(|value| value == id) {
                for (slug, _) in &enabled {
                    unlink_from(&root, slug);
                }
            }
            continue;
        }
        for (slug, dir) in &enabled {
            let _ = link_into(&root, slug, dir);
        }
    }
    mirror_mcp()?;
    build_state()
}

#[tauri::command(async)]
fn check_updates() -> Result<Vec<SkillUpdate>, String> {
    let mut updates = Vec::new();
    for skill in scan_skills().into_iter().filter(|skill| skill.source == "store") {
        let Some(meta) = read_meta(Path::new(&skill.path)) else {
            updates.push(SkillUpdate {
                category: skill.category,
                slug: skill.slug,
                name: skill.name,
                origin: "unknown".into(),
                current: None,
                latest: None,
                status: "unknown".into(),
                detail: "Installed before Kilgharrah recorded installs, so there is no source to check."
                    .into(),
            });
            continue;
        };
        if meta.origin != "clawhub" {
            updates.push(SkillUpdate {
                category: skill.category,
                slug: skill.slug,
                name: skill.name,
                origin: meta.origin,
                current: meta.version,
                latest: None,
                status: "unknown".into(),
                detail: "Kilgharrah has no update feed for this source.".into(),
            });
            continue;
        }
        let owner = meta.upstream.clone().unwrap_or_default();
        let url = format!(
            "{BASE}/api/v1/skills/{}{}",
            encode(&skill.slug),
            if owner.is_empty() { String::new() } else { format!("?owner={}", encode(&owner)) }
        );
        match get_json(&url) {
            Ok(value) => {
                let latest = value
                    .get("latestVersion")
                    .and_then(|v| v.get("version"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let available = match (&meta.version, &latest) {
                    (Some(current), Some(next)) => current != next,
                    _ => false,
                };
                updates.push(SkillUpdate {
                    category: skill.category,
                    slug: skill.slug,
                    name: skill.name,
                    origin: meta.origin,
                    current: meta.version,
                    latest: latest.clone(),
                    status: if available { "available" } else { "current" }.into(),
                    detail: if available {
                        format!("Version {} is published.", latest.unwrap_or_default())
                    } else {
                        "You have the latest published version.".into()
                    },
                });
            }
            Err(error) => updates.push(SkillUpdate {
                category: skill.category,
                slug: skill.slug,
                name: skill.name,
                origin: meta.origin,
                current: meta.version,
                latest: None,
                status: "error".into(),
                detail: error,
            }),
        }
    }
    Ok(updates)
}

fn update_skill_inner(category: &str, slug: &str) -> Result<AppState, String> {
    ensure_store()?;
    if !safe_segment(&category) || !safe_segment(&slug) {
        return Err("That skill path is not valid.".into());
    }
    let dir = skills_store().join(category).join(slug);
    if !dir.is_dir() {
        return Err("That skill is not in the library.".into());
    }
    let meta = read_meta(&dir).ok_or("Kilgharrah has no install record for that skill.")?;
    if meta.origin != "clawhub" {
        return Err("Kilgharrah can only update skills that came from ClawHub.".into());
    }
    let temp_root = store_root().join(".tmp");
    fs::create_dir_all(&temp_root)
        .map_err(|e| format!("Could not create the update folder: {e}"))?;
    let stamp = format!("{}-{}", slug, std::process::id());
    let temp = temp_root.join(&stamp);
    let backup = temp_root.join(format!("{stamp}-old"));
    let _ = fs::remove_dir_all(&temp);
    let _ = fs::remove_dir_all(&backup);
    fs::create_dir_all(&temp).map_err(|e| format!("Could not create the update folder: {e}"))?;
    let owner = meta.upstream.clone().unwrap_or_default();
    let version = match download_clawhub(slug, &owner, &temp, &|_, _| {}) {
        Ok(version) => version,
        Err(error) => {
            let _ = fs::remove_dir_all(&temp);
            return Err(error);
        }
    };
    if let Err(error) = write_meta(
        &temp,
        &InstalledMeta {
            name: meta.name.clone(),
            origin: "clawhub".into(),
            upstream: meta.upstream.clone(),
            version: Some(version),
            installed_at: now_secs(),
            hash: hash_skill(&temp),
            profile: meta.profile.clone(),
        },
    ) {
        let _ = fs::remove_dir_all(&temp);
        return Err(error);
    }
    let was_enabled = is_enabled(slug);
    unlink_skill(slug);
    fs::rename(&dir, &backup).map_err(|e| format!("Could not move the old copy aside: {e}"))?;
    if let Err(error) = fs::rename(&temp, &dir) {
        let _ = fs::rename(&backup, &dir);
        return Err(format!("Could not finish the update: {error}"));
    }
    let _ = fs::remove_dir_all(&backup);
    let _ = fs::remove_dir(&temp_root);
    if was_enabled {
        link_skill(slug, &dir)?;
    }
    build_state()
}

#[tauri::command(async)]
fn update_skill(category: String, slug: String) -> Result<AppState, String> {
    update_skill_inner(&category, &slug)
}

/// Update every skill that has a newer published version. One failure does not
/// stop the run.
#[tauri::command(async)]
fn update_all_skills() -> Result<UpdateReport, String> {
    let updates = check_updates()?;
    let mut updated = 0usize;
    let mut failed = Vec::new();
    for entry in updates.iter().filter(|entry| entry.status == "available") {
        match update_skill_inner(&entry.category, &entry.slug) {
            Ok(_) => updated += 1,
            Err(error) => failed.push(format!("{}: {}", entry.name, error)),
        }
    }
    Ok(UpdateReport {
        state: build_state()?,
        updated,
        failed,
    })
}

/// Record where a skill came from, so an unknown source can be fixed by hand.
#[tauri::command]
fn set_skill_source(category: String, slug: String, owner: String) -> Result<AppState, String> {
    ensure_store()?;
    if !safe_segment(&category) || !safe_segment(&slug) {
        return Err("That skill path is not valid.".into());
    }
    let dir = skills_store().join(&category).join(&slug);
    if !dir.is_dir() {
        return Err("That skill is not in the library.".into());
    }
    let mut meta = read_meta(&dir).ok_or("Kilgharrah has no install record for that skill.")?;
    let owner = owner.trim().trim_start_matches('@').to_string();
    if owner.is_empty() {
        meta.origin = "local".into();
        meta.upstream = None;
    } else if safe_segment(&owner) {
        meta.origin = "clawhub".into();
        meta.upstream = Some(owner);
    } else {
        return Err("Use a ClawHub owner name, letters, numbers and hyphens only.".into());
    }
    write_meta(&dir, &meta)?;
    build_state()
}

#[tauri::command]
fn diagnostics() -> Result<Diagnostics, String> {
    ensure_store()?;
    let mut checks = Vec::new();

    let probe = store_root().join(".write-probe");
    let store_ok = fs::write(&probe, b"ok").is_ok() && fs::remove_file(&probe).is_ok();
    checks.push(Check {
        id: "store".into(),
        label: "Library is writable".into(),
        status: if store_ok { "ok" } else { "error" }.into(),
        detail: store_root().display().to_string(),
    });

    let link_probe = links_root().join(".write-probe");
    let links_ok = fs::create_dir_all(links_root()).is_ok()
        && fs::write(&link_probe, b"ok").is_ok()
        && fs::remove_file(&link_probe).is_ok();
    checks.push(Check {
        id: "links".into(),
        label: "CLI skills folder is writable".into(),
        status: if links_ok { "ok" } else { "error" }.into(),
        detail: links_root().display().to_string(),
    });

    let mcp = mcp_path();
    if !mcp.exists() {
        checks.push(Check {
            id: "mcp".into(),
            label: "MCP config".into(),
            status: "ok".into(),
            detail: "No mcp.json yet. Kilgharrah creates one when you add a server.".into(),
        });
    } else {
        let parsed = fs::read_to_string(&mcp)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        checks.push(Check {
            id: "mcp".into(),
            label: "MCP config parses".into(),
            status: if parsed.is_some() { "ok" } else { "error" }.into(),
            detail: mcp.display().to_string(),
        });
    }

    let mut roots = vec![links_root()];
    for (_, _, root) in enabled_targets() {
        roots.push(root);
    }
    let mut links = 0usize;
    let mut broken: Vec<String> = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            links += 1;
            let is_link = fs::symlink_metadata(&path)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);
            if is_link && fs::metadata(&path).is_err() {
                broken.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    checks.push(Check {
        id: "broken-links".into(),
        label: "Enabled links resolve".into(),
        status: if broken.is_empty() { "ok" } else { "warn" }.into(),
        detail: if broken.is_empty() {
            format!("{links} links, none broken")
        } else {
            format!("{} broken: {}", broken.len(), broken.iter().take(3).cloned().collect::<Vec<_>>().join(", "))
        },
    });

    let skills = scan_skills();
    let unmanaged: Vec<&Skill> = skills
        .iter()
        .filter(|skill| skill.source == "store" && skill.origin == "unknown")
        .collect();
    checks.push(Check {
        id: "records".into(),
        label: "Library skills know where they came from".into(),
        status: if unmanaged.is_empty() { "ok" } else { "warn" }.into(),
        detail: if unmanaged.is_empty() {
            format!("{} skills recorded", skills.iter().filter(|s| s.source == "store").count())
        } else {
            format!(
                "{} recorded without a known source: {}",
                unmanaged.len(),
                unmanaged.iter().take(3).map(|s| s.slug.clone()).collect::<Vec<_>>().join(", ")
            )
        },
    });

    let modified: Vec<&Skill> = skills.iter().filter(|skill| skill.modified).collect();
    checks.push(Check {
        id: "modified".into(),
        label: "Skill files match their records".into(),
        status: if modified.is_empty() { "ok" } else { "warn" }.into(),
        detail: if modified.is_empty() {
            "No installed skill has changed since it was recorded.".into()
        } else {
            format!(
                "{} edited since install: {}",
                modified.len(),
                modified.iter().take(3).map(|s| s.slug.clone()).collect::<Vec<_>>().join(", ")
            )
        },
    });

    let targets = agent_target_state();
    let on = targets.iter().filter(|t| t.enabled).count();
    checks.push(Check {
        id: "agents".into(),
        label: "Agent mirrors".into(),
        status: "ok".into(),
        detail: if on == 0 {
            "No agent mirrors are on. The main agent still gets every enabled skill.".into()
        } else {
            format!("{on} of 3 mirrors on: {}", targets.iter().filter(|t| t.enabled).map(|t| t.name.clone()).collect::<Vec<_>>().join(", "))
        },
    });

    let ok = !checks.iter().any(|check| check.status == "error");
    Ok(Diagnostics { ok, checks, checked_at: now_secs() })
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
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
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
            set_agent_targets,
            check_updates,
            update_skill,
            update_all_skills,
            set_skill_source,
            complete_setup,
            read_dragon,
            save_dragon,
            export_profile,
            import_profile,
            apply_profile,
            delete_profile,
            scan_installed_skills,
            import_installed_skills,
            diagnostics,
            open_url,
            reveal_store
        ])
        .run(tauri::generate_context!())
        .expect("Kilgharrah failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The commands read `KILGHARRAH_HOME` and `KILGHARRAH_HERMES`, so every
    /// test that sets them holds this lock. Without it a parallel test can see
    /// another test's temporary folders.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn seed_skill(dir: &Path, text: &str) {
        fs::create_dir_all(dir).expect("skill folder is created");
        fs::write(dir.join("SKILL.md"), text).expect("skill file is written");
    }

    #[test]
    fn emrys_leaves_out_single_platform_skills() {
        let macos = vec!["macos".to_string()];
        let no_windows = vec!["linux".to_string(), "macos".to_string()];
        let cross = vec![
            "linux".to_string(),
            "macos".to_string(),
            "windows".to_string(),
        ];
        assert!(!is_cross_platform(&macos));
        assert!(!is_cross_platform(&no_windows));
        assert!(is_cross_platform(&cross));
        let undeclared: Vec<String> = Vec::new();
        assert!(is_cross_platform(&undeclared));
    }

    #[test]
    fn frontmatter_platforms_reads_the_inline_list() {
        let text = "---\nname: imessage\ndescription: x\nplatforms: [macos]\n---\n# iMessage\n";
        assert_eq!(frontmatter_platforms(text), vec!["macos".to_string()]);
        assert_eq!(frontmatter_platforms("# no frontmatter"), Vec::<String>::new());
    }

    /// Drives the same functions the Scan and Emrys commands call, against a
    /// temporary home and Hermes install, so the portability filter, the scan
    /// and the import are checked end to end without touching this machine.
    #[test]
    fn emrys_scan_and_import_round_trip() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let root = std::env::temp_dir().join(format!("kilgharrah-emrys-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        let hermes = root.join("hermes");
        seed_skill(
            &hermes.join("development").join("alpha"),
            "---\nname: Alpha\ndescription: Runs anywhere\n---\n\nBody\n",
        );
        seed_skill(
            &hermes.join("apple").join("beta"),
            "---\nname: Beta\ndescription: Apple only\nplatforms: [macos]\n---\n\nBody\n",
        );
        std::env::set_var("KILGHARRAH_HOME", &home);
        std::env::set_var("KILGHARRAH_HERMES", &hermes);

        let (kept, skipped) = portable_hermes_candidates();
        assert_eq!(kept.len(), 1, "only the cross-platform skill is kept");
        assert_eq!(skipped, vec!["Beta".to_string()]);

        let installed = apply_emrys().expect("Emrys applies");
        assert_eq!(installed, 1);
        let store = home.join("kilgharrah").join("skills");
        assert!(
            store.join("development").join("alpha").join("SKILL.md").is_file(),
            "an existing category folder is kept"
        );
        assert!(
            !store.join("apple").exists(),
            "the macOS-only skill never reaches the library"
        );

        let found = scan_installed_skills().expect("the scan runs");
        let alpha = found
            .iter()
            .find(|skill| skill.slug == "alpha")
            .expect("alpha is offered");
        assert_eq!(alpha.source, "Hermes");
        assert!(alpha.portable, "alpha declares no platforms");
        assert!(alpha.in_library, "alpha was just applied from Emrys");
        let beta = found
            .iter()
            .find(|skill| skill.slug == "beta")
            .expect("beta is offered");
        assert!(!beta.portable);
        assert!(!beta.in_library);
        assert_eq!(beta.platforms, vec!["macos".to_string()]);

        // A folder the scan offers can be imported; anything else is refused.
        seed_skill(
            &hermes.join("productivity").join("gamma"),
            "---\nname: Gamma\n---\n\nBody\n",
        );
        let outside = root.join("elsewhere").join("evil");
        seed_skill(&outside, "---\nname: Evil\n---\n\nBody\n");
        import_installed_skills(vec![
            hermes.join("productivity").join("gamma").display().to_string(),
            outside.display().to_string(),
        ])
        .expect("gamma imports");
        assert!(store.join("productivity").join("gamma").join("SKILL.md").is_file());
        assert!(
            !store.join("evil").exists(),
            "a path outside the scanned roots is refused"
        );

        std::env::remove_var("KILGHARRAH_HOME");
        std::env::remove_var("KILGHARRAH_HERMES");
        let _ = fs::remove_dir_all(&root);
    }

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
    fn dragon_frames_keep_their_grid() {
        let frame = (0..23)
            .map(|_| " ".repeat(70))
            .collect::<Vec<_>>()
            .join("\n");
        let text = build_dragon_text(&[frame.clone(), frame]).expect("valid frames save");
        assert_eq!(text.matches("\n~~~~~~~~\n").count(), 1);
        assert!(build_dragon_text(&["one line".into()]).is_err());
        let wide = (0..23)
            .map(|_| "x".repeat(71))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(build_dragon_text(&[wide]).is_err());
    }

    #[test]
    fn dragon_frames_keep_blank_bottom_rows() {
        // Several frames rest the dragon near the top, so the last rows are empty.
        let mut rows = vec![" ".repeat(20); 19];
        for _ in 0..4 {
            rows.push(String::new());
        }
        let frame = rows.join("\n");
        let text = build_dragon_text(&[frame]).expect("blank bottom rows save");
        assert_eq!(text.split('\n').count(), 23);
    }

    #[test]
    fn dragon_frames_round_trip_through_a_file() {
        let dir = std::env::temp_dir().join(format!("kilgharrah-dragon-{}", std::process::id()));
        let path = dir.join("dragon.txt");
        let mut resting = vec![" ".repeat(20); 19];
        for _ in 0..4 {
            resting.push(String::new());
        }
        let frames = vec![(0..23).map(|_| "+".repeat(70)).collect::<Vec<_>>().join("\n"), resting.join("\n")];

        save_dragon_at(&path, &frames).expect("frames save to a file");
        let saved = read_dragon_at(&path).expect("frames read back").expect("file exists");
        assert!(saved.ends_with('\n'));

        // Drop only the newline the file ends with, the way the frontend does.
        // Blank rows at the end of the last frame must survive.
        let body = saved.strip_suffix('\n').expect("file ends with a newline");
        let reread: Vec<String> = body
            .split("\n~~~~~~~~\n")
            .map(|frame| frame.to_string())
            .collect();
        assert_eq!(reread, frames);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hermes_yaml_servers_are_read() {
        let text = r#"
onboarding:
  run: false
mcp_servers:
  rea:
    command: /opt/homebrew/bin/rea.mjs
    args:
      - mcp
    enabled: true
    env:
      PATH: /usr/bin:/bin
  paused:
    command: npx
    enabled: false
"#;
        let servers = servers_from_hermes_yaml(text).expect("the YAML parses");
        assert_eq!(servers.len(), 1, "a disabled server is skipped");
        assert_eq!(servers[0].name, "rea");
        assert_eq!(servers[0].command, "/opt/homebrew/bin/rea.mjs");
        assert_eq!(servers[0].args, vec!["mcp".to_string()]);
        assert_eq!(
            servers[0].env.get("PATH").map(String::as_str),
            Some("/usr/bin:/bin")
        );
    }

    #[test]
    fn hermes_json_servers_are_read() {
        let text = r#"{"mcpServers":{"mine":{"command":"npx","args":["-y","pkg"]}}}"#;
        let servers = servers_from_hermes_json(text);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "mine");
        assert_eq!(servers[0].args, vec!["-y".to_string(), "pkg".to_string()]);
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
    fn agentic_counts_follow_suffixes() {
        assert_eq!(parse_count("445.8K"), 445_800);
        assert_eq!(parse_count("1.2M"), 1_200_000);
        assert_eq!(parse_count("87"), 87);
        assert_eq!(parse_count(""), 0);
    }

    #[test]
    fn agentic_cards_are_parsed() {
        let page = r#"
        <script type="application/ld+json">[{"position":4,"url":"https://agenticskills.io/skills/humanizer","name":"Humanizer"}]</script>
        <div class="h-full"><a class="group block h-full" href="/skills/humanizer"><div class="relative h-full p-5">
        <h3 class="text-sm font-semibold">Humanizer &amp; editor</h3>
        <p class="text-xs">by<!-- --> <span class="text-[var(--mute-1)]">obra</span></p>
        <span class="text-[10px] px-2">S<!-- -->-rank</span>
        <p class="text-xs leading-relaxed line-clamp-2 mb-4">Strip the tells that make writing sound generated.</p>
        <div class="flex items-center gap-2 mb-3"><span class="text-[10px] font-mono uppercase tracking-[0.08em] px-2">Writing</span><span class="text-[10px] font-mono uppercase tracking-[0.08em] px-2">Open Source</span></div>
        <span class="flex items-center gap-1 text-[11px] text-[var(--mute-2)]"><svg><path d="M1 1"></path></svg>12.5K</span>
        </div></a></div>"#;
        let items = parse_agentic(page, &parse_rank_map(page));
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.slug, "humanizer");
        assert_eq!(item.name, "Humanizer & editor");
        assert_eq!(item.owner, "obra");
        assert_eq!(item.grade, "S");
        assert_eq!(item.rank, 4);
        assert_eq!(item.stars, 12_500);
        assert_eq!(item.category, "writing");
        assert_eq!(item.market, "agentic");
        assert!(item.installable);
    }

    #[test]
    fn short_words_need_boundaries() {
        assert!(!mentions("email detail", "ai"));
        assert!(mentions("ai tools", "ai"));
        assert!(mentions("codebase work", "code"));
    }

    #[test]
    fn state_backfills_records_and_lists_agent_targets() {
        let root = std::env::temp_dir().join(format!("kilgharrah-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let skill = root.join("kilgharrah/skills/design/legacy");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: Legacy\n---\n\nBody\n").unwrap();
        let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        std::env::set_var("KILGHARRAH_HOME", &root);

        let state = build_state().unwrap();
        let legacy = state.skills.iter().find(|s| s.slug == "legacy").unwrap();
        assert_eq!(legacy.origin, "unknown");
        assert!(legacy.installed_at > 0, "the record gets a baseline time");
        assert!(read_meta(&skill).is_some(), "a baseline record is written");
        assert_eq!(state.agent_targets.len(), 3);
        assert!(state.agent_targets.iter().all(|target| !target.enabled));

        std::env::remove_var("KILGHARRAH_HOME");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn install_records_round_trip_and_hash_ignores_meta() {
        let root = std::env::temp_dir().join(format!("kilgharrah-meta-{}", std::process::id()));
        let dir = root.join("demo");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: Demo\n---\n\nBody\n").unwrap();

        let first = hash_skill(&dir);
        write_meta(
            &dir,
            &InstalledMeta {
                name: "Demo".into(),
                origin: "clawhub".into(),
                upstream: Some("owner".into()),
                version: Some("1.2.0".into()),
                installed_at: 10,
                hash: first.clone(),
                profile: None,
            },
        )
        .unwrap();

        assert_eq!(hash_skill(&dir), first, "the record does not change the hash");
        assert_eq!(count_files(&dir, 3), 1, "the record is not counted as a skill file");
        let record = read_meta(&dir).expect("record reads back");
        assert_eq!(record.origin, "clawhub");
        assert_eq!(record.version.as_deref(), Some("1.2.0"));

        fs::write(dir.join("SKILL.md"), "---\nname: Demo\n---\n\nEdited\n").unwrap();
        assert_ne!(hash_skill(&dir), first, "editing a file changes the hash");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn disabling_removes_a_copied_skill_folder() {
        let root = std::env::temp_dir().join(format!("kilgharrah-unlink-{}", std::process::id()));
        let link = root.join("demo");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&link).unwrap();
        fs::write(link.join("SKILL.md"), "Body\n").unwrap();

        unlink_from(&root, "demo");
        assert!(!link.exists(), "a copied folder is removed on disable");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn profile_snapshot_keeps_files_and_servers() {
        let root = std::env::temp_dir().join(format!("kilgharrah-profile-{}", std::process::id()));
        let dir = root.join("creative/p5js");
        fs::create_dir_all(dir.join("references")).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: p5js\n---\n\nBody\n").unwrap();
        fs::write(dir.join("references/api.md"), "Reference\n").unwrap();

        let profile = snapshot_profile(
            "Hermes",
            vec![(dir, "creative".to_string(), "hermes".to_string())],
            vec![ProfileServer {
                name: "rea".into(),
                command: "/opt/rea".into(),
                args: vec!["mcp".into()],
                env: BTreeMap::new(),
            }],
        )
        .expect("the snapshot builds");

        assert_eq!(profile.skills.len(), 1);
        assert_eq!(profile.skills[0].name, "p5js");
        assert_eq!(profile.skills[0].files.len(), 2, "SKILL.md and its reference travel");
        assert_eq!(profile.servers[0].name, "rea");
        let json = serde_json::to_string(&profile).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.skills[0].slug, "p5js");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn profile_export_path_keeps_json() {
        let dir = std::env::temp_dir().join(format!("kilgharrah-export-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let given = profile_destination("Hermes", Some(dir.join("hermes").display().to_string()));
        assert_eq!(given.extension().and_then(|value| value.to_str()), Some("json"));
        let kept = profile_destination("Hermes", Some(dir.join("hermes.json").display().to_string()));
        assert_eq!(kept.file_name().and_then(|value| value.to_str()), Some("hermes.json"));
        let _ = fs::remove_dir_all(&dir);
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

    #[test]
    fn github_urls_split_into_a_repo_and_optional_folder() {
        let plain = github_parts("https://github.com/morluto/rea").unwrap();
        assert_eq!(plain, ("morluto".into(), "rea".into(), None));

        let nested = github_parts("https://github.com/squirrelscan/skills/tree/main/skills/audit-website")
            .unwrap();
        assert_eq!(nested.0, "squirrelscan");
        assert_eq!(nested.1, "skills");
        assert_eq!(nested.2, Some(("main".into(), "skills/audit-website".into())));

        let root = github_parts("https://github.com/owner/repo/tree/main").unwrap();
        assert_eq!(root.2, Some(("main".into(), String::new())));

        assert!(github_parts("https://github.com/owner").is_none());
    }

    #[test]
    fn profiles_are_deleted_by_name() {
        let dir = std::env::temp_dir().join(format!("kilgharrah-profiles-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let profile = Profile {
            name: "Test Profile".into(),
            created_at: 1,
            skills: vec![],
            servers: vec![],
        };
        let path = dir.join("test-profile.json");
        fs::write(&path, serde_json::to_string_pretty(&profile).unwrap()).unwrap();

        let removed = remove_profile_file(&dir, "test profile").expect("a slug match deletes");
        assert_eq!(removed, path);
        assert!(!path.exists());

        assert!(
            remove_profile_file(&dir, "Test Profile").is_err(),
            "a deleted profile is no longer found"
        );
        assert!(
            delete_profile("Hermes".into()).is_err(),
            "the built-in profile is protected"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mirror_json_keeps_foreign_servers_and_drops_stale_ones() {
        let root = std::env::temp_dir().join(format!("kilgharrah-mirror-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("config.json");
        fs::write(
            &path,
            r#"{"theme":"dark","mcpServers":{"mine":{"command":"own"},"old":{"command":"gone"}}}"#,
        )
        .unwrap();

        let mut servers = serde_json::Map::new();
        servers.insert("next".into(), json!({ "command": "npx", "args": ["-y", "pkg"] }));
        let written = mirror_json(&path, &servers, &["old".to_string()]).unwrap();

        let value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["theme"], "dark", "unrelated keys stay");
        assert!(value["mcpServers"]["mine"].is_object(), "user servers stay");
        assert!(value["mcpServers"]["old"].is_null(), "our stale copy is removed");
        assert_eq!(value["mcpServers"]["next"]["command"], "npx");
        assert_eq!(written, vec!["next".to_string()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn mirror_toml_replaces_its_block_once() {
        let root = std::env::temp_dir().join(format!("kilgharrah-toml-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("config.toml");
        fs::write(&path, "model = \"gpt\"\n").unwrap();

        let mut servers = serde_json::Map::new();
        servers.insert(
            "pg".into(),
            json!({ "command": "npx", "args": ["-y", "pg"], "env": { "DSN": "a b" } }),
        );
        mirror_toml(&path, &servers).unwrap();
        mirror_toml(&path, &servers).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("model = \"gpt\""), "existing config stays first");
        assert_eq!(text.matches(TOML_BEGIN).count(), 1, "the block is not duplicated");
        assert!(text.contains("[mcp_servers.pg]"));
        assert!(text.contains("args = [\"-y\", \"pg\"]"));
        assert!(text.contains("DSN = \"a b\""));

        let _ = fs::remove_dir_all(&root);
    }
}
