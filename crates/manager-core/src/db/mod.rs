pub mod users;
pub mod nodes;
pub mod events;
pub mod settings;
pub mod audit;

use sqlx::SqlitePool;

/// Initialize the database pool and run migrations.
pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}
