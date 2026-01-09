use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get the database path for the application
fn get_db_path(app: &AppHandle) -> PathBuf {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");
    
    std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
    app_data_dir.join("log15.db")
}

/// Initialize the SQLite database and create necessary tables
pub fn init_db(app: &AppHandle) -> Result<Connection> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path)?;
    
    // Create workblocks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS workblocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            start_time DATETIME NOT NULL,
            end_time DATETIME,
            duration_minutes INTEGER,
            status TEXT NOT NULL,
            is_archived BOOLEAN DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // Create intervals table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            workblock_id INTEGER NOT NULL,
            interval_number INTEGER NOT NULL,
            start_time DATETIME NOT NULL,
            end_time DATETIME,
            words TEXT,
            status TEXT NOT NULL,
            recorded_at DATETIME,
            FOREIGN KEY (workblock_id) REFERENCES workblocks(id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    // Create daily_archives table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily_archives (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL UNIQUE,
            total_workblocks INTEGER DEFAULT 0,
            total_minutes INTEGER DEFAULT 0,
            visualization_data TEXT,
            archived_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // Create indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workblocks_date ON workblocks(date)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workblocks_status ON workblocks(status)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_intervals_workblock_id ON intervals(workblock_id)",
        [],
    )?;
    
    Ok(conn)
}

/// Get a database connection
pub fn get_db_connection(app: &AppHandle) -> Result<Connection> {
    let db_path = get_db_path(app);
    Connection::open(&db_path)
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workblock {
    pub id: Option<i64>,
    pub date: String,  // YYYY-MM-DD format
    pub start_time: String,  // ISO 8601 format
    pub end_time: Option<String>,
    pub duration_minutes: Option<i32>,
    pub status: WorkblockStatus,
    pub is_archived: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum WorkblockStatus {
    Active,
    Completed,
    Cancelled,
}

impl WorkblockStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WorkblockStatus::Active => "active",
            WorkblockStatus::Completed => "completed",
            WorkblockStatus::Cancelled => "cancelled",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => WorkblockStatus::Active,
            "completed" => WorkblockStatus::Completed,
            "cancelled" => WorkblockStatus::Cancelled,
            _ => WorkblockStatus::Active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interval {
    pub id: Option<i64>,
    pub workblock_id: i64,
    pub interval_number: i32,
    pub start_time: String,  // ISO 8601 format
    pub end_time: Option<String>,
    pub words: Option<String>,
    pub status: IntervalStatus,
    pub recorded_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IntervalStatus {
    Pending,
    Recorded,
    AutoAway,
}

impl IntervalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            IntervalStatus::Pending => "pending",
            IntervalStatus::Recorded => "recorded",
            IntervalStatus::AutoAway => "auto_away",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => IntervalStatus::Pending,
            "recorded" => IntervalStatus::Recorded,
            "auto_away" => IntervalStatus::AutoAway,
            _ => IntervalStatus::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyArchive {
    pub id: Option<i64>,
    pub date: String,  // YYYY-MM-DD format
    pub total_workblocks: i32,
    pub total_minutes: i32,
    pub visualization_data: Option<String>,  // JSON string
    pub archived_at: Option<String>,
}

// ============================================================================
// Workblock Operations
// ============================================================================

/// Create a new workblock
pub fn create_workblock(app: &AppHandle, duration_minutes: i32) -> Result<Workblock> {
    let conn = get_db_connection(app)?;
    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let start_time = now.to_rfc3339();
    
    conn.execute(
        "INSERT INTO workblocks (date, start_time, duration_minutes, status, is_archived)
         VALUES (?1, ?2, ?3, ?4, 0)",
        params![date, start_time, duration_minutes, WorkblockStatus::Active.as_str()],
    )?;
    
    let id = conn.last_insert_rowid();
    
    Ok(Workblock {
        id: Some(id),
        date,
        start_time,
        end_time: None,
        duration_minutes: Some(duration_minutes),
        status: WorkblockStatus::Active,
        is_archived: false,
        created_at: Some(now.to_rfc3339()),
    })
}

/// Get the active workblock (if any)
pub fn get_active_workblock(app: &AppHandle) -> Result<Option<Workblock>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, duration_minutes, status, is_archived, created_at
         FROM workblocks
         WHERE status = 'active'
         ORDER BY start_time DESC
         LIMIT 1"
    )?;
    
    let workblock_result = stmt.query_row([], |row| {
        Ok(Workblock {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_minutes: row.get(4)?,
            status: WorkblockStatus::from_str(&row.get::<_, String>(5)?),
            is_archived: row.get(6)?,
            created_at: row.get(7)?,
        })
    });
    
    match workblock_result {
        Ok(workblock) => Ok(Some(workblock)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Complete a workblock
pub fn complete_workblock(app: &AppHandle, workblock_id: i64) -> Result<Workblock> {
    let conn = get_db_connection(app)?;
    let end_time = Local::now().to_rfc3339();
    
    // Calculate duration
    let workblock = get_workblock_by_id(app, workblock_id)?;
    let start_time = DateTime::parse_from_rfc3339(&workblock.start_time)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("Invalid start_time: {}", e), rusqlite::types::Type::Text))?;
    let end_time_dt = DateTime::parse_from_rfc3339(&end_time)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("Invalid end_time: {}", e), rusqlite::types::Type::Text))?;
    let duration = (end_time_dt - start_time).num_minutes() as i32;
    
    conn.execute(
        "UPDATE workblocks 
         SET end_time = ?1, duration_minutes = ?2, status = 'completed'
         WHERE id = ?3",
        params![end_time, duration, workblock_id],
    )?;
    
    get_workblock_by_id(app, workblock_id)
}

