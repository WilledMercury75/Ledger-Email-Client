use rusqlite::{Connection, params, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::message::*;

/// Thread-safe SQLite database wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create database at the given path
    pub fn open(data_dir: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("ledger.db");
        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_tables()?;
        Ok(db)
    }

    /// Create tables if they don't exist
    fn initialize_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                subject TEXT DEFAULT '',
                body TEXT DEFAULT '',
                timestamp INTEGER NOT NULL,
                delivery_method TEXT DEFAULT 'p2p',
                is_read INTEGER DEFAULT 0,
                folder TEXT DEFAULT 'inbox',
                signature TEXT,
                encrypted INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS contacts (
                ledger_id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL,
                encryption_public_key TEXT,
                display_name TEXT,
                gmail_address TEXT
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_folder ON messages(folder);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            CREATE INDEX IF NOT EXISTS idx_messages_to_id ON messages(to_id);"
        )?;

        // Insert default settings
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params!["delivery_mode", "auto"],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params!["tor_enabled", "false"],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params!["dht_ttl_hours", "72"],
        )?;

        Ok(())
    }

    // ── Messages ──

    /// Insert a message
    pub fn insert_message(&self, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO messages (id, from_id, to_id, subject, body, timestamp, delivery_method, is_read, folder, signature, encrypted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                msg.id,
                msg.from_id,
                msg.to_id,
                msg.subject,
                msg.body,
                msg.timestamp,
                msg.delivery_method.to_string(),
                msg.is_read as i32,
                msg.folder.to_string(),
                msg.signature,
                msg.encrypted as i32,
            ],
        )?;
        Ok(())
    }

    /// Get all messages, optionally filtered by folder
    pub fn get_messages(&self, folder: Option<&str>) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let (query, param_values): (&str, Vec<String>) = if let Some(f) = folder {
            ("SELECT id, from_id, to_id, subject, body, timestamp, delivery_method, is_read, folder, signature, encrypted
              FROM messages WHERE folder = ?1 ORDER BY timestamp DESC",
             vec![f.to_string()])
        } else {
            ("SELECT id, from_id, to_id, subject, body, timestamp, delivery_method, is_read, folder, signature, encrypted
              FROM messages ORDER BY timestamp DESC",
             vec![])
        };

        let mut stmt = conn.prepare(query)?;

        let rows = if param_values.is_empty() {
            stmt.query_map([], Self::row_to_message)?
        } else {
            stmt.query_map(params![param_values[0]], Self::row_to_message)?
        };

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Get a single message by ID
    pub fn get_message(&self, id: &str) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, from_id, to_id, subject, body, timestamp, delivery_method, is_read, folder, signature, encrypted
             FROM messages WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], Self::row_to_message)?;
        Ok(rows.next().transpose()?)
    }

    /// Delete a message
    pub fn delete_message(&self, id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let affected = conn.execute("DELETE FROM messages WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    /// Mark a message as read
    pub fn mark_read(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("UPDATE messages SET is_read = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_message(row: &rusqlite::Row<'_>) -> SqlResult<Message> {
        Ok(Message {
            id: row.get(0)?,
            from_id: row.get(1)?,
            to_id: row.get(2)?,
            subject: row.get(3)?,
            body: row.get(4)?,
            timestamp: row.get(5)?,
            delivery_method: DeliveryMethod::from_str(&row.get::<_, String>(6)?),
            is_read: row.get::<_, i32>(7)? != 0,
            folder: Folder::from_str(&row.get::<_, String>(8)?),
            signature: row.get(9)?,
            encrypted: row.get::<_, i32>(10)? != 0,
        })
    }

    // ── Contacts ──

    /// Upsert a contact
    pub fn upsert_contact(&self, contact: &Contact) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO contacts (ledger_id, public_key, display_name, gmail_address)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                contact.ledger_id,
                contact.public_key,
                contact.display_name,
                contact.gmail_address,
            ],
        )?;
        Ok(())
    }

    /// Get a contact by Ledger ID
    pub fn get_contact(&self, ledger_id: &str) -> Result<Option<Contact>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT ledger_id, public_key, display_name, gmail_address FROM contacts WHERE ledger_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![ledger_id], |row| {
            Ok(Contact {
                ledger_id: row.get(0)?,
                public_key: row.get(1)?,
                display_name: row.get(2)?,
                gmail_address: row.get(3)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Get all contacts
    pub fn get_contacts(&self) -> Result<Vec<Contact>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT ledger_id, public_key, display_name, gmail_address FROM contacts ORDER BY display_name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Contact {
                ledger_id: row.get(0)?,
                public_key: row.get(1)?,
                display_name: row.get(2)?,
                gmail_address: row.get(3)?,
            })
        })?;
        let mut contacts = Vec::new();
        for row in rows {
            contacts.push(row?);
        }
        Ok(contacts)
    }

    // ── Settings ──

    /// Get a setting
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        Ok(rows.next().transpose()?)
    }

    /// Set a setting
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get all settings as key-value pairs
    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut settings = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            settings.insert(k, v);
        }
        Ok(settings)
    }
}
