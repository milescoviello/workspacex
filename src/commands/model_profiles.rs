//! Named model endpoints that an agent instance can be pinned to.
//!
//! Stored as a newline-separated blob in the `model_profiles` setting, one
//! profile per line: a name followed by `key=value` pairs.
//!
//! ```text
//! local-qwen  base_url=http://127.0.0.1:8091 model=qwen3.8-27b max_context=212992
//! gpu-box     base_url=http://gpu-box.lan:8091 model=qwen3.8-27b auth_token_env=GPU_TOKEN
//! ```
//!
//! A profile is deliberately just an endpoint plus a model. wsx knows nothing
//! about ollama, llama.cpp, vLLM or any hosted provider as such — they are all
//! a base URL — so supporting a new one never requires a code change here.
//!
//! Credentials are referenced, never stored: `auth_token_env` names an
//! environment variable to read at spawn time. `state.db` is a plain
//! unencrypted file that travels with a home directory, so a literal token in a
//! setting would be a credential at rest that nothing knows how to rotate.

use crate::data::store::Store;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelProfile {
    pub name: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    /// Name of an environment variable holding the token — never the token.
    pub auth_token_env: Option<String>,
    pub max_context: Option<u64>,
}

/// Keys that would put a credential in the database. Rejected by name rather
/// than by sniffing values: a heuristic over the value would both miss tokens
/// that look ordinary and reject models whose names happen to look secret.
const CREDENTIAL_KEYS: &[&str] = &["auth_token", "token", "api_key", "apikey", "password"];

const KNOWN_KEYS: &[&str] = &["base_url", "model", "auth_token_env", "max_context"];

/// Parse one line. `Ok(None)` is a line with nothing in it (blank or comment);
/// `Err` carries a message naming what is wrong with it.
fn parse_line(raw: &str) -> std::result::Result<Option<ModelProfile>, String> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let mut fields = line.split_whitespace();
    // `split_whitespace` on a non-empty trimmed string always yields one item.
    let name = fields.next().unwrap_or_default().to_string();
    if name.contains('=') {
        return Err(format!(
            "profile line must start with a name, got a key=value pair: {name}"
        ));
    }
    // A leading dash makes the name unusable as a command argument — `wsx agent
    // profile -x` reads it as a flag — so a profile that could be created and
    // then not referred to is worse than one that is refused up front.
    if name.starts_with('-') {
        return Err(format!(
            "profile name '{name}' cannot start with '-'; it would be read as a \
             command-line flag"
        ));
    }
    let mut profile = ModelProfile {
        name,
        ..Default::default()
    };
    for field in fields {
        let Some((key, value)) = field.split_once('=') else {
            return Err(format!(
                "field '{field}' in profile '{}' is not key=value",
                profile.name
            ));
        };
        let (key, value) = (key.trim(), value.trim());
        if CREDENTIAL_KEYS.contains(&key) {
            return Err(format!(
                "profile '{}' sets '{key}' — store the NAME of an environment \
                 variable in auth_token_env instead, never the token itself",
                profile.name
            ));
        }
        if value.is_empty() {
            return Err(format!(
                "field '{key}' in profile '{}' has no value",
                profile.name
            ));
        }
        match key {
            "base_url" => {
                // Checked here because the alternative is discovering it much
                // later, as an opaque connection failure inside an agent that
                // has already spawned. A value without a scheme cannot be
                // anything but a mistake.
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(format!(
                        "base_url in profile '{}' must start with http:// or https://, got \
                         '{value}'",
                        profile.name
                    ));
                }
                // Trailing slashes are stripped so the same server written two
                // ways is one endpoint. Contention compares these strings, and
                // `http://h:1` vs `http://h:1/` would otherwise look like two
                // servers and under-report agents queuing on one.
                //
                // Deliberately not full URL normalisation — host case, default
                // ports, localhost vs 127.0.0.1 are a rabbit hole needing a URL
                // parser, and the trailing slash is the case people actually
                // hit.
                profile.base_url = Some(value.trim_end_matches('/').to_string());
            }
            "model" => profile.model = Some(value.to_string()),
            "auth_token_env" => profile.auth_token_env = Some(value.to_string()),
            "max_context" => {
                let n = value.parse::<u64>().map_err(|_| {
                    format!(
                        "max_context in profile '{}' must be a whole number of \
                         tokens, got '{value}'",
                        profile.name
                    )
                })?;
                // Zero would be forwarded to the agent as its context limit,
                // which is not a smaller window but an unusable one.
                if n == 0 {
                    return Err(format!(
                        "max_context in profile '{}' must be greater than zero",
                        profile.name
                    ));
                }
                profile.max_context = Some(n);
            }
            other => {
                return Err(format!(
                    "unknown field '{other}' in profile '{}' (known: {})",
                    profile.name,
                    KNOWN_KEYS.join(", ")
                ));
            }
        }
    }
    if profile.base_url.is_none() && profile.model.is_none() {
        return Err(format!(
            "profile '{}' sets neither base_url nor model, so it would do nothing",
            profile.name
        ));
    }
    Ok(Some(profile))
}

