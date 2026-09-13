//! Aevum Platform — AUTH-26 User Preferences
//!
//! Per-user display and behavioral preferences.
//!
//! Security boundary:
//! - `user_id` is server-owned and MUST come from authenticated context.
//! - clients may update only fields represented by `UserPreferencesUpdate`.
//! - `updated_at` is server-owned.
//! - unknown JSON fields are rejected.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// UI theme preference.
///
/// `System` follows the operating system's `prefers-color-scheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system preference.
    #[default]
    System,

    /// Force light theme.
    Light,

    /// Force dark theme.
    Dark,
}

/// UI language preference.
///
/// Currently only English is supported.
/// Additional languages must be introduced together with frontend i18n.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English.
    #[default]
    En,
}

/// Server-owned, persisted preferences for one user.
///
/// `user_id` and `updated_at` are never client-controlled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: Uuid,
    pub theme: Theme,
    pub language: Language,
    pub notifications_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl UserPreferences {
    /// Construct canonical default preferences for a user.
    pub fn defaults(user_id: Uuid) -> Self {
        Self {
            user_id,
            theme: Theme::default(),
            language: Language::default(),
            notifications_enabled: true,
            updated_at: Utc::now(),
        }
    }
}

/// Client-supplied partial preference update.
///
/// All fields are optional. Only explicitly supplied fields are changed.
///
/// Security properties:
/// - no `user_id` field
/// - no `updated_at` field
/// - unknown fields are rejected
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPreferencesUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications_enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_system_english_and_notifications_enabled() {
        let user_id = Uuid::new_v4();
        let prefs = UserPreferences::defaults(user_id);

        assert_eq!(prefs.user_id, user_id);
        assert_eq!(prefs.theme, Theme::System);
        assert_eq!(prefs.language, Language::En);
        assert!(prefs.notifications_enabled);
    }

    #[test]
    fn theme_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Theme::System).unwrap(),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&Theme::Light).unwrap(),
            "\"light\""
        );
        assert_eq!(
            serde_json::to_string(&Theme::Dark).unwrap(),
            "\"dark\""
        );
    }

    #[test]
    fn language_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Language::En).unwrap(),
            "\"en\""
        );
    }

    #[test]
    fn update_accepts_partial_fields() {
        let json = r#"{"theme":"dark"}"#;

        let update: UserPreferencesUpdate =
            serde_json::from_str(json).unwrap();

        assert_eq!(update.theme, Some(Theme::Dark));
        assert_eq!(update.language, None);
        assert_eq!(update.notifications_enabled, None);
    }

    #[test]
    fn update_accepts_all_supported_fields() {
        let json = r#"{
            "theme":"light",
            "language":"en",
            "notifications_enabled":false
        }"#;

        let update: UserPreferencesUpdate =
            serde_json::from_str(json).unwrap();

        assert_eq!(update.theme, Some(Theme::Light));
        assert_eq!(update.language, Some(Language::En));
        assert_eq!(update.notifications_enabled, Some(false));
    }

    #[test]
    fn update_rejects_unknown_fields() {
        let json = r#"{
            "theme":"dark",
            "user_id":"00000000-0000-0000-0000-000000000000"
        }"#;

        let result =
            serde_json::from_str::<UserPreferencesUpdate>(json);

        assert!(result.is_err());
    }

    #[test]
    fn update_cannot_set_server_owned_timestamp() {
        let json = r#"{
            "theme":"dark",
            "updated_at":"2026-01-01T00:00:00Z"
        }"#;

        let result =
            serde_json::from_str::<UserPreferencesUpdate>(json);

        assert!(result.is_err());
    }

    #[test]
    fn empty_update_is_valid() {
        let update =
            serde_json::from_str::<UserPreferencesUpdate>("{}").unwrap();

        assert_eq!(update.theme, None);
        assert_eq!(update.language, None);
        assert_eq!(update.notifications_enabled, None);
    }
}