/// Cancel a workblock
pub fn cancel_workblock(app: &AppHandle, workblock_id: i64) -> Result<Workblock> {
    let conn = get_db_connection(app)?;
    let end_time = Local::now().to_rfc3339();
    
    // Calculate duration
    let workblock = get_workblock_by_id(app, workblock_id)?;
    let start_time = DateTime::parse_from_rfc3339(&workblock.start_time)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("Invalid start_time: {}", e), rusqlite::types::Type::Text))?;
    let end_time_dt = DateTime::parse_from_rfc3339(&end_time)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("Invalid end_time: {}", e), rusqlite::types::Type::Text))?;
    let duration = (end_time_dt - start_time).num_minutes() as i32;
    
    conn.execute(
        "UPDATE workblocks 
         SET end_time = ?1, duration_minutes = ?2, status = 'cancelled'
         WHERE id = ?3",
        params![end_time, duration, workblock_id],
    )?;
    
    get_workblock_by_id(app, workblock_id)
}

/// Get workblock by ID
pub fn get_workblock_by_id(app: &AppHandle, workblock_id: i64) -> Result<Workblock> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, duration_minutes, status, is_archived, created_at
         FROM workblocks
         WHERE id = ?1"
    )?;
    
    stmt.query_row(params![workblock_id], |row| {
        Ok(Workblock {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_minutes: row.get(4)?,
            status: WorkblockStatus::from_str(&row.get::<_, String>(5)?),
            is_archived: row.get(6)?,
            created_at: row.get(7)?,
        })
    })
}

/// Get all workblocks for a specific date
pub fn get_workblocks_by_date(app: &AppHandle, date: &str) -> Result<Vec<Workblock>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, duration_minutes, status, is_archived, created_at
         FROM workblocks
         WHERE date = ?1
         ORDER BY start_time ASC"
    )?;
    
    let workblock_iter = stmt.query_map(params![date], |row| {
        Ok(Workblock {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_minutes: row.get(4)?,
            status: WorkblockStatus::from_str(&row.get::<_, String>(5)?),
            is_archived: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    
    let mut workblocks = Vec::new();
    for workblock in workblock_iter {
        workblocks.push(workblock?);
    }
    Ok(workblocks)
}

// ============================================================================
// Interval Operations
// ============================================================================

/// Add an interval to a workblock
pub fn add_interval(app: &AppHandle, workblock_id: i64, interval_number: i32) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    let start_time = Local::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO intervals (workblock_id, interval_number, start_time, status)
         VALUES (?1, ?2, ?3, 'pending')",
        params![workblock_id, interval_number, start_time],
    )?;
    
    let id = conn.last_insert_rowid();
    
    Ok(Interval {
        id: Some(id),
        workblock_id,
        interval_number,
        start_time,
        end_time: None,
        words: None,
        status: IntervalStatus::Pending,
        recorded_at: None,
    })
}

/// Update interval with words
pub fn update_interval_words(
    app: &AppHandle,
    interval_id: i64,
    words: String,
    status: IntervalStatus,
) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    let recorded_at = Local::now().to_rfc3339();
    
    conn.execute(
        "UPDATE intervals 
         SET words = ?1, status = ?2, recorded_at = ?3, end_time = ?3
         WHERE id = ?4",
        params![words, status.as_str(), recorded_at, interval_id],
    )?;
    
    get_interval_by_id(app, interval_id)
}

