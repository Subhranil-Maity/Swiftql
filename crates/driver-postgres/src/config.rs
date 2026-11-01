use std::collections::HashMap;

use driver_core::{
    connect::{Auth, ConnectConfig, Secret, SslMode, Target, TlsConfig},
    dbresults::DbResult,
    error::{DbError, ErrorKind},
};
use tokio_postgres::config::SslMode as PgSslMode;

const DEFAULT_PORT: u16 = 5432;

/// postgres://user:pass@host:port/db?sslmode=... → ConnectConfig
pub fn parse_url(url: &str) -> DbResult<ConnectConfig> {
    let parsed = ParsedUrl::parse(url)?;

    let user = parsed
        .user
        .ok_or_else(|| invalid_url("URL has no user name"))?;
    let auth = match parsed.password {
        Some(password) => Auth::Password {
            user,
            password: Secret::new(password),
        },
        None => Auth::PasswordPrompt { user },
    };

    Ok(ConnectConfig {
        target: Target::Host {
            host: parsed.host,
            port: parsed.port,
            database: parsed.database,
        },
        auth,
        tls: TlsConfig {
            mode: parsed.ssl_mode.unwrap_or_default(),
            ca_cert: None,
        },
        tunnel: None,
        options: HashMap::new(),
    })
}

/// ConnectConfig → tokio_postgres::Config (target, auth, options).
/// Unsupported target/auth → ErrorKind::Unsupported.
pub fn to_pg_config(config: &ConnectConfig) -> DbResult<tokio_postgres::Config> {
    let mut pg = tokio_postgres::Config::new();

    match &config.target {
        // The URL only provides host/port/db; credentials come from config.auth
        // and TLS from config.tls.
        Target::Url { url } => {
            let parsed = ParsedUrl::parse(url)?;
            pg.host(&parsed.host);
            pg.port(parsed.port.unwrap_or(DEFAULT_PORT));
            if let Some(db) = &parsed.database {
                pg.dbname(db);
            }
        }
        Target::Host {
            host,
            port,
            database,
        } => {
            pg.host(host);
            pg.port(port.unwrap_or(DEFAULT_PORT));
            if let Some(db) = database {
                pg.dbname(db);
            }
        }
        #[cfg(unix)]
        Target::Socket { path, database } => {
            pg.host_path(path);
            pg.port(DEFAULT_PORT); // part of the socket file name
            if let Some(db) = database {
                pg.dbname(db);
            }
        }
        #[cfg(not(unix))]
        Target::Socket { .. } => {
            return Err(unsupported(
                "Unix sockets are not available on this platform",
            ));
        }
        Target::File { .. } => {
            return Err(unsupported("PostgreSQL does not connect to a file"));
        }
    }

    match &config.auth {
        Auth::Password { user, password } => {
            pg.user(user);
            pg.password(password.expose());
        }
        Auth::PasswordPrompt { .. } => {
            // The app prompts and passes Auth::Password instead.
            return Err(DbError::new(
                ErrorKind::Internal,
                "password prompt was not answered before connecting",
            ));
        }
        Auth::ClientCert { user, .. } => {
            pg.user(user); // TODO: cert/key must be loaded into the TLS connector
        }
        Auth::None => {
            return Err(unsupported("PostgreSQL always needs a user name"));
        }
        Auth::PgPass { .. } => {
            return Err(unsupported(
                "TODO: ~/.pgpass / PGPASSWORD not supported yet",
            ));
        }
        Auth::Token { .. } => {
            return Err(unsupported("TODO: token authentication not supported yet"));
        }
    }

    pg.ssl_mode(match config.tls.mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        // certificate checks for VerifyCa / VerifyFull happen in the TLS connector
        SslMode::Require | SslMode::VerifyCa | SslMode::VerifyFull => PgSslMode::Require,
    });

    if config.tunnel.is_some() {
        return Err(unsupported("TODO: SSH tunnels not supported yet"));
    }

    // No driver options are supported yet.
    if let Some(key) = config.options.keys().next() {
        return Err(unsupported(format!("unknown option `{key}`")));
    }

    Ok(pg)
}

// ---------- URL parsing ----------

#[derive(Debug, Default, PartialEq)]
struct ParsedUrl {
    user: Option<String>,
    password: Option<String>,
    host: String,
    port: Option<u16>,
    database: Option<String>,
    ssl_mode: Option<SslMode>,
}

