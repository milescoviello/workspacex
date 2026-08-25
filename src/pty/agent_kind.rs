//! The coding-agent taxonomy.
//!
//! `AgentKind` is the small leaf enum naming which coding agent a session
//! drives. It's re-exported from `pty::session` (and `pty`) so existing
//! `crate::pty::session::AgentKind` paths keep resolving.

/// Which coding agent to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Pi,
    Hermes,
    Codex,
    /// oh-my-pi (`omp`). A separate harness from [`AgentKind::Pi`] despite the
    /// shared ancestry: `@oh-my-pi/pi-coding-agent` vs
    /// `@earendil-works/pi-coding-agent`, different binaries, both installable
    /// at once.
    Omp,
}

impl AgentKind {
    /// All agent kinds, in stable display order. Add new variants here when
    /// extending the enum — `const` arrays do not get exhaustiveness checking,
    /// so this is the one place the compiler can't catch a drift.
    pub const ALL: [AgentKind; 5] = [
        AgentKind::Claude,
        AgentKind::Pi,
        AgentKind::Hermes,
        AgentKind::Codex,
        AgentKind::Omp,
    ];

    pub fn from_str_or_default(s: Option<&str>) -> Self {
        match s {
            Some("pi") => AgentKind::Pi,
            Some("hermes") => AgentKind::Hermes,
            Some("codex") => AgentKind::Codex,
            Some("omp") => AgentKind::Omp,
            _ => AgentKind::Claude,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Pi => "pi",
            AgentKind::Hermes => "hermes",
            AgentKind::Codex => "codex",
            AgentKind::Omp => "omp",
        }
    }

    pub fn default_binary(self) -> &'static str {
        self.display_name()
    }

    pub fn store_value(self) -> &'static str {
        self.display_name()
    }

    /// Environment variable that overrides this agent's model.
    ///
    /// The names live here rather than inline at each spawn site so the
    /// capture path (which reads them) and the builders (which fall back to
    /// them) cannot drift apart.
    pub fn model_env(self) -> Option<&'static str> {
        match self {
            AgentKind::Claude => Some("WSX_CLAUDE_MODEL"),
            AgentKind::Pi => Some("WSX_PI_MODEL"),
            AgentKind::Hermes => Some("WSX_HERMES_MODEL"),
            AgentKind::Codex => Some("WSX_CODEX_MODEL"),
            AgentKind::Omp => Some("WSX_OMP_MODEL"),
        }
    }

    /// Whether this agent can be pointed at an arbitrary endpoint by a model
    /// profile — that is, whether `base_url` / `auth_token_env` /
    /// `max_context` mean anything for it.
    ///
    /// Only `claude` today. The others take a profile's `model` but reach
    /// their endpoint through their own config, by mechanisms this crate does
    /// not model. Saying so explicitly is what stops a profile with a
    /// `base_url` from being pinned to them and silently doing half of what it
    /// looks like it does.
    pub fn supports_endpoint(self) -> bool {
        matches!(self, AgentKind::Claude)
    }

    /// Provider counterpart to [`Self::model_env`]. Only the agents that
    /// separate provider from model have one.
    pub fn provider_env(self) -> Option<&'static str> {
        match self {
            AgentKind::Pi => Some("WSX_PI_PROVIDER"),
            AgentKind::Hermes => Some("WSX_HERMES_PROVIDER"),
            AgentKind::Omp => Some("WSX_OMP_PROVIDER"),
            AgentKind::Claude | AgentKind::Codex => None,
        }
    }

    pub fn from_store(store: &crate::data::store::Store) -> Self {
        Self::from_str_or_default(store.get_setting("coding_agent").ok().flatten().as_deref())
    }
}
