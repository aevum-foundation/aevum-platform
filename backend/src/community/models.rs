//! Community domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Public community profile owned by a platform user.
///
/// Username is immutable in Community v1.
/// `normalized_username` is used exclusively for uniqueness/index lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityProfile {
    pub user_id: Uuid,
    pub username: String,
    pub normalized_username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommunityProfile {
    pub fn new(
        user_id: Uuid,
        username: String,
        normalized_username: String,
        display_name: Option<String>,
        bio: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            username,
            normalized_username,
            display_name,
            bio,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunityRole {
    User,
    Moderator,
    Admin,
}

impl Default for CommunityRole {
    fn default() -> Self {
        Self::User
    }
}

impl CommunityRole {
    /// Stable string representation used for persistence and API contracts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
        }
    }

    /// Parse a role from its stable string representation.
    ///
    /// Returns `None` for unknown values — callers decide whether to fail
    /// or fall back to a default.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "moderator" => Some(Self::Moderator),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_as_str_is_stable() {
        assert_eq!(CommunityRole::User.as_str(), "user");
        assert_eq!(CommunityRole::Moderator.as_str(), "moderator");
        assert_eq!(CommunityRole::Admin.as_str(), "admin");
    }

    #[test]
    fn role_from_str_round_trips() {
        for role in [
            CommunityRole::User,
            CommunityRole::Moderator,
            CommunityRole::Admin,
        ] {
            assert_eq!(CommunityRole::from_str(role.as_str()), Some(role));
        }
    }

    #[test]
    fn role_from_str_rejects_unknown() {
        assert_eq!(CommunityRole::from_str("owner"), None);
        assert_eq!(CommunityRole::from_str(""), None);
        assert_eq!(CommunityRole::from_str("USER"), None);
    }

    #[test]
    fn role_default_is_user() {
        assert_eq!(CommunityRole::default(), CommunityRole::User);
    }
}
