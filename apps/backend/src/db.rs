use anyhow::{Context, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use rustls::{ClientConfig, RootCertStore};
use tokio_postgres::config::Host;
use tokio_postgres::{Config as PgConfig, NoTls};
use tokio_postgres_rustls::MakeRustlsConnect;

const MIGRATION_LOCK_ID: i64 = 4_186_472_901;

fn required_schema_objects() -> &'static [&'static str] {
    &[
        "users",
        "devices",
        "alarms",
        "chat_messages",
        "pairing_sessions",
    ]
}

#[derive(Clone, Copy)]
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "baseline",
        sql: include_str!("../sql/001_init.sql"),
    },
    Migration {
        version: 2,
        name: "production_hardening",
        sql: include_str!("../sql/002_production_hardening.sql"),
    },
    Migration {
        version: 3,
        name: "security_limits",
        sql: include_str!("../sql/003_security_limits.sql"),
    },
    Migration {
        version: 4,
        name: "identity_security",
        sql: include_str!("../sql/004_identity_security.sql"),
    },
    Migration {
        version: 5,
        name: "delivery_backoff",
        sql: include_str!("../sql/005_delivery_backoff.sql"),
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationAction {
    Baseline(i64),
    Apply(i64),
}

fn migration_plan(existing_schema: bool, applied: &[i64]) -> Vec<MigrationAction> {
    MIGRATIONS
        .iter()
        .filter(|migration| !applied.contains(&migration.version))
        .map(|migration| {
            if migration.version == 1 && existing_schema {
                MigrationAction::Baseline(1)
            } else {
                MigrationAction::Apply(migration.version)
            }
        })
        .collect()
}

pub async fn create_pool(database_url: &str) -> Result<Pool> {
    let pg_config: PgConfig = database_url
        .parse()
        .context("DATABASE_URL must be a valid Postgres connection string")?;
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let remote = pg_config.get_hosts().iter().any(|host| match host {
        Host::Tcp(host) => {
            host != "localhost"
                && host
                    .parse::<std::net::IpAddr>()
                    .map_or(true, |address| !address.is_loopback())
        }
        Host::Unix(_) => false,
    });
    let manager = if remote {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = MakeRustlsConnect::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        );
        Manager::from_config(pg_config, tls, manager_config)
    } else {
        Manager::from_config(pg_config, NoTls, manager_config)
    };
    Pool::builder(manager)
        .max_size(16)
        .runtime(Runtime::Tokio1)
        .wait_timeout(Some(std::time::Duration::from_secs(5)))
        .build()
        .context("failed to create Postgres pool")
}