/// Get interval by ID
pub fn get_interval_by_id(app: &AppHandle, interval_id: i64) -> Result<Interval> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, workblock_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals
         WHERE id = ?1"
    )?;
    
    stmt.query_row(params![interval_id], |row| {
        Ok(Interval {
            id: Some(row.get(0)?),
            workblock_id: row.get(1)?,
            interval_number: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            words: row.get(5)?,
            status: IntervalStatus::from_str(&row.get::<_, String>(6)?),
            recorded_at: row.get(7)?,
        })
    })
}

/// Get all intervals for a workblock
pub fn get_intervals_by_workblock(app: &AppHandle, workblock_id: i64) -> Result<Vec<Interval>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, workblock_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals
         WHERE workblock_id = ?1
         ORDER BY interval_number ASC"
    )?;
    
    let interval_iter = stmt.query_map(params![workblock_id], |row| {
        Ok(Interval {
            id: Some(row.get(0)?),
            workblock_id: row.get(1)?,
            interval_number: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            words: row.get(5)?,
            status: IntervalStatus::from_str(&row.get::<_, String>(6)?),
            recorded_at: row.get(7)?,
        })
    })?;
    
    let mut intervals = Vec::new();
    for interval in interval_iter {
        intervals.push(interval?);
    }
    Ok(intervals)
}

/// Get current interval for active workblock
pub fn get_current_interval(app: &AppHandle, workblock_id: i64) -> Result<Option<Interval>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, workblock_id, interval_number, start_time, end_time, words, status, recorded_at
         FROM intervals
         WHERE workblock_id = ?1 AND status = 'pending'
         ORDER BY interval_number DESC
         LIMIT 1"
    )?;
    
    let interval_result = stmt.query_row(params![workblock_id], |row| {
        Ok(Interval {
            id: Some(row.get(0)?),
            workblock_id: row.get(1)?,
            interval_number: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            words: row.get(5)?,
            status: IntervalStatus::from_str(&row.get::<_, String>(6)?),
            recorded_at: row.get(7)?,
        })
    });
    
    match interval_result {
        Ok(interval) => Ok(Some(interval)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Daily Operations
// ============================================================================

/// Get the date string for today
pub fn get_today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Check if we need to reset for a new day and archive previous day
pub fn check_and_reset_daily(app: &AppHandle) -> Result<Option<String>> {
    let today = get_today_date();
    let conn = get_db_connection(app)?;
    
    // Check if there are any workblocks from previous days that are still active
    let mut stmt = conn.prepare(
        "SELECT date FROM workblocks 
         WHERE status = 'active' AND date != ?1
         LIMIT 1"
    )?;
    
    let previous_date_result = stmt.query_row(params![today], |row| {
        Ok(row.get::<_, String>(0)?)
    });
    
    if let Ok(previous_date) = previous_date_result {
        // Archive the previous day
        archive_daily_data(app, &previous_date)?;
        
        // Mark any active workblocks from previous day as completed
        conn.execute(
            "UPDATE workblocks 
             SET status = 'completed', end_time = datetime('now')
             WHERE status = 'active' AND date != ?1",
            params![today],
        )?;
        
        return Ok(Some(previous_date));
    }
    
    // Check if we need to archive yesterday (if there are completed workblocks from yesterday)
    let yesterday = (Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM workblocks 
         WHERE date = ?1 AND is_archived = 0"
    )?;
    
    let count: i32 = stmt.query_row(params![yesterday], |row| row.get(0))?;
    
    if count > 0 {
        archive_daily_data(app, &yesterday)?;
        return Ok(Some(yesterday));
    }
    
    Ok(None)
}

/// Archive daily data and generate visualization JSON
pub fn archive_daily_data(app: &AppHandle, date: &str) -> Result<DailyArchive> {
    let conn = get_db_connection(app)?;
    
    // Get all workblocks for the date
    let workblocks = get_workblocks_by_date(app, date)?;
    
    if workblocks.is_empty() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("No workblocks found for date".to_string()),
        ));
    }
    
    // Mark all workblocks as archived
    conn.execute(
        "UPDATE workblocks SET is_archived = 1 WHERE date = ?1",
        params![date],
    )?;
    
    // Calculate totals
    let total_workblocks = workblocks.len() as i32;
    let total_minutes: i32 = workblocks
        .iter()
        .map(|wb| wb.duration_minutes.unwrap_or(0))
        .sum();
    
    // Generate visualization data
    let visualization_data = generate_daily_visualization_data(app, date)?;
    let visualization_json = serde_json::to_string(&visualization_data)
        .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("JSON serialization error: {}", e), rusqlite::types::Type::Text))?;
    
    // Insert or update daily archive
    conn.execute(
        "INSERT OR REPLACE INTO daily_archives (date, total_workblocks, total_minutes, visualization_data, archived_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![date, total_workblocks, total_minutes, visualization_json],
    )?;
    
    let id = conn.last_insert_rowid();
    
    Ok(DailyArchive {
        id: Some(id),
        date: date.to_string(),
        total_workblocks,
        total_minutes,
        visualization_data: Some(visualization_json),
        archived_at: Some(Local::now().to_rfc3339()),
    })
}

