//! Just enough of `~/.ssh/config` to connect the way `ssh <alias>` would:
//! `Host` blocks (patterns, `!negation`), `Include`, and the keys HostName,
//! User, Port, IdentityFile and ProxyJump. As in OpenSSH, the first value
//! found for a key wins; IdentityFile accumulates.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Nested `Include`s deeper than this are ignored (loops).
const MAX_INCLUDE_DEPTH: usize = 8;

#[derive(Debug, Clone, Default)]
struct Block {
    /// None for the options before the first `Host` line (they apply to all).
    patterns: Option<Vec<String>>,
    options: Vec<(String, String)>,
}

/// A parsed config file (with its includes).
#[derive(Debug, Clone, Default)]
pub struct SshConfig {
    blocks: Vec<Block>,
}

/// What the config says about one host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConfig {
    /// The name `ssh` was given (the alias).
    pub alias: String,
    pub host_name: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    /// Paths with `~` expanded.
    pub identity_files: Vec<PathBuf>,
    /// Hops, outermost first; `ProxyJump none` gives an empty list.
    pub proxy_jump: Vec<String>,
}

impl HostConfig {
    /// HostName, or the alias itself.
    pub fn host(&self) -> &str {
        self.host_name.as_deref().unwrap_or(&self.alias)
    }
}

pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// `~/.ssh/config`.
pub fn default_config_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".ssh").join("config"))
}

/// `~/.ssh/known_hosts`.
pub fn default_known_hosts_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".ssh").join("known_hosts"))
}

/// `~/` → home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    match (
        path.strip_prefix("~/").or(path.strip_prefix("~\\")),
        home_dir(),
    ) {
        (Some(rest), Some(home)) => home.join(rest),
        _ if path == "~" => home_dir().unwrap_or_else(|| PathBuf::from(path)),
        _ => PathBuf::from(path),
    }
}

