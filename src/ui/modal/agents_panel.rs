//! Extracted from ui/modal.rs.

use super::*;

/// One row of the panel: the instance, whether it is running, the model its
/// live session started on, and what it would start on next when that differs.
pub type AgentRow = (
    crate::data::agents::AgentInstance,
    bool,
    Option<String>,
    Option<String>,
);

/// Render the floating agents-panel modal. Lists all instances attached to
/// the workspace and lets the user add / add-all / remove agents.
/// Called directly from `render.rs` with live app state — never goes through
/// the generic `render()` function.
/// `agents` pairs each instance with whether it is running, the model its live
/// session actually started on, and what it would start on next when that
/// differs — all decided by the caller, from the session rather than the row.
///
/// The pending value is computed once, centrally, rather than by comparing the
/// running model against the pin here: those are a model name and a profile
/// name, and comparing them claimed a pending change every time a profile was
/// named differently from the model it selects — which is nearly always.
///
/// Liveness is carried separately because "no model" alone is ambiguous: it
/// means both "no agent" and "an agent on its own default", and those want
/// different sentences.
pub fn render_agents_panel(
    f: &mut Frame,
    area: Rect,
    agents: &[AgentRow],
    shared: bool,
    selected: usize,
    theme: &Theme,
) {
    let inner = panel_frame(f, area, 60, 16, " agents ", theme);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("Attached:"));
    for (a, live, running, pending) in agents {
        let tag = if a.is_primary { "  (primary)" } else { "" };
        let mut spans = vec![
            Span::styled("\u{258E}", theme.agent_style(a.agent)),
            Span::raw(format!(" {}{}", a.label(), tag)),
        ];
        // A shared workspace keeps its agent in a tmux server that outlives the
        // client, and re-attaching does not re-run the command, so "next spawn"
        // there would promise something that never happens.
        let when = if shared { "tmux restart" } else { "next spawn" };
        let label = match (live, running.as_deref(), pending.as_deref()) {
            // Not running: whatever it will start on, stated plainly. No arrow
            // — there is no current state for it to be a change from.
            (false, _, Some(next)) => Some(next.to_string()),
            (false, _, None) => None,
            // Running, with a change queued behind it.
            (true, run, Some(next)) => Some(format!(
                "{} \u{2192} {next} {when}",
                run.unwrap_or("agent default")
            )),
            (true, Some(run), None) => Some(run.to_string()),
            (true, None, None) => Some("agent default".to_string()),
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
        "Enter add   a add all   x remove   \u{2191}\u{2193} move   Esc close",
    ));
    // On its own line for two reasons: adding it to the row above pushed that
    // row past the panel's 58 inner columns and chopped "Esc close" down to
    // "Esc"; and the attached list has no cursor, so "p model" alone never said
    // which row it moves. It moves the primary.
    lines.push(Line::from("p   cycle the primary agent's model"));

    f.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::agents::AgentInstance;
    use crate::data::store::{AgentInstanceId, WorkspaceId};
    use crate::pty::AgentKind;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn instance() -> AgentInstance {
        AgentInstance {
            id: AgentInstanceId(1),
            workspace_id: WorkspaceId(1),
            agent: AgentKind::Claude,
            ordinal: 1,
            is_primary: true,
            session_ref: None,
            created_at: 0,
            model: None,
            provider: None,
            model_profile: None,
        }
    }

    fn draw(rows: &[AgentRow], shared: bool) -> String {
        let theme = Theme::wsx();
        let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
        term.draw(|f| render_agents_panel(f, f.area(), rows, shared, 0, &theme))
            .unwrap();
        let buf = term.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// An agent running exactly what its pin selects has nothing queued. The
    /// panel used to decide this by comparing the running model against the
    /// profile name — a model name and a profile name — and so claimed a
    /// pending change whenever a profile was named differently from the model
    /// it selects, which is nearly always.
    #[test]
    fn running_under_its_own_pin_shows_no_pending_change() {
        let mut inst = instance();
        inst.model_profile = Some("local-qwen".to_string());
        let rows = vec![(inst, true, Some("qwen3.8-27b".to_string()), None)];
        let text = draw(&rows, false);
        assert!(text.contains("[qwen3.8-27b]"), "{text}");
        assert!(
            !text.contains('→'),
            "no arrow when nothing is queued:\n{text}"
        );
    }

    #[test]
    fn a_queued_change_is_shown_with_an_arrow() {
        let mut inst = instance();
        inst.model_profile = Some("gpu-box".to_string());
        let rows = vec![(
            inst,
            true,
            Some("qwen3.8-27b".to_string()),
            Some("gpu-box".to_string()),
        )];
        let text = draw(&rows, false);
        assert!(text.contains("qwen3.8-27b → gpu-box next spawn"), "{text}");
    }

    /// A shared workspace does not respawn on re-attach, so "next spawn" there
    /// would promise something that never happens.
    #[test]
    fn a_shared_workspace_says_tmux_restart_instead() {
        let mut inst = instance();
        inst.model_profile = Some("gpu-box".to_string());
        let rows = vec![(
            inst,
            true,
            Some("qwen3.8-27b".to_string()),
            Some("gpu-box".to_string()),
        )];
        let text = draw(&rows, true);
        assert!(
            text.contains("qwen3.8-27b → gpu-box tmux restart"),
            "{text}"
        );
    }

    /// Not running: the pin is simply what it will start on, with no arrow —
    /// there is no current state for it to be a change from.
    #[test]
    fn an_idle_agent_shows_what_it_will_start_on() {
        let mut inst = instance();
        inst.model_profile = Some("local-qwen".to_string());
        let rows = vec![(inst, false, None, Some("local-qwen".to_string()))];
        let text = draw(&rows, false);
        assert!(text.contains("[local-qwen]"), "{text}");
        assert!(!text.contains('→'), "{text}");
    }

    /// Every hint line must fit the panel's inner width. Adding a model hint to
    /// the existing footer pushed it from 54 to 64 columns and silently chopped
    /// "Esc close" down to "Esc" for everyone who opened the panel.
    #[test]
    fn every_footer_line_fits_the_panel() {
        let rows = vec![(instance(), false, None, None)];
        let text = draw(&rows, false);
        assert!(
            text.contains("Esc close"),
            "close hint was truncated:\n{text}"
        );
        assert!(
            text.contains("cycle the primary agent's model"),
            "the model hint must name its target:\n{text}"
        );
        // Nothing inside the box may reach its right border.
        for line in text.lines().filter(|l| l.contains('\u{2502}')) {
            let inner: String = line.trim().trim_matches('\u{2502}').to_string();
            assert!(
                inner.chars().count() <= 58,
                "line exceeds the 58-column panel: {inner:?}"
            );
        }
    }

    /// A long profile name must not run off a fixed-width box and get clipped
    /// mid-word, leaving an unterminated bracket.
    #[test]
    fn a_long_label_is_truncated_inside_the_bracket() {
        let mut inst = instance();
        let long = "a-really-absurdly-long-profile-name-for-testing-overflow".to_string();
        inst.model_profile = Some(long.clone());
        let rows = vec![(inst, false, None, Some(long))];
        let text = draw(&rows, false);
        let row = text
            .lines()
            .find(|l| l.contains("claude"))
            .expect("agent row");
        assert!(row.contains('…'), "should be truncated: {row}");
        assert!(row.contains(']'), "bracket must still close: {row}");
    }
}
