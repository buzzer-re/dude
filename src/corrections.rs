use crate::config::db_path;
use rusqlite::{params, Connection};

pub struct Corrections {
    conn: Connection,
}

impl Corrections {
    pub fn open() -> Result<Self, rusqlite::Error> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS corrections (
                typo TEXT NOT NULL,
                correction TEXT NOT NULL,
                count INTEGER NOT NULL DEFAULT 1,
                last_used TEXT,
                PRIMARY KEY (typo, correction)
            )",
        )?;
        Ok(Self { conn })
    }

    /// Record that the user accepted a correction.
    pub fn record(&self, typo: &str, correction: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let _ = self.conn.execute(
            "INSERT INTO corrections (typo, correction, count, last_used)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(typo, correction)
             DO UPDATE SET count = count + 1, last_used = ?3",
            params![typo, correction, now],
        );
    }

    /// Check if a typo has been corrected enough times to auto-suggest instantly.
    pub fn is_confident(&self, typo: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT correction FROM corrections WHERE typo = ?1 AND count >= 3 ORDER BY count DESC LIMIT 1",
                params![typo],
                |row| row.get(0),
            )
            .ok()
    }
}
