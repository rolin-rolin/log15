# Database Testing Guide

This guide helps you test the database operations, particularly verifying that visualization data persists when transitioning to a new day.

## Quick Test via Browser Console

1. **Start the app:**

    ```bash
    npm run tauri dev
    ```

2. **Open DevTools** (usually F12 or Cmd+Option+I)

3. **Run these commands in the console:**

### Test 1: Create Workblock and Add Data

```javascript
// Start a 60-minute workblock
const workblock = await window.__TAURI_INVOKE__("start_workblock", { durationMinutes: 60 });
console.log("Created workblock:", workblock);

// Get the workblock ID
const wbId = workblock.id;

// Create intervals (you'll need to do this through the timer system, but for testing...)
// Note: In real usage, intervals are created automatically every 15 minutes

// Get today's date
const today = await window.__TAURI_INVOKE__("get_today_date_cmd");
console.log("Today:", today);
```

### Test 2: Generate Visualization Data

```javascript
// Get visualization data for today
const vizData = await window.__TAURI_INVOKE__("get_daily_visualization_data_cmd", { date: today });
const parsed = JSON.parse(vizData);
console.log("Visualization data:", parsed);
console.log("Workblocks:", parsed.workblocks);
console.log("Daily aggregate:", parsed.daily_aggregate);
```

### Test 3: Test Archiving (Simulate Day Change)

To test day transitions without waiting for midnight:

1. **Create test data for "yesterday":**

    ```javascript
    // This would normally happen automatically, but for testing we can:
    // 1. Create workblocks
    // 2. Manually archive them
    // 3. Check that archived data persists
    ```

2. **Check archived data:**
    ```javascript
    const yesterday = "2025-01-03"; // Use a past date
    const archived = await window.__TAURI_INVOKE__("get_archived_day_cmd", { date: yesterday });
    if (archived) {
        const viz = JSON.parse(archived.visualization_data);
        console.log("Archived visualization:", viz);
    }
    ```

## Testing Day Transitions

### Option 1: Manual Database Manipulation

You can directly modify the database to test day transitions:

1. **Find the database:**

    - macOS: `~/Library/Application Support/com.ronaldlin.log15/log15.db`
    - Windows: `%APPDATA%\com.ronaldlin.log15\log15.db`
    - Linux: `~/.local/share/com.ronaldlin.log15/log15.db`

2. **Use SQLite CLI:**

    ```bash
    sqlite3 ~/Library/Application\ Support/com.ronaldlin.log15/log15.db
    ```

3. **Manually change dates:**

    ```sql
    -- View current workblocks
    SELECT * FROM workblocks;

    -- Change a workblock date to yesterday
    UPDATE workblocks SET date = '2025-01-03' WHERE id = 1;

    -- Restart the app to trigger daily reset
    ```

### Option 2: Programmatic Test

Create a test that:

1. Creates workblocks with yesterday's date
2. Adds intervals with words
3. Archives the day
4. Verifies visualization data persists
5. Creates new workblock for today
6. Verifies old data is archived and new data is separate

## Expected Behavior

### When a New Day Starts:

1. **On app startup or first workblock:**

    - `check_and_reset_daily()` is called
    - If previous day has unarchived workblocks, they are archived
    - Visualization data is generated and stored in `daily_archives`

2. **Archived data should contain:**

    - All workblocks from that day
    - Individual workblock visualizations (timeline, activity, word frequency)
    - Daily aggregate (combined across all workblocks)

3. **New day starts fresh:**
    - New workblocks are created with today's date
    - Old workblocks remain archived and accessible

## Verification Checklist

-   [ ] Create workblock and add intervals with words
-   [ ] Generate visualization data - verify structure
-   [ ] Archive a day - verify data is stored
-   [ ] Retrieve archived day - verify visualization data is complete
-   [ ] Create new workblock on new day - verify old data is separate
-   [ ] Verify daily aggregate combines all workblocks correctly
-   [ ] Verify individual workblock visualizations are preserved

## SQL Queries for Verification

```sql
-- Check workblocks
SELECT id, date, status, is_archived FROM workblocks ORDER BY date DESC;

-- Check intervals
SELECT i.id, i.workblock_id, i.words, i.status
FROM intervals i
JOIN workblocks w ON i.workblock_id = w.id
ORDER BY w.date DESC, i.interval_number;

-- Check archives
SELECT date, total_workblocks, total_minutes,
       length(visualization_data) as viz_size
FROM daily_archives
ORDER BY date DESC;

-- View visualization data (first 500 chars)
SELECT date, substr(visualization_data, 1, 500) as viz_preview
FROM daily_archives
WHERE date = '2025-01-03';
```

## Troubleshooting

**Issue: Visualization data not persisting**

-   Check that `archive_daily_data()` is being called
-   Verify `visualization_data` column is not NULL in `daily_archives`
-   Check JSON is valid: `SELECT json_valid(visualization_data) FROM daily_archives`

**Issue: Daily reset not working**

-   Verify `check_and_reset_daily()` is called on app startup
-   Check date comparison logic
-   Verify workblocks are marked as `is_archived = 1`

**Issue: Aggregate data incorrect**

-   Verify all intervals have words recorded
-   Check that intervals are grouped correctly by words
-   Verify duration calculations are correct
