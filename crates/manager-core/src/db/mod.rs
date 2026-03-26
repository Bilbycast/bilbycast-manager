// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

pub mod users;
pub mod nodes;
pub mod events;
pub mod settings;
pub mod audit;
pub mod tunnels;
pub mod sessions;
pub mod topology_positions;
pub mod ui_preferences;

use sqlx::SqlitePool;

/// Initialize the database pool and run migrations.
pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}
