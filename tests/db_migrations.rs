use vessel::db;

#[tokio::test]
async fn migrations_run_idempotent() {
    let db = libsql::Builder::new_local(":memory:")
        .build()
        .await
        .unwrap();
    let conn = db.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    // run twice — must not error
    db::schema::run_migrations(&conn).await.unwrap();
    let mut rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
            (),
        )
        .await
        .unwrap();
    let mut tables = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        tables.push(row.get::<String>(0).unwrap());
    }
    assert!(tables.contains(&"profiles".to_string()));
    assert!(tables.contains(&"projects".to_string()));
    assert!(tables.contains(&"generations".to_string()));
    assert!(tables.contains(&"generation_outputs".to_string()));
    assert!(tables.contains(&"revision_notes".to_string()));
    assert!(tables.contains(&"content_feedback".to_string()));
    assert!(tables.contains(&"github_tokens".to_string()));
    assert!(tables.contains(&"_migrations".to_string()));
}
