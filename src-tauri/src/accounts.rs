//! The account a piece of local state belongs to.
//!
//! Two things are keyed by account — the keychain entries and the database file —
//! and both have to agree on the spelling, or one would write `n4od` while the
//! other looked for `N4OD`. The rule lives here so there is one answer.

/// GitHub logins are case-insensitive, and so are the filesystems and credential
/// stores this runs on: `n4od` and `N4OD` are one account and must be one key.
///
/// The display name is never normalised — it is GitHub's to spell.
pub fn normalize(login: &str) -> String {
    login.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn a_login_is_lowercased_and_trimmed() {
        assert_eq!(normalize("N4OD"), "n4od");
        assert_eq!(normalize("  octocat  "), "octocat");
        assert_eq!(normalize("Already"), "already");
    }
}
