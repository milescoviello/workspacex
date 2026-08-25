//! Extracted from ui/modal.rs.

use super::*;

/// Render the floating agents-panel modal. Lists all instances attached to
/// the workspace and lets the user add / add-all / remove agents.
/// Called directly from `render.rs` with live app state — never goes through
/// the generic `render()` function.
pub fn render_agents_panel(
    f: &mut Frame,
    area: Rect,
    agents: &[crate::data::agents::AgentInstance],
    selected: usize,
    theme: &Theme,
) {
    let inner = panel_frame(f, area, 60, 16, " agents ", theme);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("Attached:"));
    for a in agents {
        let tag = if a.is_primary { "  (primary)" } else { "" };
        let mut spans = vec![
            Span::styled("▎", theme.agent_style(a.agent)),
            Span::raw(format!(" {}{}", a.label(), tag)),
        ];
        // Profile first, then the model recorded at creation: the profile is
        // what someone chose, the model is what happened to be exported.
        if let Some(model) = a.model_profile.as_deref().or(a.model.as_deref()) {
            spans.push(Span::styled(format!("  [{model}]"), theme.dim_style()));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Add:"));
    let add: Vec<Span> = crate::pty::session::AgentKind::ALL
        .iter()
        .enumerate()
        .flat_map(|(i, k)| {
            let marker = if i == selected { "> " } else { "  " };
            vec![
                Span::raw(marker.to_string()),
                Span::styled("▎", theme.agent_style(*k)),
                Span::raw(format!("{}   ", k.display_name())),
            ]
        })
        .collect();
    lines.push(Line::from(add));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter add   a add all   x remove   p model   \u{2191}\u{2193} move   Esc close",
    ));

    f.render_widget(Paragraph::new(lines), inner);
}
