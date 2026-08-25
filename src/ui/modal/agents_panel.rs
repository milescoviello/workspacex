//! Extracted from ui/modal.rs.

use super::*;

/// Render the floating agents-panel modal. Lists all instances attached to
/// the workspace and lets the user add / add-all / remove agents.
/// Called directly from `render.rs` with live app state — never goes through
/// the generic `render()` function.
/// `agents` pairs each instance with the model its **live** session actually
/// started on, which the caller reads from the session rather than the row.
/// The two differ whenever a pin changed while an agent was running, and that
/// difference is the whole reason this panel shows models at all: without it,
/// pressing `p` looks like it did nothing.
pub fn render_agents_panel(
    f: &mut Frame,
    area: Rect,
    agents: &[(crate::data::agents::AgentInstance, Option<String>)],
    selected: usize,
    theme: &Theme,
) {
    let inner = panel_frame(f, area, 60, 16, " agents ", theme);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("Attached:"));
    for (a, running) in agents {
        let tag = if a.is_primary { "  (primary)" } else { "" };
        let mut spans = vec![
            Span::styled("▎", theme.agent_style(a.agent)),
            Span::raw(format!(" {}{}", a.label(), tag)),
        ];
        let pinned = a.model_profile.as_deref().or(a.model.as_deref());
        match (running.as_deref(), pinned) {
            // Running on something, with a pin that has not taken effect. A
            // process's environment is fixed at spawn, so the pin waits for a
            // respawn — say so, or the keypress reads as broken.
            (Some(run), Some(pin)) if run != pin => spans.push(Span::styled(
                format!("  [{run} → {pin} next spawn]"),
                theme.dim_style(),
            )),
            (Some(run), _) => spans.push(Span::styled(format!("  [{run}]"), theme.dim_style())),
            // Nothing running yet: the pin is what it will start on.
            (None, Some(pin)) => spans.push(Span::styled(format!("  [{pin}]"), theme.dim_style())),
            (None, None) => {}
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
