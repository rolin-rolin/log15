use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn get_db_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");
    std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
    app_data_dir.join("log15.db")
}

pub fn init_db<R: Runtime>(app: &AppHandle<R>) -> Result<Connection> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path)?;

    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            start_time DATETIME NOT NULL,
            end_time DATETIME,
            status TEXT NOT NULL DEFAULT 'active'
        )",
        [],
    )?;

    // Create intervals table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            interval_number INTEGER NOT NULL,
            start_time DATETIME NOT NULL,
            end_time DATETIME,
            words TEXT,
            status TEXT NOT NULL,
            recorded_at DATETIME,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create daily_archives table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily_archives (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL UNIQUE,
            total_sessions INTEGER DEFAULT 0,
            total_minutes INTEGER DEFAULT 0,
            visualization_data TEXT,
            archived_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Indexes
    conn.execute("CREATE INDEX IF NOT EXISTS idx_sessions_date ON sessions(date)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_intervals_session_id ON intervals(session_id)", [])?;

    Ok(conn)
}

pub fn get_db_connection<R: Runtime>(app: &AppHandle<R>) -> Result<Connection> {
    let db_path = get_db_path(app);
    Connection::open(&db_path)
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Stopped,
}

impl SessionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Stopped => "stopped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => SessionStatus::Active,
            _ => SessionStatus::Stopped,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: Option<i64>,
    pub date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub status: SessionStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IntervalStatus {
    Pending,
    Recorded,
    AutoAway,
    StoppedEarly,
}

impl IntervalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            IntervalStatus::Pending => "pending",
            IntervalStatus::Recorded => "recorded",
            IntervalStatus::AutoAway => "auto_away",
            IntervalStatus::StoppedEarly => "stopped_early",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => IntervalStatus::Pending,
            "recorded" => IntervalStatus::Recorded,
            "auto_away" => IntervalStatus::AutoAway,
            "stopped_early" => IntervalStatus::StoppedEarly,
            _ => IntervalStatus::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interval {
    pub id: Option<i64>,
    pub session_id: i64,
    pub interval_number: i32,
    pub start_time: String,
    pub end_time: Option<String>,
    pub words: Option<String>,
    pub status: IntervalStatus,
    pub recorded_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyArchive {
    pub id: Option<i64>,
    pub date: String,
    pub total_sessions: i32,
    pub total_minutes: i32,
    pub visualization_data: Option<String>,
    pub archived_at: Option<String>,
}

// ============================================================================
// Session Operations
// ============================================================================

pub fn create_session<R: Runtime>(app: &AppHandle<R>) -> Result<Session> {
    let conn = get_db_connection(app)?;
    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let start_time = now.to_rfc3339();

    conn.execute(
        "INSERT INTO sessions (date, start_time, status) VALUES (?1, ?2, 'active')",
        params![date, start_time],
    )?;

    let id = conn.last_insert_rowid();
    Ok(Session { id: Some(id), date, start_time, end_time: None, status: SessionStatus::Active })
}

pub fn get_active_session<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Session>> {
    let conn = get_db_connection(app)?;
    let result = conn.query_row(
        "SELECT id, date, start_time, end_time, status FROM sessions
         WHERE status = 'active' ORDER BY start_time DESC LIMIT 1",
        [],
        |r| Ok(Session {
            id: Some(r.get(0)?),
            date: r.get(1)?,
            start_time: r.get(2)?,
            end_time: r.get(3)?,
            status: SessionStatus::from_str(&r.get::<_, String>(4)?),
        }),
    );

    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_session_by_id<R: Runtime>(app: &AppHandle<R>, session_id: i64) -> Result<Session> {
    let conn = get_db_connection(app)?;
    conn.query_row(
        "SELECT id, date, start_time, end_time, status FROM sessions WHERE id = ?1",
        params![session_id],
        |r| Ok(Session {
            id: Some(r.get(0)?),
            date: r.get(1)?,
            start_time: r.get(2)?,
            end_time: r.get(3)?,
            status: SessionStatus::from_str(&r.get::<_, String>(4)?),
        }),
    )
}

pub fn stop_session<R: Runtime>(app: &AppHandle<R>, session_id: i64) -> Result<Session> {
    let conn = get_db_connection(app)?;
    let end_time = Local::now().to_rfc3339();

    // Mark any in-progress pending intervals as stopped_early with the stop time
    conn.execute(
        "UPDATE intervals SET end_time = ?1, status = 'stopped_early' WHERE session_id = ?2 AND status = 'pending'",
        params![end_time, session_id],
    )?;

    conn.execute(
        "UPDATE sessions SET end_time = ?1, status = 'stopped' WHERE id = ?2",
        params![end_time, session_id],
    )?;

    get_session_by_id(app, session_id)
}

pub fn set_interval_end_time<R: Runtime>(app: &AppHandle<R>, interval_id: i64, end_time: &str) -> Result<()> {
    let conn = get_db_connection(app)?;
    conn.execute(
        "UPDATE intervals SET end_time = ?1 WHERE id = ?2",
        params![end_time, interval_id],
    )?;
    Ok(())
}

pub fn get_sessions_by_date<R: Runtime>(app: &AppHandle<R>, date: &str) -> Result<Vec<Session>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, status FROM sessions
         WHERE date = ?1 ORDER BY start_time ASC",
    )?;

    let iter = stmt.query_map(params![date], |r| {
        Ok(Session {
            id: Some(r.get(0)?),
            date: r.get(1)?,
            start_time: r.get(2)?,
            end_time: r.get(3)?,
            status: SessionStatus::from_str(&r.get::<_, String>(4)?),
        })
    })?;

    let mut sessions = Vec::new();
    for s in iter { sessions.push(s?); }
    Ok(sessions)
}

