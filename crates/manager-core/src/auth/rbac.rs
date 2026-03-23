// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use crate::models::UserRole;

/// Check if a user role has permission for a given minimum role requirement.
pub fn check_permission(user_role: UserRole, required: UserRole) -> bool {
    user_role.has_permission(required)
}

/// Permission check result.
#[derive(Debug)]
pub enum PermissionError {
    /// User does not have the required role.
    InsufficientRole { required: UserRole, actual: UserRole },
    /// User does not have access to the requested node.
    NodeAccessDenied { node_id: String },
    /// User account has expired.
    AccountExpired,
    /// User account is disabled.
    AccountDisabled,
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientRole { required, actual } => {
                write!(f, "Requires role {required}, user has {actual}")
            }
            Self::NodeAccessDenied { node_id } => {
                write!(f, "Access denied to node {node_id}")
            }
            Self::AccountExpired => write!(f, "Account has expired"),
            Self::AccountDisabled => write!(f, "Account is disabled"),
        }
    }
}

impl std::error::Error for PermissionError {}