pub async fn migrate(pool: &Pool) -> Result<()> {
    let mut client = pool.get().await.context("failed to get Postgres client")?;
    let transaction = client
        .transaction()
        .await
        .context("failed to start migration transaction")?;
    transaction
        .query_one("SELECT pg_advisory_xact_lock($1)", &[&MIGRATION_LOCK_ID])
        .await
        .context("failed to acquire migration advisory lock")?;
    transaction
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                baselined BOOLEAN NOT NULL DEFAULT FALSE,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
        )
        .await
        .context("failed to initialize migration ledger")?;
    let users_exists: bool = transaction
        .query_one(
            "SELECT to_regclass(format('%I.users', current_schema())) IS NOT NULL AS exists",
            &[],
        )
        .await?
        .get("exists");
    let mut existing_schema = users_exists;
    if users_exists {
        let mut missing = Vec::new();
        for object in required_schema_objects() {
            let exists: bool = transaction
                .query_one(
                    "SELECT to_regclass(format('%I.%I',current_schema(),$1)) IS NOT NULL AS exists",
                    &[object],
                )
                .await?
                .get("exists");
            if !exists {
                missing.push(*object);
            }
        }
        anyhow::ensure!(
            missing.is_empty(),
            "refusing to baseline incomplete existing schema; missing: {}",
            missing.join(", ")
        );
        existing_schema = true;
    }
    let applied = transaction
        .query(
            "SELECT version FROM schema_migrations ORDER BY version",
            &[],
        )
        .await?
        .into_iter()
        .map(|row| row.get::<_, i64>("version"))
        .collect::<Vec<_>>();

    for action in migration_plan(existing_schema, &applied) {
        let version = match action {
            MigrationAction::Baseline(version) | MigrationAction::Apply(version) => version,
        };
        let migration = MIGRATIONS
            .iter()
            .find(|migration| migration.version == version)
            .context("migration plan referenced an unknown version")?;
        if matches!(action, MigrationAction::Apply(_)) {
            transaction
                .batch_execute(migration.sql)
                .await
                .with_context(|| {
                    format!(
                        "failed to apply migration {} {}",
                        migration.version, migration.name
                    )
                })?;
        }
        transaction
            .execute(
                "INSERT INTO schema_migrations (version,name,baselined) VALUES ($1,$2,$3)",
                &[
                    &migration.version,
                    &migration.name,
                    &matches!(action, MigrationAction::Baseline(_)),
                ],
            )
            .await?;
    }
    transaction
        .commit()
        .await
        .context("failed to commit schema migrations")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn migration_plan_baselines_existing_schema_without_replaying_baseline() {
        assert_eq!(
            migration_plan(true, &[]),
            vec![
                MigrationAction::Baseline(1),
                MigrationAction::Apply(2),
                MigrationAction::Apply(3),
                MigrationAction::Apply(4),
                MigrationAction::Apply(5),
            ]
        );
    }

    #[test]
    fn migration_plan_runs_baseline_then_hardening_for_fresh_database() {
        assert_eq!(
            migration_plan(false, &[]),
            vec![
                MigrationAction::Apply(1),
                MigrationAction::Apply(2),
                MigrationAction::Apply(3),
                MigrationAction::Apply(4),
                MigrationAction::Apply(5),
            ]
        );
    }

    #[test]
    fn migration_plan_is_empty_after_all_versions_apply() {
        assert!(migration_plan(true, &[1, 2, 3, 4, 5]).is_empty());
        assert_eq!(
            MIGRATIONS
                .iter()
                .map(|migration| migration.version)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[tokio::test]
    #[ignore = "requires ANTIROT_MIGRATION_TEST_DATABASE_URL or DATABASE_URL pointing to disposable PostgreSQL"]
    async fn migration_runner_serializes_baselines_and_backfills_idempotently() -> Result<()> {
        let database_url = std::env::var("ANTIROT_MIGRATION_TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .context("migration integration test requires a PostgreSQL URL")?;
        let admin_pool = create_pool(&database_url).await?;
        let admin = admin_pool.get().await?;

        let fresh_schema = format!("migration_fresh_{}", Uuid::new_v4().simple());
        let existing_schema = format!("migration_existing_{}", Uuid::new_v4().simple());
        admin
            .batch_execute(&format!(
                "CREATE SCHEMA {fresh_schema}; CREATE SCHEMA {existing_schema};"
            ))
            .await?;

        async fn schema_pool(database_url: &str, schema: &str) -> Result<Pool> {
            let mut config: PgConfig = database_url.parse()?;
            config.options(format!("-csearch_path={schema}"));
            let manager = Manager::from_config(
                config,
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            );
            Ok(Pool::builder(manager).max_size(4).build()?)
        }

        let fresh_pool = schema_pool(&database_url, &fresh_schema).await?;
        let (first, concurrent) = tokio::join!(migrate(&fresh_pool), migrate(&fresh_pool));
        first?;
        concurrent?;
        let fresh = fresh_pool.get().await?;
        let fresh_rows = fresh
            .query(
                "SELECT version,baselined,applied_at FROM schema_migrations ORDER BY version",
                &[],
            )
            .await?;
        assert_eq!(fresh_rows.len(), 2);
        assert!(!fresh_rows.iter().any(|row| row.get::<_, bool>("baselined")));
        let applied_at = fresh_rows
            .iter()
            .map(|row| row.get::<_, chrono::DateTime<chrono::Utc>>("applied_at"))
            .collect::<Vec<_>>();
        drop(fresh);
        migrate(&fresh_pool).await?;
        let fresh = fresh_pool.get().await?;
        let restarted_at = fresh
            .query(
                "SELECT applied_at FROM schema_migrations ORDER BY version",
                &[],
            )
            .await?
            .into_iter()
            .map(|row| row.get::<_, chrono::DateTime<chrono::Utc>>("applied_at"))
            .collect::<Vec<_>>();
        assert_eq!(restarted_at, applied_at, "restart must apply no migration");
        drop(fresh);

        let existing_pool = schema_pool(&database_url, &existing_schema).await?;
        let existing = existing_pool.get().await?;
        existing
            .batch_execute(
                "CREATE TABLE users (
                     id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, display_name TEXT,
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE TABLE devices (
                     device_id TEXT PRIMARY KEY, platform TEXT NOT NULL, app_version TEXT NOT NULL,
                     notification_capability TEXT NOT NULL, usage_capability TEXT NOT NULL,
                     push_provider TEXT, push_token TEXT,
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE TABLE alarms (
                     id TEXT PRIMARY KEY, device_id TEXT NOT NULL REFERENCES devices(device_id),
                     kind TEXT NOT NULL, severity TEXT NOT NULL, title TEXT NOT NULL,
                     message TEXT NOT NULL, fire_at TIMESTAMPTZ NOT NULL,
                     hidden_buffer_applied BOOLEAN NOT NULL DEFAULT false,
                     requires_acknowledgement BOOLEAN NOT NULL DEFAULT true,
                     expires_at TIMESTAMPTZ, status TEXT NOT NULL DEFAULT 'pending',
                     delivery_attempts INTEGER NOT NULL DEFAULT 0, last_delivered_at TIMESTAMPTZ,
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE TABLE user_memories (
                     user_id TEXT NOT NULL REFERENCES users(id), memory_key TEXT NOT NULL,
                     content TEXT NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     PRIMARY KEY (user_id,memory_key)
                 );
                 CREATE TABLE chat_messages (
                     id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id),
                     role TEXT NOT NULL, content TEXT, tool_calls JSONB, tool_call_id TEXT,
                     name TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE TABLE user_runtime_states (
                     user_id TEXT PRIMARY KEY REFERENCES users(id), state TEXT NOT NULL,
                     entered_at TIMESTAMPTZ NOT NULL DEFAULT now(), source_tool TEXT,
                     metadata JSONB NOT NULL DEFAULT '{}'::JSONB
                 );
                 CREATE TABLE user_state_metrics (
                     user_id TEXT PRIMARY KEY REFERENCES users(id),
                     usual_sleep_start_minute_utc INTEGER, average_sleep_minutes INTEGER,
                     average_sleep_quality NUMERIC, sleep_sample_count INTEGER NOT NULL DEFAULT 0,
                     last_sleep_started_at TIMESTAMPTZ, last_woke_at TIMESTAMPTZ,
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE TABLE memory_chunks (
                     id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id),
                     memory_key TEXT NOT NULL, chunk_index INTEGER NOT NULL, content TEXT NOT NULL,
                     content_hash TEXT NOT NULL, embedding JSONB, embedding_provider TEXT,
                     embedding_model TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     UNIQUE (user_id,memory_key,chunk_index,content_hash)
                 );
                 INSERT INTO users (id,email) VALUES ('legacy-user','legacy@test.invalid');
                 INSERT INTO user_memories (user_id,memory_key,content)
                 VALUES ('legacy-user','behavior','legacy canonical memory');",
            )
            .await?;
        drop(existing);

        let (first, concurrent) = tokio::join!(migrate(&existing_pool), migrate(&existing_pool));
        first?;
        concurrent?;
        let existing = existing_pool.get().await?;
        let ledger = existing
            .query(
                "SELECT version,baselined FROM schema_migrations ORDER BY version",
                &[],
            )
            .await?;
        assert_eq!(ledger.len(), 2);
        assert!(ledger[0].get::<_, bool>("baselined"));
        assert!(!ledger[1].get::<_, bool>("baselined"));
        assert_eq!(
            existing
                .query_one(
                    "SELECT COUNT(*)::BIGINT AS count FROM memory_index_jobs
                     WHERE user_id='legacy-user' AND memory_key='behavior'",
                    &[],
                )
                .await?
                .get::<_, i64>("count"),
            1
        );
        existing
            .execute(
                "UPDATE memory_index_jobs SET status='completed' WHERE user_id='legacy-user'",
                &[],
            )
            .await?;
        existing.batch_execute(MIGRATIONS[1].sql).await?;
        let idempotent = existing
            .query_one(
                "SELECT COUNT(*)::BIGINT AS count,MIN(status) AS status
                 FROM memory_index_jobs WHERE user_id='legacy-user' AND memory_key='behavior'",
                &[],
            )
            .await?;
        assert_eq!(idempotent.get::<_, i64>("count"), 1);
        assert_eq!(idempotent.get::<_, String>("status"), "completed");
        drop(existing);
        drop(fresh_pool);
        drop(existing_pool);
        admin
            .batch_execute(&format!(
                "DROP SCHEMA {fresh_schema} CASCADE; DROP SCHEMA {existing_schema} CASCADE;"
            ))
            .await?;
        Ok(())
    }
}
