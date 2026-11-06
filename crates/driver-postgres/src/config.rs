use std::{collections::HashMap, path::PathBuf, time::Duration};

use driver_core::{
    connect::{Auth, ConnectConfig, Secret, SslMode, Target, Timeouts, TlsConfig},
    dbresults::DbResult,
    error::{DbError, ErrorKind},
};
use tokio_postgres::config::{Host, SslMode as PgSslMode, TargetSessionAttrs};

use crate::pgpass;

const DEFAULT_PORT: u16 = 5432;
/// When the app passes no `Timeouts::connect_secs`.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const APPLICATION_NAME: &str = "SwiftQL";

/// postgres://user:pass@host:port/db?sslmode=... → ConnectConfig.
///
/// One host gives `Target::Host`; several (`h1,h2:5433`) give `Target::Url`
/// with the password left out, so it isn't stored in the URL.
pub fn parse_url(url: &str) -> DbResult<ConnectConfig> {
    let parsed = ParsedUrl::parse(url)?;

    let user = parsed
        .user
        .clone()
        .ok_or_else(|| invalid_url("URL has no user name"))?;
    let auth = match (&parsed.ssl_cert, &parsed.ssl_key, &parsed.password) {
        (Some(cert), Some(key), _) => Auth::ClientCert {
            user,
            cert: cert.clone(),
            key: key.clone(),
        },
        (_, _, Some(password)) => Auth::Password {
            user,
            password: Secret::new(password.clone()),
        },
        _ => Auth::PasswordPrompt { user },
    };

    let target = match parsed.hosts.as_slice() {
        [(host, port)] => Target::Host {
            host: host.clone(),
            port: *port,
            database: parsed.database.clone(),
        },
        _ => Target::Url {
            url: without_password(url),
        },
    };

    Ok(ConnectConfig {
        target,
        auth,
        tls: TlsConfig {
            mode: parsed.ssl_mode.unwrap_or_default(),
            ca_cert: parsed.ssl_root_cert.clone(),
        },
        tunnel: None,
        options: HashMap::new(),
        timeouts: Timeouts::default(),
        host_keys: None,
    })
}

/// The URL with its password removed: "postgres://a:pw@h/db" → "postgres://a@h/db".
fn without_password(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(authority_end);
    match authority.rsplit_once('@') {
        Some((userinfo, hosts)) => {
            let user = userinfo.split_once(':').map_or(userinfo, |(u, _)| u);
            format!("{scheme}://{user}@{hosts}{tail}")
        }
        None => url.to_owned(),
    }
}

/// ConnectConfig → tokio_postgres::Config (target, auth, timeouts).
/// Unsupported target/auth → ErrorKind::Unsupported.
pub fn to_pg_config(config: &ConnectConfig) -> DbResult<tokio_postgres::Config> {
    let mut pg = tokio_postgres::Config::new();

    match &config.target {
        // The URL provides hosts, database and session attributes; credentials
        // come from config.auth and TLS from config.tls.
        Target::Url { url } => {
            let parsed = ParsedUrl::parse(url)?;
            for (host, port) in &parsed.hosts {
                pg.host(host);
                pg.port(port.unwrap_or(DEFAULT_PORT));
            }
            if let Some(db) = &parsed.database {
                pg.dbname(db);
            }
            if let Some(attrs) = parsed.target_session_attrs {
                pg.target_session_attrs(attrs);
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
            pg.user(user); // the certificate goes into the TLS connector (tls.rs)
        }
        Auth::PgPass { user } => {
            pg.user(user);
            let (host, port) = first_host(&pg);
            let lookup = pgpass::Lookup {
                host: &host,
                port,
                database: pg.get_dbname().unwrap_or(user),
                user,
            };
            match pgpass::find(&lookup) {
                Some(password) => {
                    pg.password(password);
                }
                None => {
                    let file = pgpass::file_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "the password file".into());
                    return Err(DbError::new(
                        ErrorKind::Auth,
                        format!("No password for {user}@{host}:{port} in PGPASSWORD or {file}"),
                    ));
                }
            }
        }
        Auth::None => {
            return Err(unsupported("PostgreSQL always needs a user name"));
        }
        Auth::Token { .. } => {
            // TODO: cloud IAM tokens (AWS RDS, Azure AD) need provider SDKs.
            return Err(unsupported("token authentication is not supported yet"));
        }
    }

    pg.ssl_mode(match config.tls.mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        // certificate checks for VerifyCa / VerifyFull happen in the TLS connector
        SslMode::Require | SslMode::VerifyCa | SslMode::VerifyFull => PgSslMode::Require,
    });

    let connect_timeout = config
        .timeouts
        .connect_secs
        .filter(|&s| s > 0)
        .map_or(DEFAULT_CONNECT_TIMEOUT, |s| {
            Duration::from_secs(u64::from(s))
        });
    pg.connect_timeout(connect_timeout);
    if let Some(secs) = config.timeouts.keepalive_secs.filter(|&s| s > 0) {
        pg.keepalives(true);
        pg.keepalives_idle(Duration::from_secs(u64::from(secs)));
    } else {
        pg.keepalives(false);
    }
    pg.application_name(APPLICATION_NAME); // shown in pg_stat_activity

    if config.tunnel.is_some()
        && (pg.get_hosts().len() != 1 || !matches!(pg.get_hosts()[0], Host::Tcp(_)))
    {
        return Err(unsupported(
            "an SSH tunnel needs exactly one host name (no sockets or host lists)",
        ));
    }

    // No driver options are supported yet.
    if let Some(key) = config.options.keys().next() {
        return Err(unsupported(format!("unknown option `{key}`")));
    }

    Ok(pg)
}

