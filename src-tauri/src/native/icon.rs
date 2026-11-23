//! Tray icons, generated at runtime.
//!
//! Four states need four icons, and shipping four PNGs for a 16-pixel dot is
//! more moving parts than drawing one. The circle is antialiased by measuring
//! the distance from the centre, which at this size is indistinguishable from
//! a proper rasteriser.

use tauri::image::Image;

/// Square size in pixels. 16 is the Windows system tray size; macOS scales.
const SIZE: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    /// Nothing running and nothing failing.
    Idle,
    /// At least one workflow in flight.
    Running,
    /// At least one failure in the cache.
    Failure,
    /// No usable token; the poller is doing nothing.
    AuthRequired,
}

impl TrayState {
    /// The tooltip, naming the account it is reporting on.
    ///
    /// The tray is the app's only persistent presence, and now that two accounts
    /// can be switched between, "build failures" without an owner is ambiguous —
    /// both accounts can see a repository called `api`.
    #[must_use]
    pub fn tooltip(self, account: Option<&str>) -> String {
        let what = match self {
            // Signing out has no account to name, and "GitPulse — n4od — sign in
            // required" would be a lie about whose session is missing.
            Self::AuthRequired => return "GitPulse — sign in required".to_owned(),
            Self::Idle => "all clear",
            Self::Running => "builds running",
            Self::Failure => "build failures",
        };

        match account {
            Some(login) => format!("GitPulse — {login} — {what}"),
            None => format!("GitPulse — {what}"),
        }
    }

    /// RGBA for the dot.
    const fn colour(self) -> [u8; 3] {
        match self {
            // Matches --color-ok.
            Self::Idle => [0x22, 0xC5, 0x5E],
            // Matches --color-live.
            Self::Running => [0x38, 0xBD, 0xF8],
            // Matches --color-bad.
            Self::Failure => [0xF0, 0x52, 0x52],
            // Matches --color-warn.
            Self::AuthRequired => [0xF5, 0x9E, 0x0B],
        }
    }
}

#[must_use]
pub fn icon_for(state: TrayState) -> Image<'static> {
    // SIZE is a small constant; the cast is exact.
    #[allow(clippy::cast_precision_loss)]
    let centre = (SIZE as f32 - 1.0) / 2.0;

    circle(centre, centre, centre - 0.5, state.colour())
}

/// The Windows taskbar overlay badge.
///
/// A full-bleed dot is right for the tray, where it fills the 16-pixel slot, but
/// wrong as an overlay. Windows scales the whole canvas into a slot at the
/// bottom-right of the taskbar icon, so a full-bleed circle spills past the
/// icon's corner and reads as a blob rather than a badge - which is how the
/// first screenshot of it looked. Same red, drawn with a margin.
#[must_use]
pub fn overlay_icon() -> Image<'static> {
    // A 2.5px margin on every side. Measured on a real taskbar: this took the
    // badge from 14px to 10px on a 24px icon, and pulled it from 1px past the
    // icon's corner to 1px inside it.
    #[allow(clippy::cast_precision_loss)]
    let centre = (SIZE as f32 - 1.0) / 2.0;
    circle(centre, centre, centre - 2.5, TrayState::Failure.colour())
}

/// An antialiased filled circle on a transparent square.
fn circle(centre_x: f32, centre_y: f32, radius: f32, colour: [u8; 3]) -> Image<'static> {
    let [red, green, blue] = colour;
    let mut pixels = vec![0_u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            // `as` here is bounded by SIZE, which is a small constant.
            #[allow(clippy::cast_precision_loss)]
            let (dx, dy) = (x as f32 - centre_x, y as f32 - centre_y);
            let distance = (dx * dx + dy * dy).sqrt();

            // One pixel of coverage at the edge, so the dot is not jagged.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let alpha = ((radius + 0.5 - distance).clamp(0.0, 1.0) * 255.0) as u8;

            let offset = ((y * SIZE + x) * 4) as usize;
            pixels[offset] = red;
            pixels[offset + 1] = green;
            pixels[offset + 2] = blue;
            pixels[offset + 3] = alpha;
        }
    }

    Image::new_owned(pixels, SIZE, SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_has_its_own_colour() {
        let colours = [
            TrayState::Idle,
            TrayState::Running,
            TrayState::Failure,
            TrayState::AuthRequired,
        ]
        .map(TrayState::colour);

        for (index, left) in colours.iter().enumerate() {
            for right in colours.iter().skip(index + 1) {
                assert_ne!(left, right, "two states share a colour");
            }
        }
    }

    #[test]
    fn the_centre_pixel_is_opaque_and_the_corners_are_transparent() {
        let image = icon_for(TrayState::Idle);
        let pixels = image.rgba();

        assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);

        let centre = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
        assert_eq!(pixels[centre + 3], 255, "the dot must be solid");

        assert_eq!(pixels[3], 0, "the corner must be transparent");
    }

    #[test]
    fn the_dot_uses_the_state_colour() {
        let image = icon_for(TrayState::Failure);
        let pixels = image.rgba();
        let centre = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;

        assert_eq!(&pixels[centre..centre + 3], &[0xF0, 0x52, 0x52]);
    }

    #[test]
    fn the_overlay_badge_leaves_a_margin_the_tray_dot_does_not() {
        // Both are 16px circles, but the tray dot fills its slot while the
        // overlay is drawn with a margin. This is pinned because Windows scales
        // the whole overlay canvas into the corner of the taskbar icon: a
        // full-bleed circle there spills past the icon's corner - measured at
        // 14px on a 24px icon, 1px beyond it - instead of reading as a badge.
        let tray = icon_for(TrayState::Failure);
        let badge = overlay_icon();

        let at = |x: usize, y: usize| (y * SIZE as usize + x) * 4;

        assert!(
            tray.rgba()[at(1, 8) + 3] > 0,
            "the tray dot is expected to fill the square"
        );
        assert_eq!(
            badge.rgba()[at(1, 8) + 3],
            0,
            "the overlay badge must leave a margin"
        );

        // It is still a solid badge, not a speck.
        assert_eq!(badge.rgba()[at(8, 8) + 3], 255, "the badge must be solid");
    }

    #[test]
    fn the_tooltip_names_the_account_it_is_reporting_on() {
        // Two accounts can both watch a repository called `api`, so a count
        // without an owner is ambiguous.
        assert_eq!(
            TrayState::Failure.tooltip(Some("n4od")),
            "GitPulse — n4od — build failures"
        );
        assert_eq!(
            TrayState::Idle.tooltip(Some("octocat")),
            "GitPulse — octocat — all clear"
        );
    }

    #[test]
    fn signing_out_has_no_account_to_name() {
        assert_eq!(
            TrayState::AuthRequired.tooltip(None),
            "GitPulse — sign in required"
        );
        assert_eq!(
            TrayState::AuthRequired.tooltip(Some("n4od")),
            "GitPulse — sign in required",
            "a stale login must not be named while signed out"
        );
    }

    #[test]
    fn an_account_is_named_even_when_there_is_nothing_to_report() {
        assert_eq!(
            TrayState::Idle.tooltip(None),
            "GitPulse — all clear",
            "no account yet is not an error"
        );
    }
}
