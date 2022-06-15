use std::str::FromStr;

use anyhow::Result;
use sqlx::{
    query,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

pub struct SqliteRequestDb {
    pool: SqlitePool,
}

impl SqliteRequestDb {
    pub async fn new(db_url: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        Ok(Self { pool })
    }

    pub async fn ensure_schema(&self) -> Result<()> {
        query(
            r"
            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                state TEXT,
                num_results INTEGER,
                error TEXT,
                duration REAL
            ) WITHOUT ROWID
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn check_request_success(&self, req_id: &str) -> Result<bool> {
        let op = query("SELECT 1 FROM requests WHERE id = ? AND state = 'success'")
            .bind(req_id)
            .fetch_optional(&self.pool)
            .await?;
        match op {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    pub async fn upsert_request_success(
        &self,
        req_id: &str,
        num_results: u32,
        duration: f64,
    ) -> Result<()> {
        query(
            r"
            REPLACE INTO requests (id, state, num_results, error, duration)
            VALUES (?, 'success', ?, NULL, ?)
            ",
        )
        .bind(req_id)
        .bind(num_results)
        .bind(duration)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_request_error(
        &self,
        req_id: &str,
        error_text: &str,
        duration: f64,
    ) -> Result<()> {
        query(
            r"
            INSERT OR IGNORE INTO requests (id, state, num_results, error, duration)
            VALUES (?, 'error', NULL, ?, ?)
            ",
        )
        .bind(req_id)
        .bind(error_text)
        .bind(duration)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
