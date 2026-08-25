//! Model module. Shows which model the workspace's primary agent will spawn
//! on, and where that choice came from.
//!
//! Worth a slot of its own because the answer is otherwise invisible: the
//! dashboard's agent column is a colour strip with room for a bar per agent and
//! no text, so a workspace pinned to a local endpoint looks exactly like one on
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
        let first = match ctx.model_label {
            Some(label) => Span::styled(
                crate::ui::text::truncate(label, width as usize),
                ctx.theme.dim_style(),
            ),
            // Not an error state — most workspaces have no pin and take the
            // agent's own default. Say so rather than leaving a blank slot the
            // reader has to interpret.
            None => Span::styled("(agent default)".to_string(), ctx.theme.dim_style()),
        };
        let mut lines = vec![Line::from(first)];
        // Only worth a line when it is actually true. A dashboard built to
        // encourage running many agents at once should say when two of them are
        // about to queue behind each other on one server.
        if ctx.endpoint_peers > 0 {
            let others = if ctx.endpoint_peers == 1 {
                "1 other workspace".to_string()
            } else {
                format!("{} other workspaces", ctx.endpoint_peers)
            };
            lines.push(Line::from(Span::styled(
                crate::ui::text::truncate(&format!("shared with {others}"), width as usize),
                ctx.theme.dim_style(),
            )));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::detail_modules::tests_helpers::stub_context;

    #[test]
    fn id_and_title_are_stable() {
        assert_eq!(Model.id(), "model");
        assert_eq!(Model.title(), "MODEL");
    }

    /// An unpinned workspace is the common case and must read as deliberate,
    /// not as missing data.
    #[test]
    fn no_selection_reads_as_the_agent_default() {
        let ctx = stub_context();
        let lines = Model.lines(&ctx, 40);
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "(agent default)");
    }

    /// Silence when nothing is shared; a count only when it is real. The
    /// singular case is the common one and reads badly as "1 other workspaces".
    #[test]
    fn contention_is_reported_only_when_it_exists() {
        let mut ctx = stub_context();
        ctx.model_label = Some("local-qwen");

        assert_eq!(Model.lines(&ctx, 40).len(), 1, "no peers, no second line");

        ctx.endpoint_peers = 1;
        let lines = Model.lines(&ctx, 40);
        let second: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(second, "shared with 1 other workspace");

        ctx.endpoint_peers = 3;
        let lines = Model.lines(&ctx, 40);
        let second: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(second, "shared with 3 other workspaces");
    }

    #[test]
    fn a_selection_is_shown_and_truncated_to_the_column() {
        let mut ctx = stub_context();
        ctx.model_label = Some("a-very-long-profile-name-that-will-not-fit");
        let lines = Model.lines(&ctx, 12);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.chars().count() <= 12, "not truncated: {text:?}");
        assert!(text.starts_with("a-very"), "{text:?}");
    }
}
