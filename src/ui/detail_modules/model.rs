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
        let line = match ctx.model_label {
            Some(label) => Span::styled(
                crate::ui::text::truncate(label, width as usize),
                ctx.theme.dim_style(),
            ),
            // Not an error state — most workspaces have no pin and take the
            // agent's own default. Say so rather than leaving a blank slot the
            // reader has to interpret.
            None => Span::styled("(agent default)".to_string(), ctx.theme.dim_style()),
        };
        vec![Line::from(line)]
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
