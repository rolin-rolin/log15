#[cfg(test)]
mod tests {
    use crate::db::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn create_test_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    fn setup(handle: &tauri::AppHandle<tauri::test::MockRuntime>) {
        init_db(handle).unwrap();
        let conn = get_db_connection(handle).unwrap();
        conn.execute_batch(
            "DELETE FROM intervals; DELETE FROM sessions; DELETE FROM daily_archives;",
        )
        .unwrap();
    }

    #[test]
    fn test_create_and_complete_session() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let app = create_test_app();
        let handle = app.handle();

        setup(handle);

        let session = create_session(handle).unwrap();
        assert!(session.id.is_some());
        assert_eq!(session.status.as_str(), "active");

        let s_id = session.id.unwrap();
        let int1 = add_interval(handle, s_id, 1).unwrap();
        let int2 = add_interval(handle, s_id, 2).unwrap();

        update_interval_words(handle, int1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, int2.id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();

        let stopped = stop_session(handle, s_id).unwrap();
        assert_eq!(stopped.status.as_str(), "stopped");
        assert!(stopped.end_time.is_some());
    }

    #[test]
    fn test_visualization_generation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let app = create_test_app();
        let handle = app.handle();
        setup(handle);

        let today = get_today_date();
        let session = create_session(handle).unwrap();
        let s_id = session.id.unwrap();

        let intervals: Vec<_> = (1..=4).map(|n| add_interval(handle, s_id, n).unwrap()).collect();
        update_interval_words(handle, intervals[0].id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, intervals[1].id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, intervals[2].id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, intervals[3].id.unwrap(), "planning".to_string(), IntervalStatus::Recorded).unwrap();

        stop_session(handle, s_id).unwrap();

        let viz = generate_daily_visualization_data(handle, &today).unwrap();

        assert_eq!(viz.timeline_data.len(), 4);
        assert!(!viz.activity_data.is_empty());

        let coding = viz.activity_data.iter().find(|a| a.words == "coding");
        assert!(coding.is_some());
        assert_eq!(coding.unwrap().total_minutes, 30); // 2 intervals × 15 min
    }

    #[test]
    fn test_daily_visualization_aggregate() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let app = create_test_app();
        let handle = app.handle();
        setup(handle);

        let today = get_today_date();

        let s1 = create_session(handle).unwrap();
        let int1 = add_interval(handle, s1.id.unwrap(), 1).unwrap();
        let int2 = add_interval(handle, s1.id.unwrap(), 2).unwrap();
        update_interval_words(handle, int1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, int2.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        stop_session(handle, s1.id.unwrap()).unwrap();

        let s2 = create_session(handle).unwrap();
        let int3 = add_interval(handle, s2.id.unwrap(), 1).unwrap();
        update_interval_words(handle, int3.id.unwrap(), "meeting".to_string(), IntervalStatus::Recorded).unwrap();
        stop_session(handle, s2.id.unwrap()).unwrap();

        let viz = generate_daily_visualization_data(handle, &today).unwrap();

        assert_eq!(viz.total_sessions, 2);
        assert!(viz.timeline_data.len() >= 3);
        assert!(viz.activity_data.len() >= 2);

        let coding = viz.activity_data.iter().find(|a| a.words == "coding");
        assert!(coding.is_some());
    }

    #[test]
    fn test_archiving_and_persistence() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let app = create_test_app();
        let handle = app.handle();
        setup(handle);

        let today = get_today_date();

        let session = create_session(handle).unwrap();
        let s_id = session.id.unwrap();
        let int1 = add_interval(handle, s_id, 1).unwrap();
        let int2 = add_interval(handle, s_id, 2).unwrap();
        update_interval_words(handle, int1.id.unwrap(), "coding".to_string(), IntervalStatus::Recorded).unwrap();
        update_interval_words(handle, int2.id.unwrap(), "testing".to_string(), IntervalStatus::Recorded).unwrap();
        stop_session(handle, s_id).unwrap();

        let archive = archive_daily_data(handle, &today).unwrap();

        assert_eq!(archive.total_sessions, 1);
        assert!(archive.visualization_data.is_some());

        // viz JSON is a serialized DailyVisualizationData
        let viz_data: serde_json::Value = serde_json::from_str(&archive.visualization_data.unwrap()).unwrap();
        assert!(viz_data["timeline_data"].is_array());
        assert!(viz_data["activity_data"].is_array());

        let timeline = viz_data["timeline_data"].as_array().unwrap();
        assert_eq!(timeline.len(), 2);

        let retrieved = get_archived_day(handle, &today).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().date, today);
    }
}
