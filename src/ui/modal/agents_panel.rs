//! Extracted from ui/modal.rs.

use super::*;

/// Render the floating agents-panel modal. Lists all instances attached to
/// the workspace and lets the user add / add-all / remove agents.
/// Called directly from `render.rs` with live app state — never goes through
/// the generic `render()` function.
/// `agents` pairs each instance with whether it is running and, if so, the
/// model its live session actually started on — both read from the session
/// rather than the row.
///
/// Liveness is carried separately because `None` alone is ambiguous: it means
/// both "no agent" and "an agent on its own default", and those want different
/// sentences. Showing the row/session difference is the whole reason this panel
/// lists models — without it, pressing `p` looks inert.
pub fn render_agents_panel(
    f: &mut Frame,
    area: Rect,
    agents: &[(crate::data::agents::AgentInstance, bool, Option<String>)],
    selected: usize,
    theme: &Theme,
) {
    let inner = panel_frame(f, area, 60, 16, " agents ", theme);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("Attached:"));
    for (a, live, running) in agents {
        let tag = if a.is_primary { "  (primary)" } else { "" };
        let mut spans = vec![
            Span::styled("\u{258E}", theme.agent_style(a.agent)),
            Span::raw(format!(" {}{}", a.label(), tag)),
        ];
        let pinned = a.model_profile.as_deref().or(a.model.as_deref());
        let label = match (live, running.as_deref(), pinned) {
            // Not running: the pin is simply what it will start on. No arrow —
            // there is no current state for it to be a change from.
            (false, _, Some(pin)) => Some(pin.to_string()),
            (false, _, None) => None,
            // Running with a pin that has not taken effect. A process's
            // environment is fixed at spawn, so the pin waits for a respawn —
            // say so, or the keypress reads as broken.
            (true, run, Some(pin)) if run != Some(pin) => Some(format!(
                "{} → {pin} next spawn",
                run.unwrap_or("agent default")
            )),
            (true, Some(run), _) => Some(run.to_string()),
            (true, None, _) => Some("agent default".to_string()),
        };
        if let Some(label) = label {
            // Budget the label against what the fixed-width panel has left
            // after the agent name, or a long profile name runs off the edge
            // and ratatui clips it mid-word — leaving an unterminated `[`.
            let used = 2 + a.label().chars().count() + tag.chars().count();
            let room = (inner.width as usize).saturating_sub(used + 4);
            let label = crate::ui::text::truncate(&label, room);
            spans.push(Span::styled(format!("  [{label}]"), theme.dim_style()));
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