impl ParsedUrl {
    fn parse(url: &str) -> DbResult<Self> {
        let rest = url
            .strip_prefix("postgres://")
            .or_else(|| url.strip_prefix("postgresql://"))
            .ok_or_else(|| invalid_url("URL must start with postgres:// or postgresql://"))?;

        let (rest, query) = match rest.split_once('?') {
            Some((rest, query)) => (rest, Some(query)),
            None => (rest, None),
        };
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, path),
            None => (rest, ""),
        };
        // rsplit: the password may contain an unescaped '@'
        let (userinfo, hostport) = match authority.rsplit_once('@') {
            Some((userinfo, hostport)) => (Some(userinfo), hostport),
            None => (None, authority),
        };

        let mut parsed = ParsedUrl::default();

        if let Some(userinfo) = userinfo {
            let (user, password) = match userinfo.split_once(':') {
                Some((user, password)) => (user, Some(password)),
                None => (userinfo, None),
            };
            if !user.is_empty() {
                parsed.user = Some(decode(user)?);
            }
            parsed.password = password.map(decode).transpose()?;
        }

        if hostport.contains(',') {
            return Err(unsupported(
                "TODO: URLs with several hosts are not supported yet",
            ));
        }
        let (host, port) = split_host_port(hostport)?;
        if host.is_empty() {
            return Err(invalid_url("URL has no host"));
        }
        parsed.host = decode(host)?;
        parsed.port = port;

        if !path.is_empty() {
            parsed.database = Some(decode(path)?);
        }

        for pair in query
            .into_iter()
            .flat_map(|q| q.split('&'))
            .filter(|p| !p.is_empty())
        {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match decode(key)?.as_str() {
                "sslmode" => parsed.ssl_mode = Some(parse_ssl_mode(&decode(value)?)?),
                other => return Err(unsupported(format!("unknown URL parameter `{other}`"))),
            }
        }

        Ok(parsed)
    }
}

/// "host", "host:5432", "[::1]", "[::1]:5432"
fn split_host_port(hostport: &str) -> DbResult<(&str, Option<u16>)> {
    let (host, port) = if let Some(rest) = hostport.strip_prefix('[') {
        let (host, after) = rest
            .split_once(']')
            .ok_or_else(|| invalid_url("missing `]` in IPv6 host"))?;
        match after {
            "" => (host, None),
            _ => match after.strip_prefix(':') {
                Some(port) => (host, Some(port)),
                None => return Err(invalid_url("unexpected text after IPv6 host")),
            },
        }
    } else {
        match hostport.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (hostport, None),
        }
    };

    let port = match port {
        None | Some("") => None,
        Some(port) => Some(
            port.parse()
                .map_err(|_| invalid_url(format!("invalid port `{port}`")))?,
        ),
    };
    Ok((host, port))
}

fn parse_ssl_mode(value: &str) -> DbResult<SslMode> {
    Ok(match value {
        "disable" => SslMode::Disable,
        "prefer" => SslMode::Prefer,
        "require" => SslMode::Require,
        "verify-ca" => SslMode::VerifyCa,
        "verify-full" => SslMode::VerifyFull,
        other => return Err(unsupported(format!("unsupported sslmode `{other}`"))),
    })
}

/// Percent-decode one URL component.
fn decode(s: &str) -> DbResult<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let byte = s
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                .ok_or_else(|| invalid_url("invalid percent-encoding"))?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| invalid_url("URL is not valid UTF-8 after decoding"))
}

fn invalid_url(message: impl Into<String>) -> DbError {
    // no dedicated kind for bad input; Syntax is the closest
    DbError::new(ErrorKind::Syntax, message)
}

fn unsupported(message: impl Into<String>) -> DbError {
    DbError::new(ErrorKind::Unsupported, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_url() {
        let p = ParsedUrl::parse("postgres://alice:p%40ss@db.example.com:6543/app?sslmode=require")
            .unwrap();
        assert_eq!(p.user.as_deref(), Some("alice"));
        assert_eq!(p.password.as_deref(), Some("p@ss"));
        assert_eq!(p.host, "db.example.com");
        assert_eq!(p.port, Some(6543));
        assert_eq!(p.database.as_deref(), Some("app"));
        assert_eq!(p.ssl_mode, Some(SslMode::Require));
    }

    #[test]
    fn ipv6_host() {
        let p = ParsedUrl::parse("postgresql://bob@[::1]:5432").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, Some(5432));
        assert_eq!(p.database, None);
    }

    #[test]
    fn no_password_becomes_prompt() {
        let config = parse_url("postgres://alice@localhost/db").unwrap();
        assert!(matches!(config.auth, Auth::PasswordPrompt { ref user } if user == "alice"));
    }

    #[test]
    fn rejects_bad_urls() {
        assert!(parse_url("mysql://a@h/db").is_err());
        assert!(parse_url("postgres://localhost/db").is_err()); // no user
        assert!(parse_url("postgres://a@h1,h2/db").is_err());
        assert!(parse_url("postgres://a@h/db?application_name=x").is_err());
        assert!(parse_url("postgres://a@h:notaport/db").is_err());
    }

    #[test]
    fn options_are_rejected() {
        let mut config = parse_url("postgres://a:b@h/db").unwrap();
        assert!(to_pg_config(&config).is_ok());
        config.options.insert("search_path".into(), "x".into());
        assert!(to_pg_config(&config).is_err());
    }
}
