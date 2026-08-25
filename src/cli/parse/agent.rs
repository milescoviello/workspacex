//! `wsx agent` — listing agents and sending them prompts.

use super::Args;
use crate::cli::action::CliAction;
use crate::error::{Error, Result};

pub(in crate::cli) const USAGE_AGENT_SEND: &str =
    "agent send [--workspace <repo>/<slug>] <label> <prompt>";

pub(in crate::cli) fn parse_agent(it: &mut Args) -> Result<CliAction> {
    match it.next().as_deref() {
        Some("list") => Ok(CliAction::AgentList),
        Some("send") => {
            let mut workspace: Option<String> = None;
            // Flags are recognised ONLY before the label. Everything from the
            // label onward is positional, so a message body that itself starts
            // with `--` is preserved verbatim.
            let target = loop {
                let arg = it.next().ok_or_else(|| Error::Usage {
                    group: None,
                    msg: USAGE_AGENT_SEND.into(),
                })?;
                match arg.as_str() {
                    "--workspace" => {
                        workspace = Some(it.next().ok_or_else(|| Error::Usage {
                            group: None,
                            msg: "--workspace needs value (<repo>/<slug>)".into(),
                        })?);
                    }
                    _ => break arg,
                }
            };
            let rest: Vec<String> = it.collect();
            if rest.is_empty() {
                return Err(Error::Usage {
                    group: None,
                    msg: USAGE_AGENT_SEND.into(),
                });
            }
            Ok(CliAction::AgentSend {
                target,
                prompt: rest.join(" "),
                workspace,
            })
        }
        Some("add") => {
            let kind = it.next().ok_or_else(|| Error::Usage {
                group: None,
                msg: "agent add <kind>".into(),
            })?;
            // Validate against the canonical agent set so this can't drift
            // from `AgentKind` as kinds are added/renamed.
            use crate::pty::session::AgentKind;
            if !AgentKind::ALL.iter().any(|k| k.display_name() == kind) {
                let valid = AgentKind::ALL
                    .iter()
                    .map(|k| k.display_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(Error::Usage {
                    group: None,
                    msg: format!("agent add: kind must be one of [{valid}], got '{kind}'"),
                });
            }
            Ok(CliAction::AgentAdd { kind })
        }
        Some("profile") => {
            let mut name: Option<String> = None;
            let mut clear = false;
            let mut target: Option<String> = None;
            while let Some(arg) = it.next() {
                match arg.as_str() {
                    // `--clear` rather than a bare missing argument: dropping a
                    // pin is destructive enough that it should be asked for by
                    // name, not achieved by forgetting to type something.
                    "--clear" => clear = true,
                    "--agent" => {
                        target = Some(it.next().ok_or_else(|| Error::Usage {
                            group: None,
                            msg: "--agent needs a label (claude, claude#2, …)".into(),
                        })?);
                    }
                    other if !other.starts_with('-') && name.is_none() => {
                        name = Some(other.to_string());
                    }
                    other => {
                        return Err(Error::Usage {
                            group: None,
                            msg: format!("agent profile: unexpected argument '{other}'"),
                        });
                    }
                }
            }
            if clear && name.is_some() {
                return Err(Error::Usage {
                    group: None,
                    msg: "agent profile: pass a name or --clear, not both".into(),
                });
            }
            if !clear && name.is_none() {
                return Err(Error::Usage {
                    group: None,
                    msg: "agent profile [--agent <label>] <name|--clear>".into(),
                });
            }
            Ok(CliAction::AgentProfile { name, target })
        }
        _ => Err(Error::Usage {
            group: None,
            msg: "agent <list|send|add|profile> ...".into(),
        }),
    }
}
