// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User roles with hierarchical permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Viewer = 0,
    Operator = 1,
    Admin = 2,
    SuperAdmin = 3,
}

impl UserRole {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            "admin" => Some(Self::Admin),
            "super_admin" => Some(Self::SuperAdmin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Admin => "admin",
            Self::SuperAdmin => "super_admin",
        }
    }

    /// Check if this role has at least the given minimum role level.
    pub fn has_permission(&self, minimum: UserRole) -> bool {
        *self >= minimum
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A user account in the manager system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub is_temporary: bool,
    pub expires_at: Option<DateTime<Utc>>,
    /// JSON array of node IDs this user can access. None means all nodes.
    pub allowed_node_ids: Option<Vec<String>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl User {
    /// Check if a temporary user has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if user has access to a specific node.
    pub fn can_access_node(&self, node_id: &str) -> bool {
        match &self.allowed_node_ids {
            None => true, // null = all nodes
            Some(ids) => ids.iter().any(|id| id == node_id),
        }
    }
}

/// Request to create a new user.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub is_temporary: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub allowed_node_ids: Option<Vec<String>>,
}

/// Request to update an existing user.
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub is_temporary: Option<bool>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub allowed_node_ids: Option<Option<Vec<String>>>,
    pub is_active: Option<bool>,
    pub password: Option<String>,
}

/// User info returned to the frontend (no sensitive data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub is_temporary: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub allowed_node_ids: Option<Vec<String>>,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<User> for UserInfo {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            role: u.role,
            is_temporary: u.is_temporary,
            expires_at: u.expires_at,
            allowed_node_ids: u.allowed_node_ids,
            is_active: u.is_active,
            last_login_at: u.last_login_at,
        }
    }
}
