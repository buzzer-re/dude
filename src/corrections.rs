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
        // Use WAL mode for better concurrent access, and full sync to avoid data loss
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
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

    /// Return the total number of learned corrections.
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM corrections", [], |row| row.get(0))
            .unwrap_or(0)
    }

    /// Return the confident correction for a typo (accepted 3+ times), if any.
    pub fn confident_correction(&self, typo: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT correction FROM corrections WHERE typo = ?1 AND count >= 3 ORDER BY count DESC LIMIT 1",
                params![typo],
                |row| row.get(0),
            )
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_corrections() -> Corrections {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS corrections (
                typo TEXT NOT NULL,
                correction TEXT NOT NULL,
                count INTEGER NOT NULL DEFAULT 1,
                last_used TEXT,
                PRIMARY KEY (typo, correction)
            )",
        )
        .unwrap();
        Corrections { conn }
    }

    #[test]
    fn test_record_and_count() {
        let db = in_memory_corrections();
        assert_eq!(db.count(), 0);
        db.record("gti", "git");
        assert_eq!(db.count(), 1);
        db.record("gti", "git");
        assert_eq!(db.count(), 1); // same pair, count incremented not duplicated
    }

    #[test]
    fn test_confident_correction_threshold() {
        let db = in_memory_corrections();
        db.record("gti", "git");
        db.record("gti", "git");
        // Only 2 times — not confident yet
        assert_eq!(db.confident_correction("gti"), None);
        db.record("gti", "git");
        // 3 times — now confident
        assert_eq!(db.confident_correction("gti"), Some("git".into()));
    }

    #[test]
    fn test_confident_returns_most_used() {
        let db = in_memory_corrections();
        // Record "gti" -> "git" 5 times
        for _ in 0..5 {
            db.record("gti", "git");
        }
        // Record "gti" -> "gzip" 3 times
        for _ in 0..3 {
            db.record("gti", "gzip");
        }
        // Should return "git" (higher count)
        assert_eq!(db.confident_correction("gti"), Some("git".into()));
    }

    #[test]
    fn test_no_correction_for_unknown() {
        let db = in_memory_corrections();
        db.record("gti", "git");
        assert_eq!(db.confident_correction("xyz"), None);
    }
}