// ============================================================================
// Interval Operations
// ============================================================================

pub fn add_interval<R: Runtime>(app: &AppHandle<R>, session_id: i64, interval_number: i32) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    let start_time = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO intervals (session_id, interval_number, start_time, status)
         VALUES (?1, ?2, ?3, 'pending')",
        params![session_id, interval_number, start_time],
    )?;

    let id = conn.last_insert_rowid();
    Ok(Interval {
        id: Some(id),
        session_id,
        interval_number,
        start_time,
        end_time: None,
        words: None,
        status: IntervalStatus::Pending,
        recorded_at: None,
    })
}

pub fn update_interval_words<R: Runtime>(
    app: &AppHandle<R>,
    interval_id: i64,
    words: String,
    status: IntervalStatus,
) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    let recorded_at = Local::now().to_rfc3339();

    conn.execute(
        "UPDATE intervals SET words = ?1, status = ?2, recorded_at = ?3 WHERE id = ?4",
        params![words, status.as_str(), recorded_at, interval_id],
    )?;

    get_interval_by_id(app, interval_id)
}

pub fn get_interval_by_id<R: Runtime>(app: &AppHandle<R>, interval_id: i64) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    conn.query_row(
        "SELECT id, session_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals WHERE id = ?1",
        params![interval_id],
        |r| Ok(Interval {
            id: Some(r.get(0)?),
            session_id: r.get(1)?,
            interval_number: r.get(2)?,
            start_time: r.get(3)?,
            end_time: r.get(4)?,
            words: r.get(5)?,
            status: IntervalStatus::from_str(&r.get::<_, String>(6)?),
            recorded_at: r.get(7)?,
        }),
    )
}

pub fn get_intervals_by_session<R: Runtime>(app: &AppHandle<R>, session_id: i64) -> Result<Vec<Interval>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, session_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals WHERE session_id = ?1 ORDER BY interval_number ASC",
    )?;

    let iter = stmt.query_map(params![session_id], |r| {
        Ok(Interval {
            id: Some(r.get(0)?),
            session_id: r.get(1)?,
            interval_number: r.get(2)?,
            start_time: r.get(3)?,
            end_time: r.get(4)?,
            words: r.get(5)?,
            status: IntervalStatus::from_str(&r.get::<_, String>(6)?),
            recorded_at: r.get(7)?,
        })
    })?;

    let mut intervals = Vec::new();
    for i in iter { intervals.push(i?); }
    Ok(intervals)
}