/// The first TCP host and its port, e.g. for pgpass and SSH tunnels;
/// "localhost" for a socket, as libpq uses for pgpass.
pub fn first_host(pg: &tokio_postgres::Config) -> (String, u16) {
    let host = match pg.get_hosts().first() {
        Some(Host::Tcp(h)) => h.clone(),
        _ => "localhost".into(),
    };
    let port = pg.get_ports().first().copied().unwrap_or(DEFAULT_PORT);
    (host, port)
}

// ---------- URL parsing ----------

#[derive(Debug, Default)]
struct ParsedUrl {
    user: Option<String>,
    password: Option<String>,
    /// One or more `host[:port]`.
    hosts: Vec<(String, Option<u16>)>,
    database: Option<String>,
    ssl_mode: Option<SslMode>,
    ssl_root_cert: Option<PathBuf>,
    ssl_cert: Option<PathBuf>,
    ssl_key: Option<PathBuf>,
    target_session_attrs: Option<TargetSessionAttrs>,
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
        let (userinfo, hostports) = match authority.rsplit_once('@') {
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

        for hostport in hostports.split(',') {
            let (host, port) = split_host_port(hostport)?;
            if host.is_empty() {
                return Err(invalid_url("URL has no host"));
            }
            parsed.hosts.push((decode(host)?, port));
        }

        if !path.is_empty() {
            parsed.database = Some(decode(path)?);
        }

        for pair in query
            .into_iter()
            .flat_map(|q| q.split('&'))
            .filter(|p| !p.is_empty())
        {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let value = decode(value)?;
            match decode(key)?.as_str() {
                "sslmode" => parsed.ssl_mode = Some(parse_ssl_mode(&value)?),
                "sslrootcert" => parsed.ssl_root_cert = Some(PathBuf::from(value)),
                "sslcert" => parsed.ssl_cert = Some(PathBuf::from(value)),
                "sslkey" => parsed.ssl_key = Some(PathBuf::from(value)),
                "target_session_attrs" => {
                    parsed.target_session_attrs = Some(match value.as_str() {
                        "any" => TargetSessionAttrs::Any,
                        "read-write" => TargetSessionAttrs::ReadWrite,
                        "read-only" => TargetSessionAttrs::ReadOnly,
                        other => {
                            return Err(unsupported(format!(
                                "unsupported target_session_attrs `{other}`"
                            )));
                        }
                    })
                }
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
        assert_eq!(p.hosts, [("db.example.com".to_owned(), Some(6543))]);
        assert_eq!(p.database.as_deref(), Some("app"));
        assert_eq!(p.ssl_mode, Some(SslMode::Require));
    }

    #[test]
    fn ipv6_host() {
        let p = ParsedUrl::parse("postgresql://bob@[::1]:5432").unwrap();
        assert_eq!(p.hosts, [("::1".to_owned(), Some(5432))]);
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
        assert!(parse_url("postgres://a@h/db?application_name=x").is_err());
        assert!(parse_url("postgres://a@h:notaport/db").is_err());
    }

    #[test]
    fn several_hosts_keep_no_password() {
        let config =
            parse_url("postgres://a:secret@h1,h2:5433/db?target_session_attrs=read-write").unwrap();
        let Target::Url { url } = &config.target else {
            panic!("{:?}", config.target)
        };
        assert_eq!(
            url,
            "postgres://a@h1,h2:5433/db?target_session_attrs=read-write"
        );
        assert!(matches!(config.auth, Auth::Password { .. }));
        let pg = to_pg_config(&config).unwrap();
        assert_eq!(pg.get_hosts().len(), 2);
        assert_eq!(pg.get_ports(), [5432, 5433]);
    }

    #[test]
    fn ssl_parameters() {
        let config = parse_url(
            "postgres://a@h/db?sslmode=verify-full&sslrootcert=ca.pem&sslcert=c.pem&sslkey=k.pem",
        )
        .unwrap();
        assert_eq!(config.tls.mode, SslMode::VerifyFull);
        assert_eq!(config.tls.ca_cert, Some(PathBuf::from("ca.pem")));
        assert!(matches!(config.auth, Auth::ClientCert { .. }));
    }

    #[test]
    fn options_are_rejected() {
        let mut config = parse_url("postgres://a:b@h/db").unwrap();
        assert!(to_pg_config(&config).is_ok());
        config.options.insert("search_path".into(), "x".into());
        assert!(to_pg_config(&config).is_err());
    }
}
