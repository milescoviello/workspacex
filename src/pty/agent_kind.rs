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

/// How an agent can be pointed at a non-default endpoint for one spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointSupport {
    /// An arbitrary URL, passed through the environment.
    BaseUrl,
    /// Not an arbitrary URL — only a named local provider the agent already
    /// knows how to speak to (`ollama`, `lmstudio`).
    LocalProvider,
    /// Nothing wsx can set per spawn; the agent's own config decides.
    None,
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

    /// How — if at all — this agent can be pointed somewhere other than its
    /// default endpoint for a single spawn.
    ///
    /// Established by reading each tool rather than assuming they are alike;
    /// they are not, and the differences decide what a profile can promise:
    ///
    /// - **claude** takes `ANTHROPIC_BASE_URL` from the environment, so any URL
    ///   works.
    /// - **pi** takes `LLAMA_BASE_URL` ("llama.cpp server URL"), so any
    ///   llama.cpp-compatible URL works. It has no `--base-url` flag; its other
    ///   providers get their URL from its own config.
    /// - **codex** cannot be given an arbitrary URL: a custom `model_providers`
    ///   entry only accepts `wire_api = "responses"` (0.149.1 rejects "chat"),
    ///   and local servers speak chat-completions. It ships `--oss
    ///   --local-provider ollama|lmstudio` for exactly this, so it is reachable
    ///   by *provider name* instead.
    /// - **hermes** resolves `base_url` as `arg or config.yaml or
    ///   OPENROUTER_BASE_URL` — the config file beats the environment and there
    ///   is no flag, so nothing wsx can set per spawn will move it.
    /// - **omp** takes custom providers only from `~/.omp/agent/models.yml`.
    ///
    /// Saying this explicitly is what stops a profile from being pinned to an
    /// agent that will quietly ignore half of it.
    pub fn endpoint_support(self) -> EndpointSupport {
        match self {
            AgentKind::Claude => EndpointSupport::BaseUrl,
            AgentKind::Codex => EndpointSupport::LocalProvider,
            // pi belongs here with hermes and omp, not with claude, and this
            // was measured rather than reasoned. pi resolves its llama.cpp
            // endpoint as `stored credential ?? $LLAMA_BASE_URL`, and the
            // credential written by `/login llama.cpp` always carries a URL —
            // so once pi is usable at all the environment is never consulted.
            // Driven directly: a logged-in pi answered normally with
            // `LLAMA_BASE_URL` pointing at a dead port. Without a credential
            // the variable *is* read, but `refreshModels` only fetches a
            // catalog from a credential, so there are no models to select and
            // nothing can run. There is no state in which wsx moves pi's
            // endpoint.
            AgentKind::Pi | AgentKind::Hermes | AgentKind::Omp => EndpointSupport::None,
        }
    }

    /// Whether a profile's `base_url` means anything for this agent.
    pub fn supports_endpoint(self) -> bool {
        !matches!(self.endpoint_support(), EndpointSupport::None)
    }

    /// Provider counterpart to [`Self::model_env`]. Only the agents that
    /// separate provider from model have one.
    pub fn provider_env(self) -> Option<&'static str> {
        match self {
            AgentKind::Pi => Some("WSX_PI_PROVIDER"),
            AgentKind::Hermes => Some("WSX_HERMES_PROVIDER"),
            // Codex reads this to choose `--oss --local-provider`. It has to be
            // listed here or the capture path never records it: a
            // `WSX_CODEX_PROVIDER` set on `workspace create` would be read by
            // the builder at spawn, in a TUI process that never saw it, and the
            // workspace would come up on the paid cloud backend instead of the
            // local one that was asked for.
            AgentKind::Codex => Some("WSX_CODEX_PROVIDER"),
            // Omp has no provider path — `build_omp_command` never reads one —
            // so advertising a variable here would capture a value onto the row
            // that nothing will ever apply.
            AgentKind::Claude | AgentKind::Omp => None,
        }
    }

    pub fn from_store(store: &crate::data::store::Store) -> Self {
        Self::from_str_or_default(store.get_setting("coding_agent").ok().flatten().as_deref())
    }
}

#[cfg(test)]
mod endpoint_support_tests {
    use super::*;

    /// These are not alike, and pretending they are is how a profile ends up
    /// silently doing half of what it says. Each value was established by
    /// reading the tool, not by assuming:
    ///
    /// - claude takes `ANTHROPIC_BASE_URL`; pi takes `LLAMA_BASE_URL`.
    /// - codex rejects `wire_api = "chat"` on a custom provider (0.149.1) and
    ///   ships `--oss --local-provider` instead, so it is reachable by name.
    /// - hermes resolves `arg or config.yaml or OPENROUTER_BASE_URL`, so its
    ///   config file beats anything wsx can set.
    /// - omp takes custom providers only from `~/.omp/agent/models.yml`.
    #[test]
    fn each_agent_reports_the_endpoint_mechanism_it_actually_has() {
        assert_eq!(
            AgentKind::Claude.endpoint_support(),
            EndpointSupport::BaseUrl
        );
        assert_eq!(AgentKind::Pi.endpoint_support(), EndpointSupport::None);
        assert_eq!(
            AgentKind::Codex.endpoint_support(),
            EndpointSupport::LocalProvider
        );
        assert_eq!(AgentKind::Hermes.endpoint_support(), EndpointSupport::None);
        assert_eq!(AgentKind::Omp.endpoint_support(), EndpointSupport::None);
    }

    /// `supports_endpoint` is the coarse question the warning paths ask, and it
    /// must stay consistent with the detailed answer above.
    #[test]
    fn supports_endpoint_agrees_with_the_detailed_mechanism() {
        for k in AgentKind::ALL {
            assert_eq!(
                k.supports_endpoint(),
                k.endpoint_support() != EndpointSupport::None,
                "{} disagrees with itself",
                k.display_name()
            );
        }
    }
}