pub fn get_current_interval<R: Runtime>(app: &AppHandle<R>, session_id: i64) -> Result<Option<Interval>> {
    let conn = get_db_connection(app)?;
    let result = conn.query_row(
        "SELECT id, session_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals WHERE session_id = ?1 AND status = 'pending'
         ORDER BY interval_number DESC LIMIT 1",
        params![session_id],
        |r| Ok(Interval {
            id: Some(r.get(0)?),
            session_id: r.get(1)?,
            interval_number: r.get(2)?,
            start_time: r.get(3)?,
            end_time: r.get(4)?,
            words: r.get(5)?,
            status: IntervalStatus::from_str(&r.get::<_, String>(6)?),
            recorded_at: r.get(7)?,
        }),
    );

    match result {
        Ok(i) => Ok(Some(i)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Daily / Archive Operations
// ============================================================================

pub fn get_today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn check_and_reset_daily<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>> {
    let today = get_today_date();
    let conn = get_db_connection(app)?;

    // Archive any past days where all sessions are stopped (never auto-stop active sessions)
    let mut stmt = conn.prepare(
        "SELECT DISTINCT date FROM sessions
         WHERE date != ?1 AND date < ?1
           AND date NOT IN (SELECT date FROM daily_archives)
           AND date NOT IN (SELECT DISTINCT date FROM sessions WHERE status = 'active')
         ORDER BY date DESC",
    )?;

    let unarchived: Vec<String> = stmt
        .query_map(params![today], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(date) = unarchived.into_iter().next() {
        archive_daily_data(app, &date)?;
        return Ok(Some(date));
    }

    Ok(None)
}

pub fn archive_all_unarchived_dates<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<String>> {
    let today = get_today_date();
    let conn = get_db_connection(app)?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT date FROM sessions
         WHERE date != ?1 AND date < ?1
           AND date NOT IN (SELECT date FROM daily_archives)
           AND date NOT IN (SELECT DISTINCT date FROM sessions WHERE status = 'active')
         ORDER BY date DESC",
    )?;

    let dates: Vec<String> = stmt
        .query_map(params![today], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut archived = Vec::new();
    for date in &dates {
        if archive_daily_data(app, date).is_ok() {
            archived.push(date.clone());
        }
    }
    Ok(archived)
}

pub fn archive_daily_data<R: Runtime>(app: &AppHandle<R>, date: &str) -> Result<DailyArchive> {
    let conn = get_db_connection(app)?;
    let sessions = get_sessions_by_date(app, date)?;

    if sessions.is_empty() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("No sessions found for date".to_string()),
        ));
    }

    let viz_data = generate_daily_visualization_data(app, date)?;
    let viz_json = serde_json::to_string(&viz_data)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, e.to_string(), rusqlite::types::Type::Text))?;

    let total_sessions = sessions.len() as i32;
    let total_minutes = viz_data.total_minutes;

    conn.execute(
        "INSERT OR REPLACE INTO daily_archives (date, total_sessions, total_minutes, visualization_data, archived_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![date, total_sessions, total_minutes, viz_json],
    )?;

    let id = conn.last_insert_rowid();
    Ok(DailyArchive {
        id: Some(id),
        date: date.to_string(),
        total_sessions,
        total_minutes,
        visualization_data: Some(viz_json),
        archived_at: Some(Local::now().to_rfc3339()),
    })
}

