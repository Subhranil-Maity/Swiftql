//! Passwords from `PGPASSWORD` or the password file, as libpq finds them:
//! `PGPASSFILE`, else `%APPDATA%\postgresql\pgpass.conf` on Windows, else
//! `~/.pgpass`. Lines are `host:port:database:user:password`; `*` matches
//! anything, `\:` and `\\` are escapes, the first matching line wins.

use std::path::PathBuf;

pub struct Lookup<'a> {
    pub host: &'a str,
    pub port: u16,
    pub database: &'a str,
    pub user: &'a str,
}

/// The password for `lookup`, if `PGPASSWORD` or the password file has one.
pub fn find(lookup: &Lookup<'_>) -> Option<String> {
    if let Ok(password) = std::env::var("PGPASSWORD")
        && !password.is_empty()
    {
        return Some(password);
    }
    let text = std::fs::read_to_string(file_path()?).ok()?;
    find_in(&text, lookup)
}

pub fn file_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PGPASSFILE") {
        return Some(PathBuf::from(path));
    }
    if cfg!(windows) {
        return dirs::config_dir().map(|d| d.join("postgresql").join("pgpass.conf"));
    }
    dirs::home_dir().map(|h| h.join(".pgpass"))
}

pub fn find_in(text: &str, lookup: &Lookup<'_>) -> Option<String> {
    let port = lookup.port.to_string();
    let wanted = [lookup.host, port.as_str(), lookup.database, lookup.user];
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .find_map(|line| {
            let fields = split(line);
            if fields.len() != 5 {
                return None;
            }
            let matches = fields[..4]
                .iter()
                .zip(wanted)
                .all(|(field, value)| field == "*" || field == value);
            matches.then(|| fields[4].clone())
        })
}

/// Fields separated by unescaped ':'.
fn split(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    fields.last_mut().unwrap().push(next);
                }
            }
            ':' if fields.len() < 5 => fields.push(String::new()),
            c => fields.last_mut().unwrap().push(c),
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r"
# comment
db.example.com:5432:app:alice:s3cr\:et
*:*:*:bob:any
localhost:6543:*:alice:other
";

    fn lookup<'a>(host: &'a str, port: u16, user: &'a str) -> Lookup<'a> {
        Lookup {
            host,
            port,
            database: "app",
            user,
        }
    }

    #[test]
    fn first_match_wins_with_escapes_and_wildcards() {
        assert_eq!(
            find_in(FILE, &lookup("db.example.com", 5432, "alice")).as_deref(),
            Some("s3cr:et")
        );
        assert_eq!(
            find_in(FILE, &lookup("anywhere", 1, "bob")).as_deref(),
            Some("any")
        );
        assert_eq!(
            find_in(FILE, &lookup("localhost", 6543, "alice")).as_deref(),
            Some("other")
        );
        assert_eq!(find_in(FILE, &lookup("localhost", 5432, "alice")), None);
    }
}