impl SshConfig {
    /// `~/.ssh/config`; empty when there is none.
    pub fn load_default() -> std::io::Result<Self> {
        match default_config_path() {
            Some(path) if path.exists() => Self::load(&path),
            _ => Ok(Self::default()),
        }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut config = Self::default();
        config.parse_into(&text, &base, 0, None);
        Ok(config)
    }

    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        let base = home_dir().map(|h| h.join(".ssh")).unwrap_or_default();
        config.parse_into(text, &base, 0, None);
        config
    }

    /// `patterns`: the Host block the text sits in (an Include inside a
    /// Host block); None at the top of the main file, where lines apply to all.
    fn parse_into(&mut self, text: &str, base: &Path, depth: usize, patterns: Option<Vec<String>>) {
        self.blocks.push(Block {
            patterns: patterns.clone(),
            options: Vec::new(),
        });
        let mut skipping = false; // inside a Match block
        for line in text.lines() {
            let Some((key, value)) = split_line(line) else {
                continue;
            };
            match key.as_str() {
                "host" => {
                    skipping = false;
                    self.blocks.push(Block {
                        patterns: Some(value.split_whitespace().map(str::to_owned).collect()),
                        options: Vec::new(),
                    });
                }
                "match" => {
                    // TODO: Match blocks (exec, user, ...) aren't evaluated; their options are ignored.
                    skipping = true;
                }
                _ if skipping => {}
                "include" => {
                    if depth >= MAX_INCLUDE_DEPTH {
                        continue;
                    }
                    for pattern in value.split_whitespace() {
                        let pattern = expand_tilde(pattern);
                        let pattern = if pattern.is_absolute() {
                            pattern
                        } else {
                            base.join(pattern)
                        };
                        let Ok(paths) = glob::glob(&pattern.to_string_lossy()) else {
                            continue;
                        };
                        for path in paths.flatten() {
                            if let Ok(text) = std::fs::read_to_string(&path) {
                                // Leading lines of an included file belong to the block
                                // the Include sits in; continue that block afterwards.
                                let current = self.blocks.last().and_then(|b| b.patterns.clone());
                                self.parse_into(&text, base, depth + 1, current.clone());
                                self.blocks.push(Block {
                                    patterns: current,
                                    options: Vec::new(),
                                });
                            }
                        }
                    }
                }
                _ => {
                    if let Some(block) = self.blocks.last_mut() {
                        block.options.push((key, value));
                    }
                }
            }
        }
    }

    /// Settings for `alias`, as `ssh alias` would use them.
    pub fn resolve(&self, alias: &str) -> HostConfig {
        let mut host = HostConfig {
            alias: alias.to_owned(),
            ..HostConfig::default()
        };
        let mut jump_set = false;
        for block in &self.blocks {
            let applies = match &block.patterns {
                None => true,
                Some(patterns) => host_matches(patterns, alias),
            };
            if !applies {
                continue;
            }
            for (key, value) in &block.options {
                match key.as_str() {
                    "hostname" if host.host_name.is_none() => host.host_name = Some(value.clone()),
                    "user" if host.user.is_none() => host.user = Some(value.clone()),
                    "port" if host.port.is_none() => host.port = value.parse().ok(),
                    "identityfile" => host.identity_files.push(expand_tilde(value)),
                    "proxyjump" if !jump_set => {
                        jump_set = true;
                        if !value.eq_ignore_ascii_case("none") {
                            host.proxy_jump =
                                value.split(',').map(|s| s.trim().to_owned()).collect();
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(name) = &host.host_name {
            // %h in HostName stands for the alias.
            host.host_name = Some(name.replace("%h", alias));
        }
        host
    }

    /// Host aliases a user can pick: names in `Host` lines without wildcards
    /// or negation, in file order, each once.
    pub fn aliases(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for patterns in self.blocks.iter().filter_map(|b| b.patterns.as_ref()) {
            for p in patterns {
                let plain = !p.contains(['*', '?', '!']);
                if plain && !out.contains(p) {
                    out.push(p.clone());
                }
            }
        }
        out
    }
}

/// "Key value", "Key=value", "Key = value"; comments and blank lines give None.
fn split_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let split = line.find(|c: char| c.is_whitespace() || c == '=')?;
    let key = line[..split].to_ascii_lowercase();
    let value = line[split..]
        .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
        .trim();
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value);
    Some((key, value.to_owned()))
}

/// `Host` patterns: any positive match and no negated match.
fn host_matches(patterns: &[String], host: &str) -> bool {
    let mut matched = false;
    for p in patterns {
        if let Some(negated) = p.strip_prefix('!') {
            if wildcard(negated, host) {
                return false;
            }
        } else if wildcard(p, host) {
            matched = true;
        }
    }
    matched
}

/// `*` and `?` wildcards, case-insensitive like OpenSSH.
fn wildcard(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// `[user@]host[:port]` of a ProxyJump hop.
pub fn parse_hop(hop: &str) -> (Option<String>, String, Option<u16>) {
    let (user, rest) = match hop.rsplit_once('@') {
        Some((u, r)) => (Some(u.to_owned()), r),
        None => (None, hop),
    };
    if let Some(inner) = rest.strip_prefix('[') {
        // [ipv6]:port
        if let Some((host, after)) = inner.split_once(']') {
            let port = after.strip_prefix(':').and_then(|p| p.parse().ok());
            return (user, host.to_owned(), port);
        }
    }
    match rest.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (user, host.to_owned(), port.parse().ok()),
        _ => (user, rest.to_owned(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"
# global
User fallback

Host bastion
    HostName bastion.example.com
    User ops
    Port 2222
    IdentityFile ~/.ssh/bastion

Host db-* !db-test
    ProxyJump bastion
    IdentityFile ~/.ssh/db

Host db-prod
    HostName 10.0.0.5
    User postgres

Match exec "true"
    User ignored

Host *
    IdentityFile ~/.ssh/id_ed25519
    Port 22
"#;

    #[test]
    fn first_value_wins_and_identities_accumulate() {
        let c = SshConfig::parse(CONFIG);
        let h = c.resolve("db-prod");
        assert_eq!(h.host(), "10.0.0.5");
        assert_eq!(h.user.as_deref(), Some("fallback")); // the global User comes first
        assert_eq!(h.port, Some(22));
        assert_eq!(h.proxy_jump, ["bastion"]);
        assert_eq!(h.identity_files.len(), 2);
        assert!(h.identity_files[0].ends_with("db"));
    }

    #[test]
    fn negation_and_aliases() {
        let c = SshConfig::parse(CONFIG);
        assert!(c.resolve("db-test").proxy_jump.is_empty());
        assert_eq!(c.resolve("bastion").port, Some(2222));
        assert_eq!(c.aliases(), ["bastion", "db-prod"]);
    }

    #[test]
    fn wildcards() {
        assert!(wildcard("db-*", "DB-prod"));
        assert!(wildcard("h?st", "host"));
        assert!(!wildcard("db-*", "web"));
        assert!(wildcard("*", ""));
    }

    #[test]
    fn hops() {
        assert_eq!(
            parse_hop("a@b:2200"),
            (Some("a".into()), "b".into(), Some(2200))
        );
        assert_eq!(parse_hop("bastion"), (None, "bastion".into(), None));
        assert_eq!(parse_hop("[::1]:22"), (None, "::1".into(), Some(22)));
    }

    #[test]
    fn line_forms() {
        assert_eq!(split_line("Port=22"), Some(("port".into(), "22".into())));
        assert_eq!(
            split_line(r#"  IdentityFile "C:\keys\a b""#),
            Some(("identityfile".into(), r"C:\keys\a b".into()))
        );
        assert_eq!(split_line("# x"), None);
    }
}
