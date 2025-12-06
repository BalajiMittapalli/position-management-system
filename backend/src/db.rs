use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    // Use PgPoolOptions for better connection pool configuration in SQLx 0.8.x
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
