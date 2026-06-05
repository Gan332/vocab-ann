use rusqlite::{Connection, Result, params};
use std::sync::Mutex;
use crate::models::*;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Database { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS banks (
                name TEXT PRIMARY KEY,
                count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bank_name TEXT NOT NULL,
                word TEXT NOT NULL,
                definition TEXT NOT NULL,
                is_starred INTEGER NOT NULL DEFAULT 0,
                wrong_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (bank_name) REFERENCES banks(name) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_words_bank ON words(bank_name);
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bank_name TEXT NOT NULL,
                mode TEXT NOT NULL,
                total INTEGER NOT NULL,
                remembered INTEGER NOT NULL,
                forgotten INTEGER NOT NULL,
                accuracy INTEGER NOT NULL,
                duration INTEGER NOT NULL,
                date INTEGER NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn get_all_banks(&self) -> Vec<Bank> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, count, created_at, updated_at FROM banks ORDER BY updated_at DESC").unwrap();
        stmt.query_map([], |row| {
            Ok(Bank { name: row.get(0)?, count: row.get(1)?, created_at: row.get(2)?, updated_at: row.get(3)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_bank(&self, name: &str) -> Option<Bank> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, count, created_at, updated_at FROM banks WHERE name = ?1").unwrap();
        stmt.query_row(params![name], |row| {
            Ok(Bank { name: row.get(0)?, count: row.get(1)?, created_at: row.get(2)?, updated_at: row.get(3)? })
        }).ok()
    }

    pub fn upsert_bank(&self, bank: &Bank) {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR REPLACE INTO banks (name, count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![bank.name, bank.count, bank.created_at, bank.updated_at]).unwrap();
    }

    pub fn delete_bank(&self, name: &str) {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM words WHERE bank_name = ?1", params![name]).unwrap();
        conn.execute("DELETE FROM banks WHERE name = ?1", params![name]).unwrap();
    }

    pub fn rename_bank(&self, old_name: &str, new_name: &str) {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE banks SET name = ?1, updated_at = ?2 WHERE name = ?3",
            params![new_name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64, old_name]).unwrap();
        conn.execute("UPDATE words SET bank_name = ?1 WHERE bank_name = ?2", params![new_name, old_name]).unwrap();
        conn.execute("UPDATE sessions SET bank_name = ?1 WHERE bank_name = ?2", params![new_name, old_name]).unwrap();
    }

    pub fn get_words_by_bank(&self, bank_name: &str) -> Vec<Word> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, bank_name, word, definition, is_starred, wrong_count FROM words WHERE bank_name = ?1").unwrap();
        stmt.query_map(params![bank_name], |row| {
            Ok(Word { id: row.get(0)?, bank_name: row.get(1)?, word: row.get(2)?, definition: row.get(3)?, is_starred: row.get::<_, i32>(4)? != 0, wrong_count: row.get(5)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_word_by_id(&self, id: i64) -> Option<Word> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, bank_name, word, definition, is_starred, wrong_count FROM words WHERE id = ?1").unwrap();
        stmt.query_row(params![id], |row| {
            Ok(Word { id: row.get(0)?, bank_name: row.get(1)?, word: row.get(2)?, definition: row.get(3)?, is_starred: row.get::<_, i32>(4)? != 0, wrong_count: row.get(5)? })
        }).ok()
    }

    pub fn search_words(&self, bank_name: &str, query: &str) -> Vec<Word> {
        let conn = self.conn.lock().unwrap();
        let q = format!("%{}%", query);
        let mut stmt = conn.prepare("SELECT id, bank_name, word, definition, is_starred, wrong_count FROM words WHERE bank_name = ?1 AND (word LIKE ?2 OR definition LIKE ?2)").unwrap();
        stmt.query_map(params![bank_name, q], |row| {
            Ok(Word { id: row.get(0)?, bank_name: row.get(1)?, word: row.get(2)?, definition: row.get(3)?, is_starred: row.get::<_, i32>(4)? != 0, wrong_count: row.get(5)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_all_starred_words(&self) -> Vec<Word> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, bank_name, word, definition, is_starred, wrong_count FROM words WHERE is_starred = 1").unwrap();
        stmt.query_map([], |row| {
            Ok(Word { id: row.get(0)?, bank_name: row.get(1)?, word: row.get(2)?, definition: row.get(3)?, is_starred: true, wrong_count: row.get(5)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_starred_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM words WHERE is_starred = 1", [], |row| row.get(0)).unwrap_or(0)
    }

    pub fn get_all_wrong_words(&self) -> Vec<Word> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, bank_name, word, definition, is_starred, wrong_count FROM words WHERE wrong_count > 0 ORDER BY wrong_count DESC").unwrap();
        stmt.query_map([], |row| {
            Ok(Word { id: row.get(0)?, bank_name: row.get(1)?, word: row.get(2)?, definition: row.get(3)?, is_starred: row.get::<_, i32>(4)? != 0, wrong_count: row.get(5)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_wrong_words_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM words WHERE wrong_count > 0", [], |row| row.get(0)).unwrap_or(0)
    }

    pub fn insert_words(&self, words: &[Word]) {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("INSERT OR REPLACE INTO words (bank_name, word, definition, is_starred, wrong_count) VALUES (?1, ?2, ?3, ?4, ?5)").unwrap();
        for w in words {
            stmt.execute(params![w.bank_name, w.word, w.definition, if w.is_starred { 1 } else { 0 }, w.wrong_count]).unwrap();
        }
    }

    pub fn delete_words_by_bank(&self, bank_name: &str) {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM words WHERE bank_name = ?1", params![bank_name]).unwrap();
    }

    pub fn update_word(&self, word: &Word) {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE words SET word = ?1, definition = ?2, is_starred = ?3, wrong_count = ?4 WHERE id = ?5",
            params![word.word, word.definition, if word.is_starred { 1 } else { 0 }, word.wrong_count, word.id]).unwrap();
    }

    pub fn insert_word(&self, word: &Word) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO words (bank_name, word, definition, is_starred, wrong_count) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![word.bank_name, word.word, word.definition, if word.is_starred { 1 } else { 0 }, word.wrong_count]).unwrap();
        conn.last_insert_rowid()
    }

    pub fn delete_word(&self, id: i64) {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM words WHERE id = ?1", params![id]).unwrap();
    }

    pub fn get_all_sessions(&self) -> Vec<Session> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, bank_name, mode, total, remembered, forgotten, accuracy, duration, date FROM sessions ORDER BY date DESC").unwrap();
        stmt.query_map([], |row| {
            Ok(Session { id: row.get(0)?, bank_name: row.get(1)?, mode: row.get(2)?, total: row.get(3)?, remembered: row.get(4)?, forgotten: row.get(5)?, accuracy: row.get(6)?, duration: row.get(7)?, date: row.get(8)? })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn insert_session(&self, session: &Session) {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO sessions (bank_name, mode, total, remembered, forgotten, accuracy, duration, date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![session.bank_name, session.mode, session.total, session.remembered, session.forgotten, session.accuracy, session.duration, session.date]).unwrap();
    }
}