/// Parse the blob, skipping lines that are unusable.
///
/// Mirrors `shared_hosts::parse` in being tolerant at read time, so one bad
/// line can never make every other profile vanish from a running dashboard.
/// [`validate`] is the strict pass, run when the value is *set* — the one
/// moment a mistake can still be reported to the person making it.
pub fn parse(text: &str) -> Vec<ModelProfile> {
    text.lines()
        .filter_map(|raw| parse_line(raw).ok().flatten())
        .collect()
}

/// Strict parse for `wsx config set model_profiles`. Returns the text
/// unchanged when every line is usable, or the first problem with its line
/// number.
pub fn validate(text: &str) -> Result<String> {
    let mut seen: Vec<&str> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        match parse_line(raw) {
            Err(msg) => {
                return Err(Error::UserInput(format!(
                    "model_profiles line {}: {msg}",
                    i + 1
                )));
            }
            Ok(Some(_)) => {
                let name = raw.split_whitespace().next().unwrap_or_default();
                if seen.contains(&name) {
                    return Err(Error::UserInput(format!(
                        "model_profiles line {}: duplicate profile name '{name}'",
                        i + 1
                    )));
                }
                seen.push(name);
            }
            Ok(None) => {}
        }
    }
    Ok(text.to_string())
}

/// Every configured profile, alphabetized by name.
pub fn list(store: &Store) -> Result<Vec<ModelProfile>> {
    let raw = store.get_setting("model_profiles")?.unwrap_or_default();
    let mut out = parse(&raw);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// The profile called `name`, or `None`. Duplicate names cannot survive
/// [`validate`], so the first match is the only match.
pub fn lookup(store: &Store, name: &str) -> Result<Option<ModelProfile>> {
    let raw = store.get_setting("model_profiles")?.unwrap_or_default();
    Ok(parse(&raw).into_iter().find(|p| p.name == name))
}

/// Resolve what an agent instance should spawn with.
///
/// Precedence, highest first: the named profile, then the model/provider
/// captured on the row at creation time, then nothing — in which case the
/// per-agent builders fall back to `WSX_*_MODEL` in the spawning process's
/// environment, exactly as they did before any of this existed.
///
/// A profile outranks the captured model because it is the more deliberate
/// choice: `--profile` names an endpoint someone configured, while the captured
/// value is whatever happened to be exported in the shell that ran `workspace
/// create`. A profile that omits `model` still defers to the captured one, so
/// `base_url` alone is a usable profile.
///
/// A name that no longer resolves is not an error. Profiles are edited freely,
/// and a workspace pinned to one that has since been renamed or deleted must
/// still open — on ambient defaults, with a warning — rather than becoming
/// unopenable because of a config edit.
pub fn selection_for(
    store: &Store,
    instance: &crate::data::agents::AgentInstance,
) -> Result<crate::pty::ModelSelection> {
    let named = instance.model_profile.as_deref();
    let profile = match named {
        Some(name) => {
            let found = lookup(store, name)?;
            if found.is_none() {
                tracing::warn!(
                    profile = name,
                    "workspace is pinned to a model profile that no longer exists; \
                     falling back to the ambient environment"
                );
            }
            found
        }
        None => None,
    };
    Ok(match profile {
        Some(p) => crate::pty::ModelSelection {
            model: p.model.or_else(|| instance.model.clone()),
            provider: instance.provider.clone(),
            base_url: p.base_url,
            auth_token_env: p.auth_token_env,
            max_context: p.max_context,
        },
        None => crate::pty::ModelSelection {
            model: instance.model.clone(),
            provider: instance.provider.clone(),
            ..Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::data::store::{NewWorkspace, Store};
    use crate::pty::AgentKind;

    /// A store holding one workspace with a primary agent, plus `profiles` as
    /// the `model_profiles` setting.
    fn seed(profiles: &str) -> (Store, crate::data::store::AgentInstanceId) {
        let store = Store::open_in_memory().unwrap();
        let repo = store
            .add_repo(std::path::Path::new("/tmp/p"), "p", "wsx")
            .unwrap();
        let ws = store
            .insert_workspace(&NewWorkspace {
                repo_id: repo,
                name: "w",
                branch: "wsx/w",
                worktree_path: std::path::Path::new("/tmp/p/w"),
                yolo: false,
                agent: AgentKind::Claude,
                shared: false,
            })
            .unwrap();
        store.add_primary_agent(ws, AgentKind::Claude, 1).unwrap();
        if !profiles.is_empty() {
            store.set_setting("model_profiles", profiles).unwrap();
        }
        let inst = store.primary_instance_id(ws).unwrap().unwrap();
        (store, inst)
    }

    fn instance(
        store: &Store,
        id: crate::data::store::AgentInstanceId,
    ) -> crate::data::agents::AgentInstance {
        store.workspace_agents_by_id(id).unwrap().unwrap()
    }

    /// The precedence rule everything else depends on, in one place.
    #[test]
    fn a_profile_outranks_a_captured_model() {
        let (store, id) = seed("local base_url=http://127.0.0.1:8091 model=from-profile");

        // Captured value alone: no endpoint, just the model.
        store
            .set_instance_model(id, Some("from-row"), Some("prov"))
            .unwrap();
        let sel = selection_for(&store, &instance(&store, id)).unwrap();
        assert_eq!(sel.model.as_deref(), Some("from-row"));
        assert_eq!(sel.provider.as_deref(), Some("prov"));
        assert_eq!(sel.base_url, None);

        // Pinned to a profile: the profile wins, and brings the endpoint.
        store.set_instance_model_profile(id, Some("local")).unwrap();
        let sel = selection_for(&store, &instance(&store, id)).unwrap();
        assert_eq!(sel.model.as_deref(), Some("from-profile"));
        assert_eq!(sel.base_url.as_deref(), Some("http://127.0.0.1:8091"));
    }

    /// `base_url` alone is a legitimate profile — "same model, different
    /// machine" — so a profile without a model must not erase the captured one.
    #[test]
    fn a_profile_without_a_model_defers_to_the_captured_one() {
        let (store, id) = seed("endpoint-only base_url=http://gpu-box.lan:8091");
        store
            .set_instance_model(id, Some("from-row"), None)
            .unwrap();
        store
            .set_instance_model_profile(id, Some("endpoint-only"))
            .unwrap();
        let sel = selection_for(&store, &instance(&store, id)).unwrap();
        assert_eq!(sel.model.as_deref(), Some("from-row"));
        assert_eq!(sel.base_url.as_deref(), Some("http://gpu-box.lan:8091"));
    }

    /// Profiles get renamed and deleted. A workspace pinned to one that is gone
    /// has to still open — on ambient defaults — rather than becoming
    /// unopenable because of an unrelated config edit.
    #[test]
    fn a_dangling_profile_name_does_not_break_the_spawn() {
        let (store, id) = seed("other model=m");
        store
            .set_instance_model(id, Some("from-row"), None)
            .unwrap();
        store
            .set_instance_model_profile(id, Some("deleted-since"))
            .unwrap();
        let sel = selection_for(&store, &instance(&store, id)).unwrap();
        assert_eq!(sel.model.as_deref(), Some("from-row"));
        assert_eq!(sel.base_url, None);
    }

    #[test]
    fn parses_a_profile_with_every_field() {
        let p = parse(
            "local  base_url=http://127.0.0.1:8091 model=qwen3.8-27b \
             auth_token_env=QWEN_TOKEN max_context=212992",
        );
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "local");
        assert_eq!(p[0].base_url.as_deref(), Some("http://127.0.0.1:8091"));
        assert_eq!(p[0].model.as_deref(), Some("qwen3.8-27b"));
        assert_eq!(p[0].auth_token_env.as_deref(), Some("QWEN_TOKEN"));
        assert_eq!(p[0].max_context, Some(212_992));
    }

    #[test]
    fn blank_and_comment_lines_are_not_profiles() {
        let p = parse("\n# a comment\n   \nlocal model=m\n");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "local");
    }

    /// `state.db` is an unencrypted file that travels with a home directory, so
    /// a literal token in a setting would be a credential at rest that nothing
    /// knows how to rotate. The rejection has to name the alternative, or the
    /// user simply cannot tell what they are supposed to do instead.
    #[test]
    fn a_literal_credential_is_refused_and_points_at_the_alternative() {
        for key in ["auth_token", "token", "api_key", "apikey", "password"] {
            let text = format!("local base_url=http://x model=m {key}=sk-secret-value");
            let err = validate(&text).unwrap_err().to_string();
            assert!(err.contains(key), "should name the offending key: {err}");
            assert!(
                err.contains("auth_token_env"),
                "should point at the alternative: {err}"
            );
            assert!(
                parse(&text).is_empty(),
                "a refused line must not load either"
            );
        }
    }

    /// Tolerant on read, strict on write: a dashboard that is already running
    /// must not lose every profile because one line was fat-fingered, but the
    /// person typing it should hear about it immediately.
    #[test]
    fn read_is_tolerant_where_write_is_strict() {
        let text = "good base_url=http://x\nbroken not-a-pair\nalso-good model=m";
        let loaded = parse(text);
        assert_eq!(loaded.len(), 2, "the two usable lines still load");
        let err = validate(text).unwrap_err().to_string();
        assert!(err.contains("line 2"), "must locate the bad line: {err}");
    }

    #[test]
    fn a_profile_that_would_do_nothing_is_refused() {
        let err = validate("empty auth_token_env=TOK")
            .unwrap_err()
            .to_string();
        assert!(err.contains("neither base_url nor model"), "{err}");
    }

    /// A base_url without a scheme cannot work, and the failure would
    /// otherwise surface as an opaque connection error inside an agent that
    /// has already spawned — long after the person who typed it has moved on.
    /// The same server written two ways has to be one endpoint, or two agents
    /// queuing on one GPU each report that they are queuing on nothing.
    /// A name that cannot be typed as an argument everywhere it is accepted is
    /// worse than one that is refused: it can be created and then not referred
    /// to.
    #[test]
    fn profile_names_cannot_start_with_a_dash() {
        let err = validate("-x base_url=http://h:1 model=m")
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot start with"), "{err}");
        assert!(err.contains("flag"), "should say why: {err}");
        assert!(parse("-x base_url=http://h:1 model=m").is_empty());
        assert!(validate("x base_url=http://h:1 model=m").is_ok());
    }

    #[test]
    fn base_url_trailing_slashes_are_normalised_away() {
        let p = parse("a base_url=http://127.0.0.1:8091/ model=m");
        assert_eq!(p[0].base_url.as_deref(), Some("http://127.0.0.1:8091"));
        let p = parse("a base_url=http://127.0.0.1:8091/// model=m");
        assert_eq!(p[0].base_url.as_deref(), Some("http://127.0.0.1:8091"));

        // Two profiles naming one server compare equal, which is what the
        // contention count relies on.
        let both = parse("a base_url=http://h:1 model=m\nb base_url=http://h:1/ model=m");
        assert_eq!(both[0].base_url, both[1].base_url);
    }

    #[test]
    fn base_url_must_carry_a_scheme() {
        for bad in ["not-a-url", "127.0.0.1:8091", "ftp://host/x"] {
            let err = validate(&format!("p base_url={bad} model=m"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("http://"), "should name the fix: {err}");
        }
        assert!(validate("p base_url=http://127.0.0.1:8091 model=m").is_ok());
        assert!(validate("p base_url=https://api.example.com model=m").is_ok());
    }

    /// Zero is not a smaller context window, it is an unusable one.
    #[test]
    fn max_context_must_be_positive() {
        let err = validate("p model=m max_context=0").unwrap_err().to_string();
        assert!(err.contains("greater than zero"), "{err}");
    }

    #[test]
    fn max_context_must_be_a_number() {
        let err = validate("local model=m max_context=lots")
            .unwrap_err()
            .to_string();
        assert!(err.contains("max_context"), "{err}");
    }

    /// Two profiles with one name would make `--profile` ambiguous, and which
    /// one won would depend on parse order rather than on anything the user
    /// could see.
    #[test]
    fn duplicate_names_are_refused() {
        let err = validate("dup model=a\ndup model=b")
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate"), "{err}");
    }

    #[test]
    fn unknown_fields_are_named_along_with_what_is_allowed() {
        let err = validate("local model=m colour=red")
            .unwrap_err()
            .to_string();
        assert!(err.contains("colour"), "{err}");
        assert!(err.contains("base_url"), "should list known fields: {err}");
    }
}
