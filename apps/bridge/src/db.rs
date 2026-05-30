use anyhow::{Context, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{Config as PgConfig, NoTls};

pub async fn create_pool(database_url: &str) -> Result<Pool> {
    let pg_config: PgConfig = database_url
        .parse()
        .context("DATABASE_URL must be a valid Postgres connection string")?;
    let manager = Manager::from_config(
        pg_config,
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );
    Pool::builder(manager)
        .max_size(16)
        .build()
        .context("failed to create Postgres pool")
}

pub async fn migrate(pool: &Pool) -> Result<()> {
    let client = pool.get().await.context("failed to get Postgres client")?;
    client
        .batch_execute(include_str!("../sql/001_init.sql"))
        .await
        .context("failed to run bridge migrations")
}
