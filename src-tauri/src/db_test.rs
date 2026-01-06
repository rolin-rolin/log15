// Test module for database operations
// Run with: cargo test --lib db_test

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use tauri::test::MockRuntime;
    use tauri::{App, Manager};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Helper to create a test app handle
    fn create_test_app() -> tauri::AppHandle<MockRuntime> {
        let app = App::new();
        app.handle()
    }

    #[tokio::test]
    async fn test_create_and_complete_workblock() {
        let app = create_test_app();
        
        // Initialize database
        let conn = init_db(&app).unwrap();
        
        // Create a workblock
        let workblock = create_workblock(&app, 60).unwrap();
        assert!(workblock.id.is_some());
        assert_eq!(workblock.status.as_str(), "active");
        assert_eq!(workblock.duration_minutes, Some(60));
        
        // Add some intervals
        let interval1 = add_interval(&app, workblock.id.unwrap(), 1).unwrap();
        let interval2 = add_interval(&app, workblock.id.unwrap(), 2).unwrap();
        
        // Update intervals with words
        update_interval_words(&app, interval1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, interval2.id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();
        
        // Complete the workblock
        let completed = complete_workblock(&app, workblock.id.unwrap()).unwrap();
        assert_eq!(completed.status.as_str(), "completed");
        assert!(completed.end_time.is_some());
        
        println!("✓ Test: Create and complete workblock passed");
    }

    #[tokio::test]
    async fn test_visualization_generation() {
        let app = create_test_app();
        init_db(&app).unwrap();
        
        // Create workblock with intervals
        let workblock = create_workblock(&app, 60).unwrap();
        let wb_id = workblock.id.unwrap();
        
        add_interval(&app, wb_id, 1).unwrap();
        add_interval(&app, wb_id, 2).unwrap();
        add_interval(&app, wb_id, 3).unwrap();
        add_interval(&app, wb_id, 4).unwrap();
        
        let intervals = get_intervals_by_workblock(&app, wb_id).unwrap();
        update_interval_words(&app, intervals[0].id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, intervals[1].id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, intervals[2].id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, intervals[3].id.unwrap(), "planning".to_string(), IntervalStatus::Recorded).unwrap();
        
        complete_workblock(&app, wb_id).unwrap();
        
        // Generate visualization
        let viz = generate_workblock_visualization(&app, wb_id).unwrap();
        
        assert_eq!(viz.timeline_data.len(), 4);
        assert!(viz.activity_data.len() > 0);
        assert!(viz.word_frequency.len() > 0);
        
        // Check activity data
        let coding_activity = viz.activity_data.iter().find(|a| a.words == "coding");
        assert!(coding_activity.is_some());
        assert_eq!(coding_activity.unwrap().total_minutes, 30); // 2 intervals * 15 min
        
        println!("✓ Test: Visualization generation passed");
    }

    #[tokio::test]
    async fn test_daily_aggregate() {
        let app = create_test_app();
        init_db(&app).unwrap();
        
        let today = get_today_date();
        
        // Create multiple workblocks
        let wb1 = create_workblock(&app, 60).unwrap();
        let wb2 = create_workblock(&app, 45).unwrap();
        
        // Add intervals to first workblock
        let int1 = add_interval(&app, wb1.id.unwrap(), 1).unwrap();
        let int2 = add_interval(&app, wb1.id.unwrap(), 2).unwrap();
        update_interval_words(&app, int1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, int2.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        complete_workblock(&app, wb1.id.unwrap()).unwrap();
        
        // Add intervals to second workblock
        let int3 = add_interval(&app, wb2.id.unwrap(), 1).unwrap();
        update_interval_words(&app, int3.id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();
        complete_workblock(&app, wb2.id.unwrap()).unwrap();
        
        // Generate daily aggregate
        let aggregate = generate_daily_aggregate(&app, &today).unwrap();
        
        assert_eq!(aggregate.total_workblocks, 2);
        assert!(aggregate.timeline_data.len() >= 3);
        assert!(aggregate.activity_data.len() >= 2);
        
        // Check that coding appears in aggregate
        let coding = aggregate.activity_data.iter().find(|a| a.words == "coding");
        assert!(coding.is_some());
        
        println!("✓ Test: Daily aggregate passed");
    }

    #[tokio::test]
    async fn test_archiving_and_persistence() {
        let app = create_test_app();
        init_db(&app).unwrap();
        
        let today = get_today_date();
        
        // Create and complete a workblock
        let wb = create_workblock(&app, 60).unwrap();
        let int1 = add_interval(&app, wb.id.unwrap(), 1).unwrap();
        let int2 = add_interval(&app, wb.id.unwrap(), 2).unwrap();
        update_interval_words(&app, int1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(&app, int2.id.unwrap(), "testing".to_string(), IntervalStatus::Recorded).unwrap();
        complete_workblock(&app, wb.id.unwrap()).unwrap();
        
        // Archive the day
        let archive = archive_daily_data(&app, &today).unwrap();
        
        assert_eq!(archive.total_workblocks, 1);
        assert!(archive.visualization_data.is_some());
        
        // Parse visualization data
        let viz_data: serde_json::Value = serde_json::from_str(&archive.visualization_data.unwrap()).unwrap();
        assert!(viz_data["workblocks"].is_array());
        assert!(viz_data["daily_aggregate"].is_object());
        
        // Verify workblock data is in archive
        let workblocks = viz_data["workblocks"].as_array().unwrap();
        assert_eq!(workblocks.len(), 1);
        
        // Verify aggregate data
        let aggregate = &viz_data["daily_aggregate"];
        assert!(aggregate["timeline_data"].is_array());
        assert!(aggregate["activity_data"].is_array());
        
        // Retrieve archived day
        let retrieved = get_archived_day(&app, &today).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().date, today);
        
        println!("✓ Test: Archiving and persistence passed");
    }
}
