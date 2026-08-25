//! Model module. Shows what the workspace's primary agent is running on, what
//! it will switch to if that has changed underneath it, and whether it is
//! sharing an endpoint with anything else.
//!
//! Worth a slot of its own because none of this is visible anywhere else: the
//! dashboard's agent column is a colour strip with room for one bar per agent
//! and no text, so a workspace on a local endpoint looks exactly like one on
//! the default.

use crate::ui::detail_modules::{DetailContext, DetailModule};
use ratatui::text::{Line, Span};

pub struct Model;

impl DetailModule for Model {
    fn id(&self) -> &'static str {
        "model"
    }
    fn title(&self) -> &'static str {
        "MODEL"
    }
    fn lines(&self, ctx: &DetailContext<'_>, width: u16) -> Vec<Line<'static>> {
        let w = width as usize;
        let dim = ctx.theme.dim_style();
        let push = |lines: &mut Vec<Line<'static>>, text: String| {
            lines.push(Line::from(Span::styled(
                crate::ui::text::truncate(&text, w),
                dim,
            )));
        };
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Three states, not two. An agent that is not running has no current
        // model to report, so its pin *is* the answer — writing "(agent
        // default)" above "X on next spawn" describes a process that does not
        // exist and a change that is not a change.
        match (ctx.agent_live, ctx.model_running.as_deref()) {
            // Not running: whatever it will start on, stated plainly.
            (false, _) => push(
                &mut lines,
                ctx.model_pending
                    .clone()
                    .unwrap_or_else(|| "(agent default)".to_string()),
            ),
            // Running, on something nameable.
            (true, Some(model)) => {
                push(&mut lines, model.to_string());
                if let Some(pending) = ctx.model_pending.as_deref() {
                    push(&mut lines, format!("{pending} on next spawn"));
                }
            }
            // Running on the agent's own default, which is a real answer.
            (true, None) => {
                push(&mut lines, "(agent default)".to_string());
                if let Some(pending) = ctx.model_pending.as_deref() {
                    push(&mut lines, format!("{pending} on next spawn"));
                }
            }
        }

        // Counted from what agents are actually running on rather than from
        // what they are pinned to. wsx exists to run many agents at once; this
        // is the one case where that stops being free, because they queue on
        // one server instead of running in parallel.
        if ctx.endpoint_peers > 0 {
            let others = if ctx.endpoint_peers == 1 {
                "1 other workspace".to_string()
            } else {
                format!("{} other workspaces", ctx.endpoint_peers)
            };
            push(&mut lines, format!("shared with {others}"));
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::detail_modules::tests_helpers::stub_context;

    fn text_of(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn id_and_title_are_stable() {
        assert_eq!(Model.id(), "model");
        assert_eq!(Model.title(), "MODEL");
    }

    /// A workspace that has never been attached has no current model. Its pin
    /// is simply what it will start on, and saying "(agent default)" above
    /// "X on next spawn" describes a process that does not exist and a change
    /// that is not a change.
    #[test]
    fn a_workspace_that_is_not_running_shows_what_it_will_start_on() {
        let mut ctx = stub_context();
        ctx.agent_live = false;
        ctx.model_pending = Some("local-qwen".to_string());
        assert_eq!(text_of(&Model.lines(&ctx, 40)), vec!["local-qwen"]);
    }

    #[test]
    fn not_running_and_unpinned_reads_as_the_agent_default() {
        let ctx = stub_context();
        assert_eq!(text_of(&Model.lines(&ctx, 40)), vec!["(agent default)"]);
    }

    #[test]
    fn a_running_agent_leads_with_what_it_is_actually_on() {
        let mut ctx = stub_context();
        ctx.agent_live = true;
        ctx.model_running = Some("qwen3.8-27b".to_string());
        assert_eq!(text_of(&Model.lines(&ctx, 40)), vec!["qwen3.8-27b"]);
    }

    /// Changing a pin cannot touch a process that has already started, so the
    /// contrast is only meaningful while something is running.
    #[test]
    fn a_running_agent_shows_a_pin_that_has_not_taken_effect() {
        let mut ctx = stub_context();
        ctx.agent_live = true;
        ctx.model_running = Some("claude-opus".to_string());
        ctx.model_pending = Some("local-qwen".to_string());
        assert_eq!(
            text_of(&Model.lines(&ctx, 40)),
            vec!["claude-opus", "local-qwen on next spawn"]
        );
    }

    /// Running on the agent's own default is a real answer, and distinct from
    /// not running at all — which is why liveness is carried separately.
    #[test]
    fn running_on_the_agent_default_still_reports_a_pending_pin() {
        let mut ctx = stub_context();
        ctx.agent_live = true;
        ctx.model_running = None;
        ctx.model_pending = Some("local-qwen".to_string());
        assert_eq!(
            text_of(&Model.lines(&ctx, 40)),
            vec!["(agent default)", "local-qwen on next spawn"]
        );
    }

    #[test]
    fn contention_is_reported_only_when_it_exists() {
        let mut ctx = stub_context();
        ctx.agent_live = true;
        ctx.model_running = Some("qwen3.8-27b".to_string());
        assert_eq!(Model.lines(&ctx, 40).len(), 1, "no peers, no extra line");

        ctx.endpoint_peers = 1;
        assert_eq!(
            text_of(&Model.lines(&ctx, 40))[1],
            "shared with 1 other workspace"
        );

        ctx.endpoint_peers = 3;
        assert_eq!(
            text_of(&Model.lines(&ctx, 40))[1],
            "shared with 3 other workspaces"
        );
    }

    #[test]
    fn every_line_is_truncated_to_the_column() {
        let mut ctx = stub_context();
        ctx.agent_live = true;
        ctx.model_running = Some("a-very-long-model-name-that-will-not-fit".to_string());
        ctx.model_pending = Some("a-very-long-profile-name-either".to_string());
        ctx.endpoint_peers = 2;
        for line in text_of(&Model.lines(&ctx, 12)) {
            assert!(line.chars().count() <= 12, "not truncated: {line:?}");
        }
    }
}
