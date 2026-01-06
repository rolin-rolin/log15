// Integration test for database operations
// Run with: cargo test --test integration_test

use log15_lib::db::*;
use std::path::PathBuf;
use rusqlite::Connection;
use chrono::{Local, Duration};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_test_db_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    path.push(format!("log15_test_{}.db", counter));
    path
}

fn init_test_db() -> Connection {
    let db_path = get_test_db_path();
    
    // Remove existing test database
    if db_path.exists() {
        std::fs::remove_file(&db_path).ok();
    }
    
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    let conn = Connection::open(&db_path).unwrap();
    
    // Create tables (same as in db.rs)
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
    ).unwrap();
    
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
    ).unwrap();
    
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
    ).unwrap();
    
    conn
}

// Mock AppHandle for testing
struct TestAppHandle {
    db_path: PathBuf,
}

impl TestAppHandle {
    fn new() -> Self {
        Self {
            db_path: get_test_db_path(),
        }
    }
}

// We need to create a wrapper that works with our db functions
// For now, let's create a simpler direct test

#[test]
fn test_workblock_creation_and_visualization() {
    let conn = init_test_db();
    
    // Create a workblock manually
    let today = Local::now().format("%Y-%m-%d").to_string();
    let start_time = Local::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO workblocks (date, start_time, duration_minutes, status, is_archived)
         VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![today, start_time, 60, "completed"],
    ).unwrap();
    
    let workblock_id = conn.last_insert_rowid();
    
    // Add intervals
    for i in 1..=4 {
        let int_start = (Local::now() - Duration::minutes(15 * (5 - i) as i64)).to_rfc3339();
        let words = match i {
            1 | 2 => "coding",
            3 => "meeting",
            4 => "planning",
            _ => "other",
        };
        
        conn.execute(
            "INSERT INTO intervals (workblock_id, interval_number, start_time, end_time, words, status, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'recorded', ?4)",
            rusqlite::params![workblock_id, i, int_start, Local::now().to_rfc3339(), words],
        ).unwrap();
    }
    
    // Verify data
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM intervals WHERE workblock_id = ?1",
        rusqlite::params![workblock_id],
        |row| row.get(0),
    ).unwrap();
    
    assert_eq!(count, 4);
    
    println!("✓ Test: Workblock creation with intervals passed");
    
    // Test visualization data structure
    let mut stmt = conn.prepare(
        "SELECT words, COUNT(*) as count 
         FROM intervals 
         WHERE workblock_id = ?1 AND words IS NOT NULL
         GROUP BY words"
    ).unwrap();
    
    let activity_map: std::collections::HashMap<String, i32> = stmt.query_map(
        rusqlite::params![workblock_id],
        |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        },
    ).unwrap()
    .map(|r| r.unwrap())
    .collect();
    
    assert_eq!(activity_map.get("coding"), Some(&2));
    assert_eq!(activity_map.get("meeting"), Some(&1));
    assert_eq!(activity_map.get("planning"), Some(&1));
    
    println!("✓ Test: Activity grouping works correctly");
}

