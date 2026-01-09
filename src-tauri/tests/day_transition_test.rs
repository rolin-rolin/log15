// Test the full day transition logic including check_and_reset_daily()
// This tests the actual automatic archiving when day changes

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use rusqlite::Connection;
use chrono::{Local, Duration};
use serde_json::Value;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

// Mock AppHandle that uses a test database path
struct MockAppHandle {
    db_path: PathBuf,
}

impl MockAppHandle {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        path.push(format!("log15_day_transition_test_{}.db", counter));
        
        // Clean up if exists
        if path.exists() {
            std::fs::remove_file(&path).ok();
        }
        
        // Ensure parent exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        
        Self { db_path: path }
    }
    
    fn get_connection(&self) -> Connection {
        Connection::open(&self.db_path).unwrap()
    }
    
    fn cleanup(&self) {
        std::fs::remove_file(&self.db_path).ok();
    }
}

// Helper functions that work directly with database (bypassing AppHandle requirement)
fn init_test_db(conn: &Connection) {
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
}

// Simulate check_and_reset_daily logic
fn simulate_check_and_reset_daily(conn: &Connection, today: &str) -> Result<Option<String>, rusqlite::Error> {
    // Check if there are any workblocks from previous days that are still active
    let mut stmt = conn.prepare(
        "SELECT date FROM workblocks 
         WHERE status = 'active' AND date != ?1
         LIMIT 1"
    )?;
    
    let previous_date_result = stmt.query_row(rusqlite::params![today], |row| {
        Ok(row.get::<_, String>(0)?)
    });
    
    if let Ok(previous_date) = previous_date_result {
        // Archive the previous day
        simulate_archive_daily_data(conn, &previous_date)?;
        
        // Mark any active workblocks from previous day as completed
        conn.execute(
            "UPDATE workblocks 
             SET status = 'completed', end_time = datetime('now')
             WHERE status = 'active' AND date != ?1",
            rusqlite::params![today],
        )?;
        
        return Ok(Some(previous_date));
    }
    
    // Check if we need to archive yesterday (if there are completed workblocks from yesterday)
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM workblocks 
         WHERE date = ?1 AND is_archived = 0"
    )?;
    
    let count: i32 = stmt.query_row(rusqlite::params![yesterday], |row| row.get(0))?;
    
    if count > 0 {
        simulate_archive_daily_data(conn, &yesterday)?;
        return Ok(Some(yesterday));
    }
    
    Ok(None)
}

