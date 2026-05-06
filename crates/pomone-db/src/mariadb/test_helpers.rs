//! MariaDB integration test infrastructure.
//!
//! A single MariaDB container is started per test process via `OnceCell`
//! and shared across tests. Each test gets its own database (named with a
//! random suffix) so they're isolated.

use super::MariaDbRepository;
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::mariadb::Mariadb;
use tokio::sync::OnceCell;
use uuid::Uuid;

struct Shared {
    _container: ContainerAsync<Mariadb>,
    port: u16,
}

static SHARED: OnceCell<Arc<Shared>> = OnceCell::const_new();

async fn shared() -> Arc<Shared> {
    SHARED
        .get_or_init(|| async {
            let container = Mariadb::default()
                .start()
                .await
                .expect("failed to start MariaDB testcontainer");
            let port = container
                .get_host_port_ipv4(3306)
                .await
                .expect("failed to read MariaDB host port");
            Arc::new(Shared {
                _container: container,
                port,
            })
        })
        .await
        .clone()
}

/// Spin up a fresh, fully-migrated MariaDB-backed [`MariaDbRepository`] using
/// a unique database within the shared container.
pub(crate) async fn fresh_repo() -> MariaDbRepository {
    let shared = shared().await;
    let db_name = format!("pomone_test_{}", Uuid::new_v4().simple());

    // Connect as root with no password to the default 'test' DB to create
    // a unique DB for this test.
    let admin_url = format!("mysql://root@127.0.0.1:{}/test", shared.port);
    let admin_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("failed to connect as admin");
    sqlx::query(&format!("CREATE DATABASE `{db_name}`"))
        .execute(&admin_pool)
        .await
        .expect("failed to create test db");
    admin_pool.close().await;

    let url = format!("mysql://root@127.0.0.1:{}/{}", shared.port, db_name);
    MariaDbRepository::connect(&url)
        .await
        .expect("failed to connect to fresh test db")
}
