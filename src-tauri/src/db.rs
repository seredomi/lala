use crate::models::*;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_db(db_path: &Path) -> Result<DbPool> {
    let conn = Connection::open(db_path).context("failed to open database")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            original_filename TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            file_id TEXT NOT NULL,
            parent_asset_id TEXT,
            asset_type TEXT NOT NULL,
            file_path TEXT NOT NULL,
            status TEXT NOT NULL,
            error_message TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(file_id) REFERENCES files(id),
            FOREIGN KEY(parent_asset_id) REFERENCES assets(id)
        )",
        [],
    )?;

    // index for faster queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_assets_status ON assets(status)",
        [],
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn create_file(pool: &DbPool, id: &str, original_filename: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT INTO files (id, original_filename, created_at) VALUES (?1, ?2, ?3)",
        params![id, original_filename, now],
    )?;

    Ok(())
}

pub fn create_asset(
    pool: &DbPool,
    id: &str,
    file_id: &str,
    parent_asset_id: Option<&str>,
    asset_type: AssetType,
    file_path: &str,
    status: ProcessingStatus,
) -> Result<()> {
    let conn = pool.lock().unwrap();
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT INTO assets (id, file_id, parent_asset_id, asset_type, file_path, status, error_message, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
        params![
            id,
            file_id,
            parent_asset_id,
            asset_type.to_string(),
            file_path,
            status.to_string(),
            now
        ],
    )?;

    Ok(())
}

pub fn update_asset_status(
    pool: &DbPool,
    asset_id: &str,
    status: ProcessingStatus,
    error_message: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().unwrap();

    conn.execute(
        "UPDATE assets SET status = ?1, error_message = ?2 WHERE id = ?3",
        params![status.to_string(), error_message, asset_id],
    )?;

    Ok(())
}

pub fn get_next_queued_asset(pool: &DbPool) -> Result<Option<Asset>> {
    let conn = pool.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, file_id, parent_asset_id, asset_type, file_path, status, error_message, created_at
         FROM assets WHERE status = 'queued' ORDER BY created_at ASC LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Asset {
            id: row.get(0)?,
            file_id: row.get(1)?,
            parent_asset_id: row.get(2)?,
            asset_type: AssetType::from_string(&row.get::<_, String>(3)?),
            file_path: row.get(4)?,
            status: ProcessingStatus::from_string(&row.get::<_, String>(5)?),
            error_message: row.get(6)?,
            created_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_assets_by_file(pool: &DbPool, file_id: &str) -> Result<Vec<Asset>> {
    let conn = pool.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, file_id, parent_asset_id, asset_type, file_path, status, error_message, created_at
         FROM assets WHERE file_id = ?1 ORDER BY created_at ASC",
    )?;

    let assets = stmt
        .query_map([file_id], |row| {
            Ok(Asset {
                id: row.get(0)?,
                file_id: row.get(1)?,
                parent_asset_id: row.get(2)?,
                asset_type: AssetType::from_string(&row.get::<_, String>(3)?),
                file_path: row.get(4)?,
                status: ProcessingStatus::from_string(&row.get::<_, String>(5)?),
                error_message: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(assets)
}

// pub fn get_asset_by_id(pool: &DbPool, asset_id: &str) -> Result<Option<Asset>> {
//     let conn = pool.lock().unwrap();
//
//     let mut stmt = conn.prepare(
//         "SELECT id, file_id, parent_asset_id, asset_type, file_path, status, error_message, created_at
//          FROM assets WHERE id = ?1",
//     )?;
//
//     let mut rows = stmt.query([asset_id])?;
//
//     if let Some(row) = rows.next()? {
//         Ok(Some(Asset {
//             id: row.get(0)?,
//             file_id: row.get(1)?,
//             parent_asset_id: row.get(2)?,
//             asset_type: AssetType::from_string(&row.get::<_, String>(3)?),
//             file_path: row.get(4)?,
//             status: ProcessingStatus::from_string(&row.get::<_, String>(5)?),
//             error_message: row.get(6)?,
//             created_at: row.get(7)?,
//         }))
//     } else {
//         Ok(None)
//     }
// }

pub fn get_all_files(pool: &DbPool) -> Result<Vec<FileRecord>> {
    let conn = pool.lock().unwrap();

    let mut stmt = conn
        .prepare("SELECT id, original_filename, created_at FROM files ORDER BY created_at DESC")?;

    let files = stmt
        .query_map([], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                original_filename: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(files)
}

// startup recovery: reset any processing jobs to queued
pub fn reset_interrupted_jobs(pool: &DbPool) -> Result<usize> {
    let conn = pool.lock().unwrap();

    let count = conn.execute(
        "UPDATE assets SET status = 'queued' WHERE status = 'processing'",
        [],
    )?;

    Ok(count)
}

pub fn delete_file_and_assets(pool: &DbPool, file_id: &str) -> Result<()> {
    let conn = pool.lock().unwrap();

    conn.execute("DELETE FROM assets WHERE file_id = ?1", [file_id])?;
    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])?;

    Ok(())
}

pub fn cancel_file_processing(pool: &DbPool, file_id: &str) -> Result<()> {
    let conn = pool.lock().unwrap();

    // delete queued assets
    conn.execute(
        "DELETE FROM assets WHERE file_id = ?1 AND status = 'queued'",
        [file_id],
    )?;

    // mark processing assets as cancelled
    conn.execute(
        "UPDATE assets SET status = 'cancelled' WHERE file_id = ?1 AND status = 'processing'",
        [file_id],
    )?;

    Ok(())
}

pub fn update_asset_parent(
    pool: &DbPool,
    asset_id: &str,
    parent_asset_id: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().unwrap();

    conn.execute(
        "UPDATE assets SET parent_asset_id = ?1 WHERE id = ?2",
        params![parent_asset_id, asset_id],
    )?;

    Ok(())
}