pub fn get_archived_day<R: Runtime>(app: &AppHandle<R>, date: &str) -> Result<Option<DailyArchive>> {
    let conn = get_db_connection(app)?;
    let result = conn.query_row(
        "SELECT id, date, COALESCE(total_sessions, 0), total_minutes, visualization_data, archived_at
         FROM daily_archives WHERE date = ?1",
        params![date],
        |r| Ok(DailyArchive {
            id: Some(r.get(0)?),
            date: r.get(1)?,
            total_sessions: r.get(2)?,
            total_minutes: r.get(3)?,
            visualization_data: r.get(4)?,
            archived_at: r.get(5)?,
        }),
    );

    match result {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_all_archived_dates<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<DailyArchive>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, COALESCE(total_sessions, 0), total_minutes, visualization_data, archived_at
         FROM daily_archives ORDER BY date DESC",
    )?;

    let iter = stmt.query_map([], |r| {
        Ok(DailyArchive {
            id: r.get(0)?,
            date: r.get(1)?,
            total_sessions: r.get(2)?,
            total_minutes: r.get(3)?,
            visualization_data: r.get(4)?,
            archived_at: r.get(5)?,
        })
    })?;

    let mut archives = Vec::new();
    for a in iter { archives.push(a?); }
    Ok(archives)
}

pub fn delete_day<R: Runtime>(app: &AppHandle<R>, date: &str) -> Result<()> {
    let conn = get_db_connection(app)?;
    conn.execute("DELETE FROM daily_archives WHERE date = ?1", params![date])?;
    conn.execute("DELETE FROM sessions WHERE date = ?1", params![date])?;
    Ok(())
}

// ============================================================================
// Visualization Data
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineData {
    pub interval_number: i32,
    pub start_time: String,
    pub end_time: Option<String>,
    pub words: Option<String>,
    pub duration_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityData {
    pub words: String,
    pub total_minutes: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyVisualizationData {
    pub total_sessions: i32,
    pub total_minutes: i32,
    pub words_entered: i32,
    pub timeline_data: Vec<TimelineData>,
    pub activity_data: Vec<ActivityData>,
}

pub fn generate_daily_visualization_data<R: Runtime>(app: &AppHandle<R>, date: &str) -> Result<DailyVisualizationData> {
    let sessions = get_sessions_by_date(app, date)?;

    let mut all_intervals: Vec<Interval> = Vec::new();
    for session in &sessions {
        if let Some(id) = session.id {
            all_intervals.extend(get_intervals_by_session(app, id)?);
        }
    }

    all_intervals.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    // Only include full intervals (recorded or auto_away)
    let timeline_data: Vec<TimelineData> = all_intervals
        .iter()
        .filter(|i| i.status == IntervalStatus::Recorded || i.status == IntervalStatus::AutoAway)
        .map(|i| TimelineData {
            interval_number: i.interval_number,
            start_time: i.start_time.clone(),
            end_time: i.end_time.clone(),
            words: i.words.clone(),
            duration_minutes: 15,
        })
        .collect();

    let total_minutes = (timeline_data.len() as i32) * 15;

    let mut activity_map: HashMap<String, i32> = HashMap::new();
    for td in &timeline_data {
        if let Some(words) = &td.words {
            let key = words.to_lowercase().trim().to_string();
            if !key.is_empty() {
                *activity_map.entry(key).or_insert(0) += 15;
            }
        }
    }

    let mut activity_data: Vec<ActivityData> = activity_map
        .into_iter()
        .map(|(words, minutes)| ActivityData {
            percentage: if total_minutes > 0 {
                (minutes as f64 / total_minutes as f64) * 100.0
            } else {
                0.0
            },
            words,
            total_minutes: minutes,
        })
        .collect();

    activity_data.sort_by(|a, b| b.total_minutes.cmp(&a.total_minutes));

    let words_entered = timeline_data.iter()
        .filter(|t| t.words.as_ref().map_or(false, |w| !w.trim().is_empty()))
        .count() as i32;

    Ok(DailyVisualizationData {
        total_sessions: sessions.len() as i32,
        total_minutes,
        words_entered,
        timeline_data,
        activity_data,
    })
}
