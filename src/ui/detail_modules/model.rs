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
        let mut lines: Vec<Line<'static>> = Vec::new();

        match ctx.model_running.as_deref() {
            Some(model) => lines.push(Line::from(Span::styled(
                crate::ui::text::truncate(model, w),
                dim,
            ))),
            // Not an error state — an agent that is not running, or one on the
            // agent's own default, is the common case. Say so rather than
            // leaving a blank the reader has to interpret.
            None => lines.push(Line::from(Span::styled("(agent default)".to_string(), dim))),
        }

        // A pin that has changed since the agent started cannot take effect
        // until it respawns — a process's environment is fixed when it starts.
        // Saying so is the difference between "this did nothing" and "this is
        // queued", which is otherwise invisible and reads as a broken keypress.
        if let Some(pending) = ctx.model_pending.as_deref() {
            lines.push(Line::from(Span::styled(
                crate::ui::text::truncate(&format!("{pending} on next spawn"), w),
                dim,
            )));
        }

        // Only when it is true, and counted from what agents are actually
        // running rather than from what they are pinned to. wsx exists to run
        // many agents at once; this is the one case where that stops being
        // free, because they queue on one server instead of running in
        // parallel.
        if ctx.endpoint_peers > 0 {
            let others = if ctx.endpoint_peers == 1 {
                "1 other workspace".to_string()
            } else {
                format!("{} other workspaces", ctx.endpoint_peers)
            };
            lines.push(Line::from(Span::styled(
                crate::ui::text::truncate(&format!("shared with {others}"), w),
                dim,
            )));
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

    /// Nothing running, or running on the agent's own default, is the common
    /// case and must read as deliberate rather than as missing data.
    #[test]
    fn no_running_model_reads_as_the_agent_default() {
        let ctx = stub_context();
        assert_eq!(text_of(&Model.lines(&ctx, 40)), vec!["(agent default)"]);
    }

    /// The running model is a fact about the live process, so it leads.
    #[test]
    fn the_running_model_is_shown_first() {
        let mut ctx = stub_context();
        ctx.model_running = Some("qwen3.8-27b".to_string());
        assert_eq!(text_of(&Model.lines(&ctx, 40)), vec!["qwen3.8-27b"]);
    }

    /// Changing a pin cannot touch a process that has already started. Without
    /// this line the keypress looks like it did nothing at all.
    #[test]
    fn a_pin_that_has_not_taken_effect_says_so() {
        let mut ctx = stub_context();
        ctx.model_running = Some("cloud".to_string());
        ctx.model_pending = Some("local-qwen".to_string());
        assert_eq!(
            text_of(&Model.lines(&ctx, 40)),
            vec!["cloud", "local-qwen on next spawn"]
        );
    }

    /// When the pin already matches what is running there is nothing pending,
    /// and repeating it would be noise.
    #[test]
    fn nothing_pending_when_the_pin_already_matches() {
        let mut ctx = stub_context();
        ctx.model_running = Some("qwen3.8-27b".to_string());
        ctx.model_pending = None;
        assert_eq!(text_of(&Model.lines(&ctx, 40)).len(), 1);
    }

    #[test]
    fn contention_is_reported_only_when_it_exists() {
        let mut ctx = stub_context();
        ctx.model_running = Some("qwen3.8-27b".to_string());

        assert_eq!(Model.lines(&ctx, 40).len(), 1, "no peers, no second line");

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
        ctx.model_running = Some("a-very-long-model-name-that-will-not-fit".to_string());
        ctx.model_pending = Some("a-very-long-profile-name-either".to_string());
        ctx.endpoint_peers = 2;
        for line in text_of(&Model.lines(&ctx, 12)) {
            assert!(line.chars().count() <= 12, "not truncated: {line:?}");
        }
    }
}
