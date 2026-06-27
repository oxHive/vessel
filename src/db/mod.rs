// TODO
pub mod schema;
pub mod profiles;
pub mod projects;
pub mod generations;
pub mod feedback;

use anyhow::Result;
use libsql::Database;
use crate::config::VesselConfig;

pub type Db = std::sync::Arc<Database>;

pub async fn init(config: &VesselConfig) -> Result<Db> {
    let path = config.db_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let db = libsql::Builder::new_local(path).build().await?;
    schema::run_migrations(&db).await?;
    Ok(std::sync::Arc::new(db))
}