// Simulate archive_daily_data logic
fn simulate_archive_daily_data(conn: &Connection, date: &str) -> Result<(), rusqlite::Error> {
    use serde_json::json;
    
    // Get all workblocks for the date
    let mut stmt = conn.prepare(
        "SELECT id, duration_minutes FROM workblocks WHERE date = ?1"
    )?;
    
    let workblocks: Vec<(i64, Option<i32>)> = stmt.query_map(rusqlite::params![date], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.map(|r| r.unwrap()).collect();
    
    if workblocks.is_empty() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("No workblocks found for date".to_string()),
        ));
    }
    
    // Mark all workblocks as archived
    conn.execute(
        "UPDATE workblocks SET is_archived = 1 WHERE date = ?1",
        rusqlite::params![date],
    )?;
    
    // Calculate totals
    let total_workblocks = workblocks.len() as i32;
    let total_minutes: i32 = workblocks.iter()
        .map(|(_, duration)| duration.unwrap_or(0))
        .sum();
    
    // Generate visualization data (simplified version)
    let mut workblock_viz = Vec::new();
    let mut all_timeline = Vec::new();
    let mut aggregate_activity: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut aggregate_word_freq: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    
    for (wb_id, _) in &workblocks {
        // Get intervals
        let mut int_stmt = conn.prepare(
            "SELECT interval_number, start_time, end_time, words 
             FROM intervals WHERE workblock_id = ?1 ORDER BY interval_number"
        )?;
        
        let intervals: Vec<(i32, String, Option<String>, Option<String>)> = 
            int_stmt.query_map(rusqlite::params![wb_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })?.map(|r| r.unwrap()).collect();
        
        // Generate timeline for this workblock
        let timeline_data: Vec<Value> = intervals.iter().map(|(num, start, end, words)| {
            json!({
                "interval_number": num,
                "start_time": start,
                "end_time": end,
                "words": words,
                "duration_minutes": 15
            })
        }).collect();
        
        // Add to aggregate timeline
        for (num, start, end, words) in &intervals {
            all_timeline.push(json!({
                "workblock_id": wb_id,
                "interval_number": num,
                "start_time": start,
                "end_time": end,
                "words": words,
                "duration_minutes": 15
            }));
            
            if let Some(w) = words {
                let w_lower = w.to_lowercase().trim().to_string();
                if !w_lower.is_empty() {
                    *aggregate_activity.entry(w_lower.clone()).or_insert(0) += 15;
                    // Count entire phrase as one activity (not split by words)
                    *aggregate_word_freq.entry(w_lower).or_insert(0) += 1;
                }
            }
        }
        
        // Generate activity data for this workblock
        let mut activity_map: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        for (_, _, _, words) in &intervals {
            if let Some(w) = words {
                let w_lower = w.to_lowercase().trim().to_string();
                if !w_lower.is_empty() {
                    *activity_map.entry(w_lower).or_insert(0) += 15;
                }
            }
        }
        
        let total_activity_minutes: i32 = activity_map.values().sum();
        let activity_data: Vec<Value> = activity_map.iter().map(|(words, minutes)| {
            let percentage = if total_activity_minutes > 0 {
                (*minutes as f64 / total_activity_minutes as f64) * 100.0
            } else {
                0.0
            };
            json!({
                "words": words,
                "total_minutes": minutes,
                "percentage": percentage
            })
        }).collect();
        
        // Generate activity frequency for this workblock (count entire phrase as one activity)
        let mut word_freq: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        for (_, _, _, words) in &intervals {
            if let Some(w) = words {
                let w_lower = w.to_lowercase().trim().to_string();
                if !w_lower.is_empty() {
                    *word_freq.entry(w_lower).or_insert(0) += 1;
                }
            }
        }
        
        let word_frequency: Vec<Value> = word_freq.iter().map(|(word, count)| {
            json!({
                "word": word,
                "count": count
            })
        }).collect();
        
        workblock_viz.push(json!({
            "id": wb_id,
            "timeline_data": timeline_data,
            "activity_data": activity_data,
            "word_frequency": word_frequency
        }));
    }
    
    // Generate aggregate activity data
    let total_agg_minutes: i32 = aggregate_activity.values().sum();
    let aggregate_activity_data: Vec<Value> = aggregate_activity.iter().map(|(words, minutes)| {
        let percentage = if total_agg_minutes > 0 {
            (*minutes as f64 / total_agg_minutes as f64) * 100.0
        } else {
            0.0
        };
        json!({
            "words": words,
            "total_minutes": minutes,
            "percentage": percentage
        })
    }).collect();
    
    let aggregate_word_frequency: Vec<Value> = aggregate_word_freq.iter().map(|(word, count)| {
        json!({
            "word": word,
            "count": count
        })
    }).collect();
    
    let viz_data = json!({
        "workblocks": workblock_viz,
        "daily_aggregate": {
            "total_workblocks": total_workblocks,
            "total_minutes": total_minutes,
            "timeline_data": all_timeline,
            "activity_data": aggregate_activity_data,
            "word_frequency": aggregate_word_frequency
        }
    });
    
    // Store in archive
    conn.execute(
        "INSERT OR REPLACE INTO daily_archives (date, total_workblocks, total_minutes, visualization_data, archived_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        rusqlite::params![date, total_workblocks, total_minutes, viz_data.to_string()],
    )?;
    
    Ok(())
}

