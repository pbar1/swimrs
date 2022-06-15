use std::str::FromStr;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

pub struct SqliteRequestDb {}

impl SqliteRequestDb {
    async fn new(connection_string: &str) -> Result<()> {
        let conn = SqliteConnectOptions::from_str(connection_string)?;

        todo!()
    }
}