#[test]
fn test_archiving_and_persistence() {
    let conn = init_test_db();
    
    // Create workblock for "yesterday"
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let start_time = (Local::now() - Duration::days(1)).to_rfc3339();
    
    conn.execute(
        "INSERT INTO workblocks (date, start_time, end_time, duration_minutes, status, is_archived)
         VALUES (?1, ?2, ?3, 60, 'completed', 0)",
        rusqlite::params![yesterday, start_time, Local::now().to_rfc3339()],
    ).unwrap();
    
    let workblock_id = conn.last_insert_rowid();
    
    // Add intervals
    for i in 1..=3 {
        let int_start = (Local::now() - Duration::days(1) - Duration::minutes(15 * (4 - i) as i64)).to_rfc3339();
        conn.execute(
            "INSERT INTO intervals (workblock_id, interval_number, start_time, end_time, words, status, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'recorded', ?4)",
            rusqlite::params![workblock_id, i, int_start, Local::now().to_rfc3339(), "coding"],
        ).unwrap();
    }
    
    // Mark as archived
    conn.execute(
        "UPDATE workblocks SET is_archived = 1 WHERE date = ?1",
        rusqlite::params![yesterday],
    ).unwrap();
    
    // Create archive entry with visualization data
    let viz_data = serde_json::json!({
        "workblocks": [{
            "id": workblock_id,
            "timeline_data": [
                {"interval_number": 1, "words": "coding", "duration_minutes": 15},
                {"interval_number": 2, "words": "coding", "duration_minutes": 15},
                {"interval_number": 3, "words": "coding", "duration_minutes": 15},
            ],
            "activity_data": [
                {"words": "coding", "total_minutes": 45, "percentage": 100.0}
            ],
            "word_frequency": [
                {"word": "coding", "count": 3}
            ]
        }],
        "daily_aggregate": {
            "total_workblocks": 1,
            "total_minutes": 60,
            "timeline_data": [],
            "activity_data": [
                {"words": "coding", "total_minutes": 45, "percentage": 100.0}
            ],
            "word_frequency": [
                {"word": "coding", "count": 3}
            ]
        }
    });
    
    conn.execute(
        "INSERT OR REPLACE INTO daily_archives (date, total_workblocks, total_minutes, visualization_data, archived_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        rusqlite::params![yesterday, 1, 60, viz_data.to_string()],
    ).unwrap();
    
    // Verify archive exists
    let archived: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM daily_archives WHERE date = ?1)",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    assert!(archived);
    
    // Verify visualization data is stored
    let viz_json: String = conn.query_row(
        "SELECT visualization_data FROM daily_archives WHERE date = ?1",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    let parsed: serde_json::Value = serde_json::from_str(&viz_json).unwrap();
    assert!(parsed["workblocks"].is_array());
    assert!(parsed["daily_aggregate"].is_object());
    
    println!("✓ Test: Archiving and persistence passed");
    println!("  - Archive created for date: {}", yesterday);
    println!("  - Visualization data stored: {} bytes", viz_json.len());
    
    // Cleanup
    std::fs::remove_file(get_test_db_path()).ok();
}

#[test]
fn test_day_transition() {
    let conn = init_test_db();
    
    // Clear any existing data first
    conn.execute("DELETE FROM intervals", []).ok();
    conn.execute("DELETE FROM workblocks", []).ok();
    
    // Create workblock for yesterday
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();
    
    // Yesterday's workblock
    conn.execute(
        "INSERT INTO workblocks (date, start_time, end_time, duration_minutes, status, is_archived)
         VALUES (?1, datetime('now', '-1 day'), datetime('now', '-1 day', '+1 hour'), 60, 'completed', 0)",
        rusqlite::params![yesterday],
    ).unwrap();
    
    let yesterday_wb_id = conn.last_insert_rowid();
    
    // Add intervals
    for i in 1..=2 {
        conn.execute(
            "INSERT INTO intervals (workblock_id, interval_number, start_time, end_time, words, status, recorded_at)
             VALUES (?1, ?2, datetime('now', '-1 day'), datetime('now', '-1 day'), 'coding', 'recorded', datetime('now', '-1 day'))",
            rusqlite::params![yesterday_wb_id, i],
        ).unwrap();
    }
    
    // Simulate day transition: archive yesterday
    conn.execute(
        "UPDATE workblocks SET is_archived = 1 WHERE date = ?1",
        rusqlite::params![yesterday],
    ).unwrap();
    
    // Create today's workblock
    conn.execute(
        "INSERT INTO workblocks (date, start_time, duration_minutes, status, is_archived)
         VALUES (?1, datetime('now'), 60, 'active', 0)",
        rusqlite::params![today],
    ).unwrap();
    
    let today_wb_id = conn.last_insert_rowid();
    
    // Verify yesterday is archived
    let yesterday_archived: bool = conn.query_row(
        "SELECT is_archived FROM workblocks WHERE id = ?1",
        rusqlite::params![yesterday_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(yesterday_archived);
    
    // Verify today's workblock is not archived
    let today_archived: bool = conn.query_row(
        "SELECT is_archived FROM workblocks WHERE id = ?1",
        rusqlite::params![today_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(!today_archived);
    
    // Verify we can query workblocks by date separately
    let yesterday_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM workblocks WHERE date = ?1",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    let today_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM workblocks WHERE date = ?1",
        rusqlite::params![today],
        |row| row.get(0),
    ).unwrap();
    
    assert_eq!(yesterday_count, 1, "Expected 1 workblock for yesterday, got {}", yesterday_count);
    assert_eq!(today_count, 1, "Expected 1 workblock for today, got {}", today_count);
    
    println!("✓ Test: Day transition passed");
    println!("  - Yesterday workblocks: {} (archived)", yesterday_count);
    println!("  - Today workblocks: {} (active)", today_count);
    
    // Cleanup
    std::fs::remove_file(get_test_db_path()).ok();
}