/// Get all archived dates
pub fn get_all_archived_dates(app: &AppHandle) -> Result<Vec<DailyArchive>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, total_workblocks, total_minutes, visualization_data, archived_at 
         FROM daily_archives 
         ORDER BY date DESC"
    )?;
    
    let archive_iter = stmt.query_map([], |row| {
        Ok(DailyArchive {
            id: row.get(0)?,
            date: row.get(1)?,
            total_workblocks: row.get(2)?,
            total_minutes: row.get(3)?,
            visualization_data: row.get(4)?,
            archived_at: row.get(5)?,
        })
    })?;
    
    let mut archives = Vec::new();
    for archive in archive_iter {
        archives.push(archive?);
    }
    
    Ok(archives)
}

/// Get archived day data
pub fn get_archived_day(app: &AppHandle, date: &str) -> Result<Option<DailyArchive>> {
    let conn = get_db_connection(app)?;
    let mut stmt = conn.prepare(
        "SELECT id, date, total_workblocks, total_minutes, visualization_data, archived_at
         FROM daily_archives
         WHERE date = ?1"
    )?;
    
    let archive_result = stmt.query_row(params![date], |row| {
        Ok(DailyArchive {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            total_workblocks: row.get(2)?,
            total_minutes: row.get(3)?,
            visualization_data: row.get(4)?,
            archived_at: row.get(5)?,
        })
    });
    
    match archive_result {
        Ok(archive) => Ok(Some(archive)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Visualization Data Generation
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineData {
    pub interval_number: i32,
    pub start_time: String,
    pub end_time: Option<String>,
    pub words: Option<String>,
    pub duration_minutes: i32,
    pub workblock_status: Option<String>, // "active", "completed", or "cancelled"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityData {
    pub words: String,
    pub total_minutes: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WordFrequency {
    pub word: String,
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkblockVisualization {
    pub id: i64,
    pub timeline_data: Vec<TimelineData>,
    pub activity_data: Vec<ActivityData>,
    pub word_frequency: Vec<WordFrequency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateTimelineData {
    pub workblock_id: i64,
    pub interval_number: i32,
    pub start_time: String,
    pub end_time: Option<String>,
    pub words: Option<String>,
    pub duration_minutes: i32,
    pub workblock_status: Option<String>, // "active", "completed", or "cancelled"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyAggregate {
    pub total_workblocks: i32,
    pub total_minutes: i32,
    pub timeline_data: Vec<AggregateTimelineData>,
    pub activity_data: Vec<ActivityData>,
    pub word_frequency: Vec<WordFrequency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyVisualizationData {
    pub workblocks: Vec<WorkblockVisualization>,
    pub daily_aggregate: DailyAggregate,
}

/// Generate visualization data for a single workblock
pub fn generate_workblock_visualization(
    app: &AppHandle,
    workblock_id: i64,
) -> Result<WorkblockVisualization> {
    let workblock = get_workblock_by_id(app, workblock_id)?;
    let mut intervals = get_intervals_by_workblock(app, workblock_id)?;
    let is_cancelled = workblock.status == WorkblockStatus::Cancelled;
    
    // If cancelled, filter out intervals that start after cancellation time
    // and identify the last interval to mark as cancelled
    let cancellation_end_time = if is_cancelled {
        workblock.end_time.as_ref().and_then(|et| {
            DateTime::parse_from_rfc3339(et).ok()
        })
    } else {
        None
    };
    
    if let Some(cancel_time) = cancellation_end_time {
        // Filter out intervals that start after cancellation
        intervals.retain(|interval| {
            if let Ok(start_time) = DateTime::parse_from_rfc3339(&interval.start_time) {
                start_time <= cancel_time
            } else {
                true // Keep if we can't parse (shouldn't happen)
            }
        });
    }
    
    // Find the last interval number to mark as cancelled (only for cancelled workblocks)
    let last_interval_number = if is_cancelled && !intervals.is_empty() {
        intervals.iter().map(|i| i.interval_number).max()
    } else {
        None
    };
    
    // Generate timeline data
    let timeline_data: Vec<TimelineData> = intervals
        .iter()
        .map(|interval| {
            let duration = if let Some(end_time) = &interval.end_time {
                let start = DateTime::parse_from_rfc3339(&interval.start_time).unwrap();
                let end = DateTime::parse_from_rfc3339(end_time).unwrap();
                (end - start).num_minutes() as i32
            } else {
                15 // Default 15 minutes if not ended
            };
            
            // Only mark as cancelled if this is the last interval and workblock is cancelled
            let status = if is_cancelled && last_interval_number == Some(interval.interval_number) {
                Some("cancelled".to_string())
            } else {
                None
            };
            
            TimelineData {
                interval_number: interval.interval_number,
                start_time: interval.start_time.clone(),
                end_time: interval.end_time.clone(),
                words: interval.words.clone(),
                duration_minutes: duration,
                workblock_status: status,
            }
        })
        .collect();
    
    // Generate activity data (group by words) - only from intervals that were actually used
    let mut activity_map: HashMap<String, i32> = HashMap::new();
    for interval in &intervals {
        if let Some(words) = &interval.words {
            let words_lower = words.to_lowercase().trim().to_string();
            if !words_lower.is_empty() {
                let duration = if let Some(end_time) = &interval.end_time {
                    let start = DateTime::parse_from_rfc3339(&interval.start_time).unwrap_or_default();
                    let end = DateTime::parse_from_rfc3339(end_time).unwrap_or_default();
                    (end - start).num_minutes() as i32
                } else {
                    15 // Default 15 minutes if not ended
                };
                *activity_map.entry(words_lower).or_insert(0) += duration;
            }
        }
    }
    
    let total_minutes: i32 = activity_map.values().sum();
    let activity_data: Vec<ActivityData> = activity_map
        .into_iter()
        .map(|(words, minutes)| {
            let percentage = if total_minutes > 0 {
                (minutes as f64 / total_minutes as f64) * 100.0
            } else {
                0.0
            };
            ActivityData {
                words,
                total_minutes: minutes,
                percentage,
            }
        })
        .collect();
    
    // Generate activity frequency (count entire phrase as one activity)
    let mut word_freq_map: HashMap<String, i32> = HashMap::new();
    for interval in &intervals {
        if let Some(words) = &interval.words {
            // Count entire phrase as one activity (not split by words)
            let words_lower = words.to_lowercase().trim().to_string();
            if !words_lower.is_empty() {
                *word_freq_map.entry(words_lower).or_insert(0) += 1;
            }
        }
    }
    
    let word_frequency: Vec<WordFrequency> = word_freq_map
        .into_iter()
        .map(|(word, count)| WordFrequency { word, count })
        .collect();
    
    Ok(WorkblockVisualization {
        id: workblock_id,
        timeline_data,
        activity_data,
        word_frequency,
    })
}

/// Generate daily aggregate visualization data
pub fn generate_daily_aggregate(app: &AppHandle, date: &str) -> Result<DailyAggregate> {
    let workblocks = get_workblocks_by_date(app, date)?;
    
    let mut all_timeline_data: Vec<AggregateTimelineData> = Vec::new();
    let mut activity_map: HashMap<String, i32> = HashMap::new();
    let mut word_freq_map: HashMap<String, i32> = HashMap::new();
    
    for workblock in &workblocks {
        let mut intervals = get_intervals_by_workblock(app, workblock.id.unwrap())?;
        let is_cancelled = workblock.status == WorkblockStatus::Cancelled;
        
        // If cancelled, filter out intervals that start after cancellation time
        let cancellation_end_time = if is_cancelled {
            workblock.end_time.as_ref().and_then(|et| {
                DateTime::parse_from_rfc3339(et).ok()
            })
        } else {
            None
        };
        
        if let Some(cancel_time) = cancellation_end_time {
            // Filter out intervals that start after cancellation
            intervals.retain(|interval| {
                if let Ok(start_time) = DateTime::parse_from_rfc3339(&interval.start_time) {
                    start_time <= cancel_time
                } else {
                    true // Keep if we can't parse (shouldn't happen)
                }
            });
        }
        
        // Find the last interval number to mark as cancelled (only for cancelled workblocks)
        let last_interval_number = if is_cancelled && !intervals.is_empty() {
            intervals.iter().map(|i| i.interval_number).max()
        } else {
            None
        };
        
        // Add to timeline
        for interval in &intervals {
            let duration = if let Some(end_time) = &interval.end_time {
                let start = DateTime::parse_from_rfc3339(&interval.start_time).unwrap();
                let end = DateTime::parse_from_rfc3339(end_time).unwrap();
                (end - start).num_minutes() as i32
            } else {
                15
            };
            
            // Only mark as cancelled if this is the last interval and workblock is cancelled
            let status = if is_cancelled && last_interval_number == Some(interval.interval_number) {
                Some("cancelled".to_string())
            } else {
                None
            };
            
            all_timeline_data.push(AggregateTimelineData {
                workblock_id: workblock.id.unwrap(),
                interval_number: interval.interval_number,
                start_time: interval.start_time.clone(),
                end_time: interval.end_time.clone(),
                words: interval.words.clone(),
                duration_minutes: duration,
                workblock_status: status,
            });
            
            // Add to activity map - only count duration that was actually used
            if let Some(words) = &interval.words {
                let words_lower = words.to_lowercase().trim().to_string();
                if !words_lower.is_empty() {
                    *activity_map.entry(words_lower).or_insert(0) += duration;
                }
            }
            
            // Add to activity frequency (count entire phrase as one activity)
            if let Some(words) = &interval.words {
                let words_lower = words.to_lowercase().trim().to_string();
                if !words_lower.is_empty() {
                    *word_freq_map.entry(words_lower).or_insert(0) += 1;
                }
            }
        }
    }
    
    // Sort timeline chronologically
    all_timeline_data.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    
    // Calculate activity percentages
    let total_minutes: i32 = activity_map.values().sum();
    let activity_data: Vec<ActivityData> = activity_map
        .into_iter()
        .map(|(words, minutes)| {
            let percentage = if total_minutes > 0 {
                (minutes as f64 / total_minutes as f64) * 100.0
            } else {
                0.0
            };
            ActivityData {
                words,
                total_minutes: minutes,
                percentage,
            }
        })
        .collect();
    
    let word_frequency: Vec<WordFrequency> = word_freq_map
        .into_iter()
        .map(|(word, count)| WordFrequency { word, count })
        .collect();
    
    let total_workblocks = workblocks.len() as i32;
    let aggregate_total_minutes: i32 = workblocks
        .iter()
        .map(|wb| wb.duration_minutes.unwrap_or(0))
        .sum();
    
    Ok(DailyAggregate {
        total_workblocks,
        total_minutes: aggregate_total_minutes,
        timeline_data: all_timeline_data,
        activity_data,
        word_frequency,
    })
}

/// Generate complete daily visualization data (workblocks + aggregate)
pub fn generate_daily_visualization_data(
    app: &AppHandle,
    date: &str,
) -> Result<DailyVisualizationData> {
    let workblocks = get_workblocks_by_date(app, date)?;
    
    let mut workblock_visualizations = Vec::new();
    for workblock in &workblocks {
        if let Some(id) = workblock.id {
            let viz = generate_workblock_visualization(app, id)?;
            workblock_visualizations.push(viz);
        }
    }
    
    let daily_aggregate = generate_daily_aggregate(app, date)?;
    
    Ok(DailyVisualizationData {
        workblocks: workblock_visualizations,
        daily_aggregate,
    })
}
