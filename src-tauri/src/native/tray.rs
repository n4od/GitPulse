//! The system tray.
//!
//! The icon is the app's only persistent presence, so it carries the one piece
//! of information worth glancing at: whether anything is failing.

use std::sync::Arc;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::db::RepoCard;
use crate::native::icon::{icon_for, TrayState};
use crate::native::notifier::Notifier;
use crate::poller::Poller;

pub const TRAY_ID: &str = "pulse-tray";

/// How long the tray's mute item silences notifications.
const MUTE_DURATION: Duration = Duration::from_secs(60 * 60);

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Pulse", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause polling", true, None::<&str>)?;
    let mute = MenuItem::with_id(
        app,
        "mute",
        "Mute notifications for 1 hour",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&open, &pause, &mute, &separator, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(TrayState::Idle))
        .tooltip(TrayState::Idle.tooltip())
        .menu(&menu)
        // Left click opens the window; the menu is the right-click behaviour.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_window(app),
            "pause" => toggle_pause(app),
            "mute" => mute_notifications(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Recomputes the tray from the current cache.
///
/// Called after every poll; it reads the cards rather than taking a state, so
/// the icon can never disagree with what the dashboard shows.
pub fn refresh<R: Runtime>(app: &AppHandle<R>, cards: &[RepoCard], signed_in: bool) {
    let state = state_for(cards, signed_in);

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let _ = tray.set_icon(Some(icon_for(state)));
    let _ = tray.set_tooltip(Some(state.tooltip()));

    #[cfg(target_os = "macos")]
    {
        let failures = cards.iter().filter(|card| card.failing_runs > 0).count();
        let _ = app.set_badge_count(i64::try_from(failures).ok());
    }
}

#[must_use]
pub fn state_for(cards: &[RepoCard], signed_in: bool) -> TrayState {
    if !signed_in {
        return TrayState::AuthRequired;
    }

    if cards.iter().any(|card| card.failing_runs > 0) {
        return TrayState::Failure;
    }

    if cards.iter().any(|card| card.has_active_run) {
        return TrayState::Running;
    }

    TrayState::Idle
}

fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        _ => show_window(app),
    }
}

fn toggle_pause<R: Runtime>(app: &AppHandle<R>) {
    let Some(poller) = app.try_state::<Arc<Poller>>() else {
        return;
    };

    let next = !poller.is_paused();
    poller.set_paused(next);
    tracing::info!(paused = next, "polling toggled from the tray");
}

fn mute_notifications<R: Runtime>(app: &AppHandle<R>) {
    let Some(notifier) = app.try_state::<Arc<Notifier>>() else {
        return;
    };

    notifier.mute_for(MUTE_DURATION);
    tracing::info!(?MUTE_DURATION, "notifications muted from the tray");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{make_card, make_repo};

    #[test]
    fn signed_out_takes_priority_over_everything() {
        let cards = vec![make_card(make_repo(), 5, true)];

        assert_eq!(state_for(&cards, false), TrayState::AuthRequired);
    }

    #[test]
    fn a_failure_beats_a_running_build() {
        let cards = vec![
            make_card(make_repo(), 0, true),
            make_card(make_repo(), 2, false),
        ];

        assert_eq!(state_for(&cards, true), TrayState::Failure);
    }

    #[test]
    fn a_running_build_shows_while_nothing_fails() {
        let cards = vec![make_card(make_repo(), 0, true)];

        assert_eq!(state_for(&cards, true), TrayState::Running);
    }

    #[test]
    fn nothing_happening_is_idle() {
        let cards = vec![make_card(make_repo(), 0, false)];

        assert_eq!(state_for(&cards, true), TrayState::Idle);
    }

    #[test]
    fn an_empty_watchlist_is_idle_not_signed_out() {
        assert_eq!(state_for(&[], true), TrayState::Idle);
    }
}
