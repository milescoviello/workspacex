//! new workspace notice tests.

use super::*;
use crate::data::store::{NewWorkspace, Store};
use crossterm::event::KeyEvent;
use std::path::PathBuf;

fn app_with_existing_workspace() -> (App, crate::data::store::RepoId) {
    let store = Store::open_in_memory().unwrap();
    let repo_id = store
        .add_repo(std::path::Path::new("/tmp/r"), "repo", "")
        .unwrap();
    store
        .insert_workspace(&NewWorkspace {
            repo_id,
            name: "alpha",
            branch: "repo/alpha",
            worktree_path: std::path::Path::new("/tmp/r/alpha"),
            yolo: false,
            agent: crate::pty::session::AgentKind::Claude,
            shared: false,
        })
        .unwrap();
    let app = App::new(store, PathBuf::from("/tmp/wsx-test")).unwrap();
    (app, repo_id)
}

fn dummy_shared() -> std::sync::Arc<tokio::sync::Mutex<App>> {
    std::sync::Arc::new(tokio::sync::Mutex::new(
        App::new(
            Store::open_in_memory().unwrap(),
            PathBuf::from("/tmp/wsx-test"),
        )
        .unwrap(),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_with_a_taken_name_shows_inline_notice_and_does_not_spawn() {
    let (mut app, repo_id) = app_with_existing_workspace();
    app.modal = Some(crate::ui::modal::Modal::NewWorkspace {
        repo_id,
        name_buffer: "alpha".to_string(),
        yolo: false,
        shared: false,
        agent: crate::pty::session::AgentKind::Claude,
        profile: None,
        notice: None,
    });
    let shared = dummy_shared();
    handle_key_modal(
        &mut app,
        &shared,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await
    .unwrap();
    match &app.modal {
        Some(crate::ui::modal::Modal::NewWorkspace {
            name_buffer,
            profile: None,
            notice,
            ..
        }) => {
            assert_eq!(name_buffer, "alpha", "buffer must survive the refusal");
            assert_eq!(
                notice.as_deref(),
                Some("a workspace named 'alpha' already exists")
            );
        }
        other => panic!("expected NewWorkspace modal with a notice, got {other:?}"),
    }
    assert!(
        app.in_flight.is_empty(),
        "a duplicate name must never spawn a create task"
    );
    assert_eq!(
        app.store.workspaces(repo_id).unwrap().len(),
        1,
        "no second row should be inserted"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_after_a_duplicate_notice_clears_it() {
    let (mut app, repo_id) = app_with_existing_workspace();
    // Simulate: user already hit a duplicate once (notice set), then
    // starts editing the buffer — mirrors `RenameWorkspace`'s Backspace/
    // Char arms, which clear `notice` on any edit.
    app.modal = Some(crate::ui::modal::Modal::NewWorkspace {
        repo_id,
        name_buffer: "alpha".to_string(),
        yolo: false,
        shared: false,
        agent: crate::pty::session::AgentKind::Claude,
        profile: None,
        notice: Some("a workspace named 'alpha' already exists".to_string()),
    });
    let shared = dummy_shared();
    handle_key_modal(
        &mut app,
        &shared,
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
    )
    .await
    .unwrap();
    match &app.modal {
        Some(crate::ui::modal::Modal::NewWorkspace {
            name_buffer,
            profile: None,
            notice,
            ..
        }) => {
            assert_eq!(name_buffer, "alpha2");
            assert!(notice.is_none(), "editing must clear the stale notice");
        }
        other => panic!("expected NewWorkspace modal, got {other:?}"),
    }
}

/// `^p` walks the new-workspace modal through the configured profiles and back
/// to the agent's default, without disturbing the name being typed.
///
/// Creation is the only moment a model choice applies without a respawn, so if
/// it cannot be made here the feature is effectively CLI-only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_p_cycles_the_model_profile_in_the_new_workspace_modal() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let (mut app, repo_id) = app_with_existing_workspace();
    app.store
        .set_setting(
            "model_profiles",
            "alpha base_url=http://a\nbeta base_url=http://b",
        )
        .unwrap();
    app.modal = Some(crate::ui::modal::Modal::NewWorkspace {
        repo_id,
        name_buffer: "typed-so-far".to_string(),
        yolo: false,
        shared: false,
        agent: crate::pty::session::AgentKind::Claude,
        profile: None,
        notice: None,
    });
    let shared = dummy_shared();

    let state = |app: &App| -> (Option<String>, String) {
        match app.modal.as_ref() {
            Some(crate::ui::modal::Modal::NewWorkspace {
                profile,
                name_buffer,
                ..
            }) => (profile.clone(), name_buffer.clone()),
            other => panic!("expected the NewWorkspace modal, got {other:?}"),
        }
    };
    let press = async |app: &mut App| {
        handle_key_modal(
            app,
            &shared,
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
        )
        .await
        .unwrap();
    };

    press(&mut app).await;
    assert_eq!(state(&app).0.as_deref(), Some("alpha"));
    press(&mut app).await;
    assert_eq!(state(&app).0.as_deref(), Some("beta"));
    press(&mut app).await;
    assert_eq!(
        state(&app).0,
        None,
        "past the last profile returns to the agent default"
    );
    assert_eq!(
        state(&app).1,
        "typed-so-far",
        "cycling the model must not disturb the name being typed"
    );
}

/// With no profiles configured the key is inert rather than erroring — there is
/// simply nothing to choose.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_p_is_inert_when_no_profiles_are_configured() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let (mut app, repo_id) = app_with_existing_workspace();
    app.modal = Some(crate::ui::modal::Modal::NewWorkspace {
        repo_id,
        name_buffer: String::new(),
        yolo: false,
        shared: false,
        agent: crate::pty::session::AgentKind::Claude,
        profile: None,
        notice: None,
    });
    let shared = dummy_shared();
    handle_key_modal(
        &mut app,
        &shared,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
    )
    .await
    .unwrap();
    match app.modal.as_ref() {
        Some(crate::ui::modal::Modal::NewWorkspace { profile, .. }) => assert_eq!(*profile, None),
        other => panic!("expected the NewWorkspace modal, got {other:?}"),
    }
}