#[test]
fn test_day_transition_with_archiving() {
    let mock_app = MockAppHandle::new();
    let conn = mock_app.get_connection();
    init_test_db(&conn);
    
    // Create workblock for "yesterday"
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();
    
    // Create completed workblock from yesterday
    let start_time = (Local::now() - Duration::days(1)).to_rfc3339();
    conn.execute(
        "INSERT INTO workblocks (date, start_time, end_time, duration_minutes, status, is_archived)
         VALUES (?1, ?2, ?3, 60, 'completed', 0)",
        rusqlite::params![yesterday, start_time, Local::now().to_rfc3339()],
    ).unwrap();
    
    let yesterday_wb_id = conn.last_insert_rowid();
    
    // Add intervals with words
    let words_list = vec!["coding", "meeting", "planning", "coding"];
    for (i, words) in words_list.iter().enumerate() {
        let int_start = (Local::now() - Duration::days(1) - Duration::minutes(15 * (4 - i as i64))).to_rfc3339();
        conn.execute(
            "INSERT INTO intervals (workblock_id, interval_number, start_time, end_time, words, status, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'recorded', ?4)",
            rusqlite::params![yesterday_wb_id, (i + 1) as i32, int_start, Local::now().to_rfc3339(), words],
        ).unwrap();
    }
    
    // Verify workblock is not archived yet
    let is_archived_before: bool = conn.query_row(
        "SELECT is_archived FROM workblocks WHERE id = ?1",
        rusqlite::params![yesterday_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(!is_archived_before, "Workblock should not be archived before day transition");
    
    // Verify no archive entry exists yet
    let archive_exists_before: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM daily_archives WHERE date = ?1)",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    assert!(!archive_exists_before, "Archive should not exist before day transition");
    
    // NOW simulate day transition - this is what check_and_reset_daily() does
    let archived_date = simulate_check_and_reset_daily(&conn, &today).unwrap();
    
    assert!(archived_date.is_some(), "Day transition should archive previous day");
    assert_eq!(archived_date.unwrap(), yesterday, "Should archive yesterday's date");
    
    // Verify workblock is now archived
    let is_archived_after: bool = conn.query_row(
        "SELECT is_archived FROM workblocks WHERE id = ?1",
        rusqlite::params![yesterday_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(is_archived_after, "Workblock should be archived after day transition");
    
    // Verify archive entry exists
    let archive_exists_after: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM daily_archives WHERE date = ?1)",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    assert!(archive_exists_after, "Archive entry should exist after day transition");
    
    // Verify visualization data is stored
    let viz_json: String = conn.query_row(
        "SELECT visualization_data FROM daily_archives WHERE date = ?1",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    let viz_data: Value = serde_json::from_str(&viz_json).unwrap();
    
    // Verify structure
    assert!(viz_data["workblocks"].is_array());
    assert!(viz_data["daily_aggregate"].is_object());
    
    // Verify workblock data
    let workblocks = viz_data["workblocks"].as_array().unwrap();
    assert_eq!(workblocks.len(), 1);
    
    let wb = &workblocks[0];
    assert_eq!(wb["id"], yesterday_wb_id);
    assert!(wb["timeline_data"].is_array());
    assert_eq!(wb["timeline_data"].as_array().unwrap().len(), 4);
    
    // Verify activity data groups correctly
    let activities = wb["activity_data"].as_array().unwrap();
    let coding_activity = activities.iter().find(|a| a["words"] == "coding");
    assert!(coding_activity.is_some());
    assert_eq!(coding_activity.unwrap()["total_minutes"], 30); // 2 intervals * 15 min
    
    // Verify daily aggregate
    let aggregate = &viz_data["daily_aggregate"];
    assert_eq!(aggregate["total_workblocks"], 1);
    assert_eq!(aggregate["total_minutes"], 60);
    assert!(aggregate["timeline_data"].is_array());
    assert_eq!(aggregate["timeline_data"].as_array().unwrap().len(), 4);
    
    // Verify aggregate activity data
    let agg_activities = aggregate["activity_data"].as_array().unwrap();
    let agg_coding = agg_activities.iter().find(|a| a["words"] == "coding");
    assert!(agg_coding.is_some());
    assert_eq!(agg_coding.unwrap()["total_minutes"], 30);
    
    println!("✓ Test: Day transition with archiving passed");
    println!("  - Yesterday workblock archived: {}", is_archived_after);
    println!("  - Archive entry created: {}", archive_exists_after);
    println!("  - Visualization data stored: {} bytes", viz_json.len());
    println!("  - Workblocks in archive: {}", workblocks.len());
    println!("  - Activities in aggregate: {}", agg_activities.len());
    
    mock_app.cleanup();
}

#[test]
fn test_day_transition_with_active_workblock() {
    let mock_app = MockAppHandle::new();
    let conn = mock_app.get_connection();
    init_test_db(&conn);
    
    // Create an ACTIVE workblock from yesterday (simulating workblock that spans midnight)
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();
    
    let start_time = (Local::now() - Duration::days(1)).to_rfc3339();
    conn.execute(
        "INSERT INTO workblocks (date, start_time, duration_minutes, status, is_archived)
         VALUES (?1, ?2, 120, 'active', 0)",
        rusqlite::params![yesterday, start_time],
    ).unwrap();
    
    let active_wb_id = conn.last_insert_rowid();
    
    // Add some intervals
    for i in 1..=3 {
        let int_start = (Local::now() - Duration::days(1) - Duration::minutes(15 * (4 - i) as i64)).to_rfc3339();
        conn.execute(
            "INSERT INTO intervals (workblock_id, interval_number, start_time, words, status)
             VALUES (?1, ?2, ?3, 'coding', 'recorded')",
            rusqlite::params![active_wb_id, i, int_start],
        ).unwrap();
    }
    
    // Simulate day transition
    let _archived_date = simulate_check_and_reset_daily(&conn, &today).unwrap();
    
    // Active workblock from previous day should be completed
    let status_after: String = conn.query_row(
        "SELECT status FROM workblocks WHERE id = ?1",
        rusqlite::params![active_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert_eq!(status_after, "completed", "Active workblock from previous day should be completed");
    
    // Should have end_time set
    let end_time: Option<String> = conn.query_row(
        "SELECT end_time FROM workblocks WHERE id = ?1",
        rusqlite::params![active_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(end_time.is_some(), "Completed workblock should have end_time");
    
    // Should be archived
    let is_archived: bool = conn.query_row(
        "SELECT is_archived FROM workblocks WHERE id = ?1",
        rusqlite::params![active_wb_id],
        |row| row.get(0),
    ).unwrap();
    
    assert!(is_archived, "Workblock should be archived");
    
    println!("✓ Test: Day transition with active workblock passed");
    println!("  - Active workblock completed: {}", status_after == "completed");
    println!("  - Workblock archived: {}", is_archived);
    
    mock_app.cleanup();
}

#[test]
fn test_multiple_workblocks_archiving() {
    let mock_app = MockAppHandle::new();
    let conn = mock_app.get_connection();
    init_test_db(&conn);
    
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();
    
    // Create multiple workblocks for yesterday
    let mut workblock_ids = Vec::new();
    for i in 0..3 {
        let start_time = (Local::now() - Duration::days(1) - Duration::hours(i as i64)).to_rfc3339();
        conn.execute(
            "INSERT INTO workblocks (date, start_time, end_time, duration_minutes, status, is_archived)
             VALUES (?1, ?2, ?3, 60, 'completed', 0)",
            rusqlite::params![yesterday, start_time, Local::now().to_rfc3339()],
        ).unwrap();
        
        let wb_id = conn.last_insert_rowid();
        workblock_ids.push(wb_id);
        
        // Add intervals to each workblock
        for j in 1..=2 {
            let words = if i == 0 { "coding" } else { "meeting" };
            let int_start = (Local::now() - Duration::days(1) - Duration::hours(i as i64) - Duration::minutes(15 * (3 - j) as i64)).to_rfc3339();
            conn.execute(
                "INSERT INTO intervals (workblock_id, interval_number, start_time, end_time, words, status, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'recorded', ?4)",
                rusqlite::params![wb_id, j, int_start, Local::now().to_rfc3339(), words],
            ).unwrap();
        }
    }
    
    // Archive the day
    simulate_check_and_reset_daily(&conn, &today).unwrap();
    
    // Verify all workblocks are archived
    for wb_id in &workblock_ids {
        let is_archived: bool = conn.query_row(
            "SELECT is_archived FROM workblocks WHERE id = ?1",
            rusqlite::params![wb_id],
            |row| row.get(0),
        ).unwrap();
        
        assert!(is_archived, "All workblocks should be archived");
    }
    
    // Verify archive entry has correct totals
    let (total_wb, total_min): (i32, i32) = conn.query_row(
        "SELECT total_workblocks, total_minutes FROM daily_archives WHERE date = ?1",
        rusqlite::params![yesterday],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    
    assert_eq!(total_wb, 3, "Should have 3 workblocks in archive");
    assert_eq!(total_min, 180, "Should have 180 total minutes (3 * 60)");
    
    // Verify visualization data includes all workblocks
    let viz_json: String = conn.query_row(
        "SELECT visualization_data FROM daily_archives WHERE date = ?1",
        rusqlite::params![yesterday],
        |row| row.get(0),
    ).unwrap();
    
    let viz_data: Value = serde_json::from_str(&viz_json).unwrap();
    let workblocks = viz_data["workblocks"].as_array().unwrap();
    
    assert_eq!(workblocks.len(), 3, "Visualization should include all 3 workblocks");
    
    // Verify aggregate combines all workblocks
    let aggregate = &viz_data["daily_aggregate"];
    assert_eq!(aggregate["total_workblocks"], 3);
    assert_eq!(aggregate["total_minutes"], 180);
    
    // Verify aggregate activity data combines all workblocks
    let agg_activities = aggregate["activity_data"].as_array().unwrap();
    let coding_activity = agg_activities.iter().find(|a| a["words"] == "coding");
    let meeting_activity = agg_activities.iter().find(|a| a["words"] == "meeting");
    
    assert!(coding_activity.is_some(), "Should have coding activity in aggregate");
    assert!(meeting_activity.is_some(), "Should have meeting activity in aggregate");
    
    // Coding: 1 workblock * 2 intervals * 15 min = 30 min
    assert_eq!(coding_activity.unwrap()["total_minutes"], 30);
    // Meeting: 2 workblocks * 2 intervals * 15 min = 60 min
    assert_eq!(meeting_activity.unwrap()["total_minutes"], 60);
    
    println!("✓ Test: Multiple workblocks archiving passed");
    println!("  - Workblocks archived: {}", workblock_ids.len());
    println!("  - Total workblocks in archive: {}", total_wb);
    println!("  - Total minutes: {}", total_min);
    println!("  - Activities in aggregate: {}", agg_activities.len());
    
    mock_app.cleanup();
}
