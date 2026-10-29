use crate::models::Download;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS downloads (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            uploader TEXT,
            thumbnail TEXT,
            duration REAL,
            status TEXT NOT NULL,
            progress REAL NOT NULL DEFAULT 0,
            downloaded_bytes INTEGER NOT NULL DEFAULT 0,
            total_bytes INTEGER NOT NULL DEFAULT 0,
            speed REAL NOT NULL DEFAULT 0,
            eta INTEGER,
            filepath TEXT,
            error TEXT,
            preset TEXT NOT NULL,
            quality TEXT,
            playlist_items TEXT,
            created_at INTEGER NOT NULL,
            finished_at INTEGER,
            source TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads(created_at DESC);
         CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    Ok(conn)
}

pub fn upsert(conn: &Connection, d: &Download) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO downloads (
            id, url, title, uploader, thumbnail, duration, status, progress,
            downloaded_bytes, total_bytes, speed, eta, filepath, error, preset,
            quality, playlist_items, created_at, finished_at, source
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)
         ON CONFLICT(id) DO UPDATE SET
            url=excluded.url, title=excluded.title, uploader=excluded.uploader,
            thumbnail=excluded.thumbnail, duration=excluded.duration, status=excluded.status,
            progress=excluded.progress, downloaded_bytes=excluded.downloaded_bytes,
            total_bytes=excluded.total_bytes, speed=excluded.speed, eta=excluded.eta,
            filepath=excluded.filepath, error=excluded.error, preset=excluded.preset,
            quality=excluded.quality, playlist_items=excluded.playlist_items,
            finished_at=excluded.finished_at, source=excluded.source",
        params![
            d.id,
            d.url,
            d.title,
            d.uploader,
            d.thumbnail,
            d.duration,
            d.status,
            d.progress,
            d.downloaded_bytes,
            d.total_bytes,
            d.speed,
            d.eta,
            d.filepath,
            d.error,
            d.preset,
            d.quality,
            d.playlist_items,
            d.created_at,
            d.finished_at,
            d.source
        ],
    )?;
    Ok(())
}

pub fn load_all(conn: &Connection) -> rusqlite::Result<Vec<Download>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, uploader, thumbnail, duration, status, progress,
                downloaded_bytes, total_bytes, speed, eta, filepath, error, preset,
                quality, playlist_items, created_at, finished_at, source
         FROM downloads ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Download {
            id: r.get(0)?,
            url: r.get(1)?,
            title: r.get(2)?,
            uploader: r.get(3)?,
            thumbnail: r.get(4)?,
            duration: r.get(5)?,
            status: r.get(6)?,
            progress: r.get(7)?,
            downloaded_bytes: r.get(8)?,
            total_bytes: r.get(9)?,
            speed: r.get(10)?,
            eta: r.get(11)?,
            filepath: r.get(12)?,
            error: r.get(13)?,
            preset: r.get(14)?,
            quality: r.get(15)?,
            playlist_items: r.get(16)?,
            created_at: r.get(17)?,
            finished_at: r.get(18)?,
            source: r.get(19)?,
        })
    })?;
    rows.collect()
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
