//! The shared PostgreSQL server of `tests/common/pg.rs`, asked for first. nextest starts this case
//! before every other one (`priority` 100 in `.config/nextest.toml`) and the PostgreSQL cases after
//! the long ones (`priority` 40), so the server starts while other cases run and no PostgreSQL case
//! waits on its start. `task test` pulls the image while the test binaries build.

use crate::common::pg;

/// The shared server answers a query in a database of this case's own.
#[test]
fn the_shared_server_answers() {
    if !pg::container_runtime("the shared PostgreSQL server") {
        return;
    }
    let db = pg::Database::create(|_| "SELECT 1".to_string());
    assert_eq!(db.psql("SELECT 41 + 1").trim(), "42");
}

/// `task test` pulls the image the server runs, and this case is the first one nextest starts.
#[test]
fn the_task_pulls_the_servers_image_and_nextest_asks_for_the_server_first() {
    let root = env!("CARGO_MANIFEST_DIR");
    let taskfile = std::fs::read_to_string(format!("{root}/Taskfile.yml")).unwrap();
    assert!(
        taskfile.contains(&format!("TEST_POSTGRES_IMAGE: {}\n", pg::IMAGE)),
        "Taskfile.yml pulls another image than {}",
        pg::IMAGE
    );
    let nextest = std::fs::read_to_string(format!("{root}/.config/nextest.toml")).unwrap();
    assert!(
        nextest.contains(
            "filter = 'test(=it::postgres_server::the_shared_server_answers)'\npriority = 100\n"
        ),
        "{nextest}"
    );
}
