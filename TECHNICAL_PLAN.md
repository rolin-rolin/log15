# Log15 Technical Implementation Plan

## Overview

A desktop workblock tracking application that prompts users every 15 minutes to log what they're working on, with daily summaries and historical archives.

## Architecture

### Tech Stack

-   **Frontend**: React + TypeScript + Vite
-   **Backend**: Rust (Tauri v2)
-   **Database**: SQLite (rusqlite)
-   **Key Tauri Plugins**: System Tray, Window Management, Notifications

---

## Database Schema

### Tables

#### `workblocks`

```sql
CREATE TABLE workblocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,  -- YYYY-MM-DD format
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    duration_minutes INTEGER,  -- Calculated duration
    status TEXT NOT NULL,  -- 'active', 'completed', 'cancelled'
    is_archived BOOLEAN DEFAULT 0,  -- True when day is archived
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
)
```

#### `intervals`

```sql
CREATE TABLE intervals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workblock_id INTEGER NOT NULL,
    interval_number INTEGER NOT NULL,  -- 1st, 2nd, 3rd 15-min interval
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    words TEXT,  -- 1-2 words user entered
    status TEXT NOT NULL,  -- 'pending', 'recorded', 'auto_away'
    recorded_at DATETIME,
    FOREIGN KEY (workblock_id) REFERENCES workblocks(id)
)
```

#### `daily_archives`

```sql
CREATE TABLE daily_archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,  -- YYYY-MM-DD format
    total_workblocks INTEGER DEFAULT 0,
    total_minutes INTEGER DEFAULT 0,
    -- Pre-computed visualization data stored as JSON for quick retrieval
    -- Structure: { workblocks: [{ id, timeline_data, activity_data, word_frequency }] }
    visualization_data TEXT,  -- JSON string containing all visualization data
    archived_at DATETIME DEFAULT CURRENT_TIMESTAMP
)
```

**Visualization Data JSON Structure**:

```json
{
    "workblocks": [
        {
            "id": 1,
            "timeline_data": [
                {
                    "interval_number": 1,
                    "start_time": "...",
                    "end_time": "...",
                    "words": "coding",
                    "duration_minutes": 15
                },
                {
                    "interval_number": 2,
                    "start_time": "...",
                    "end_time": "...",
                    "words": "meeting",
                    "duration_minutes": 15
                }
            ],
            "activity_data": [
                { "words": "coding", "total_minutes": 45, "percentage": 50.0 },
                { "words": "meeting", "total_minutes": 30, "percentage": 33.3 }
            ],
            "word_frequency": [
                { "word": "coding", "count": 3 },
                { "word": "meeting", "count": 2 }
            ]
        }
    ],
    "daily_aggregate": {
        "total_workblocks": 3,
        "total_minutes": 180,
        "timeline_data": [
            // All intervals from all workblocks, sorted chronologically
            {
                "workblock_id": 1,
                "interval_number": 1,
                "start_time": "...",
                "end_time": "...",
                "words": "coding",
                "duration_minutes": 15
            },
            {
                "workblock_id": 1,
                "interval_number": 2,
                "start_time": "...",
                "end_time": "...",
                "words": "meeting",
                "duration_minutes": 15
            },
            {
                "workblock_id": 2,
                "interval_number": 1,
                "start_time": "...",
                "end_time": "...",
                "words": "coding",
                "duration_minutes": 15
            }
        ],
        "activity_data": [
            // Combined activity breakdown across all workblocks
            { "words": "coding", "total_minutes": 90, "percentage": 50.0 },
            { "words": "meeting", "total_minutes": 60, "percentage": 33.3 },
            { "words": "planning", "total_minutes": 30, "percentage": 16.7 }
        ],
        "word_frequency": [
            // Combined word frequency across all workblocks
            { "word": "coding", "count": 6 },
            { "word": "meeting", "count": 4 },
            { "word": "planning", "count": 2 }
        ]
    }
}
```

**Daily Aggregate Summary Rules**:

-   Initialized when the first workblock of the day starts
-   Updated incrementally as each workblock completes (add its data to aggregate)
-   Workblocks that start before midnight but end after midnight are counted in the day they started
-   Each workblock maintains its own individual data in `workblocks` array
-   Aggregate combines all workblocks' intervals, activities, and word frequencies

**Note**: Raw data remains in `workblocks` and `intervals` tables (marked as archived). The JSON in `daily_archives` provides pre-computed visualization data for fast display without recalculation. If visualization logic changes, we can regenerate from raw data.

---

## Core Components

### 1. System Tray Integration

**Location**: `src-tauri/src/tray.rs`

**Features**:

-   System tray icon with different states:
    -   **Idle**: Gray/default icon (no active workblock)
    -   **Active**: Colored icon (workblock in progress)
    -   **Summary Ready**: Different color (workblock completed, summary available)
-   Context menu:
    -   "Start Workblock"
    -   "View Summary" (only when available)
    -   "View Last Words" (during active workblock)
    -   "Quit"

**Tauri Plugin**: `tauri-plugin-system-tray`

### 2. Workblock Timer System

**Location**: `src-tauri/src/timer.rs`

**Features**:

-   Manages active workblock state
-   15-minute interval tracking
-   Auto-advance to next interval
-   Auto-record "Away from workspace" after 10 minutes of no response
-   Background operation (continues when main window closed)

**Implementation**:

-   Use `tokio::time::interval` for 15-minute timers
-   Store active workblock state in memory (with persistence to DB)
-   Emit events to frontend when intervals complete

### 3. Overlay Prompt Window

**Location**:

-   Rust: `src-tauri/src/window_manager.rs`
-   Frontend: `src/components/PromptWindow.tsx`

**Features**:

-   Small overlay window (bottom-right corner)
-   Always-on-top, transparent background
-   Fade-in animation on show
-   Input field for 1-2 words
-   Submit button or Enter key
-   On submit: Show green checkmark, fade-out animation
-   Auto-close after 10 minutes with "Away from workspace"

**Window Configuration**:

```rust
WindowBuilder::new()
    .title("Log15 Prompt")
    .width(300)
    .height(120)
    .always_on_top(true)
    .decorations(false)
    .transparent(true)
    .skip_taskbar(true)
    .visible(false)
```

**CSS Animations**:

-   Fade-in: `@keyframes fadeIn` (opacity 0 → 1, transform scale)
-   Fade-out: `@keyframes fadeOut` (opacity 1 → 0, transform scale)
-   Checkmark: Green checkmark icon with scale animation

### 4. Daily Reset Logic

**Location**: `src-tauri/src/db.rs` (add `check_and_reset_daily` function)

**Logic**:

-   On app startup or first workblock start:
    -   Check if current date differs from last workblock date
    -   If new day:
        1. Archive previous day's data to `daily_archives`
        2. Clear active workblocks (mark as completed if still active)
        3. Reset daily counters

### 5. Summary View

**Location**: `src/components/SummaryView.tsx`

**Features**:

-   Tabbed interface (one tab per workblock for current day)
-   Each tab shows:
    -   Timeline visualization (horizontal bar chart)
    -   List of intervals with words
    -   Time spent per activity (word grouping)
    -   Word frequency chart
-   Archive view for past days (read-only)
-   Data visualizations:
    -   **Timeline**: Horizontal bar showing intervals with color coding
    -   **Activity Time**: Pie chart or bar chart of time spent per word/activity
    -   **Word Frequency**: Bar chart of most common words

**Data Processing**:

-   Group intervals by words (case-insensitive)
-   Calculate total minutes per activity
-   Count word frequency across all intervals

### 6. Main Application Window

**Location**: `src/App.tsx`, `src/components/WorkblockControl.tsx`

**Features**:

-   Start workblock interface:
    -   Duration selector (15-minute increments)
    -   Start button
-   Active workblock view:
    -   Current interval number
    -   Time remaining in current interval
    -   Last few words entered
    -   Stop workblock button (with confirmation)
-   Summary navigation button

---

## Implementation Steps

### Phase 1: Database & Core Data Models

1. ✅ Update database schema in `db.rs`

    - Create `workblocks` table
    - Create `intervals` table
    - Create `daily_archives` table
    - Add helper functions for CRUD operations

2. Create Rust data models

    - `Workblock` struct
    - `Interval` struct
    - `DailyArchive` struct
    - Serialization with serde

3. Implement database operations
    - `create_workblock()`
    - `get_active_workblock()`
    - `add_interval()`
    - `update_interval_words()`
    - `complete_workblock()` - Also updates daily aggregate when workblock completes
    - `cancel_workblock()`
    - `get_workblocks_by_date()`
    - `initialize_daily_aggregate(date)` - Initialize aggregate with first workblock of day
    - `update_daily_aggregate(workblock_id)` - Add workblock data to daily aggregate
    - `get_daily_aggregate(date)` - Get current day's aggregate (for active day, not archived)
    - `generate_visualization_data(workblock_id)` - Generate timeline, activity, and word frequency data for single workblock
    - `generate_daily_aggregate(date)` - Generate aggregate visualization data from all workblocks for a day
    - `archive_daily_data()` - Archive day's data and generate/store visualization JSON (includes aggregate)
    - `check_and_reset_daily()`
    - `get_archived_day(date)` - Retrieve archived day with visualization data (includes aggregate)

### Phase 2: System Tray

1. Add `tauri-plugin-system-tray` to `Cargo.toml`
2. Create `src-tauri/src/tray.rs`
    - System tray icon setup
    - Context menu creation
    - Icon state management
3. Integrate into `lib.rs`
    - Initialize tray in `setup()`
    - Handle tray events

### Phase 3: Timer System

1. Create `src-tauri/src/timer.rs`
    - Workblock state management
    - 15-minute interval tracking
    - Auto-away logic (10-minute timeout)
    - Background timer with tokio
2. Create Tauri commands:
    - `start_workblock(duration_minutes: i32)`
    - `stop_workblock()`
    - `get_active_workblock()`
    - `get_current_interval()`
3. Create Tauri events:
    - `interval-complete` (emitted every 15 minutes)
    - `workblock-complete` (emitted when workblock ends)

### Phase 4: Overlay Prompt Window

1. Add window management to `lib.rs`
    - Create overlay window builder
    - Window positioning (bottom-right)
    - Window state management
2. Create `src/components/PromptWindow.tsx`
    - Input field (1-2 words)
    - Submit button
    - Checkmark animation
    - Fade in/out animations
3. Create Tauri commands:
    - `show_prompt_window(interval_id: i64)`
    - `hide_prompt_window()`
    - `submit_interval_words(interval_id: i64, words: String)`
4. Handle auto-away:
    - Start 10-minute timer when prompt shows
    - Auto-submit "Away from workspace" if no response

### Phase 5: Main Application UI

1. Update `src/App.tsx`
    - Main layout
    - Route between start/workblock/summary views
2. Create `src/components/WorkblockControl.tsx`
    - Duration selector (15-min increments)
    - Start/Stop buttons
    - Active workblock display
3. Create `src/components/SummaryView.tsx`
    - Tab navigation for workblocks
    - Daily aggregate summary tab (shows combined data from all workblocks)
    - Individual workblock tabs (one per workblock)
    - Timeline visualization (for aggregate and individual workblocks)
    - Activity time chart (for aggregate and individual workblocks)
    - Word frequency chart (for aggregate and individual workblocks)
    - Archive view for past days (with aggregate summaries)
4. Add React hooks:
    - `useWorkblock()` - workblock state management
    - `useIntervals()` - interval data fetching
    - `useDailyData()` - daily summary data

### Phase 6: Daily Reset & Archive

1. Implement `check_and_reset_daily()` in `db.rs`
2. Implement `initialize_daily_aggregate()`:
    - Called when first workblock of the day starts
    - Creates initial aggregate structure with first workblock's data
    - Stores in memory/state (not in DB until archived)
3. Implement `update_daily_aggregate()`:
    - Called when each workblock completes
    - Merges workblock's intervals into aggregate timeline (chronologically sorted)
    - Combines activity breakdown (sum minutes per word/activity)
    - Combines word frequency (sum counts per word)
    - Updates total workblocks and total minutes
4. Implement `generate_visualization_data()` function:
    - For each workblock: Generate timeline data (all intervals with times and words)
    - Calculate activity breakdown: Group intervals by words, sum minutes per activity
    - Calculate word frequency: Count occurrences of each word across all intervals
    - Structure data as JSON matching the schema above
5. Implement `generate_daily_aggregate()`:
    - Combines all workblocks' intervals into single chronological timeline
    - Merges all activity breakdowns (sum minutes, recalculate percentages)
    - Merges all word frequencies (sum counts)
    - Returns aggregate structure matching JSON schema
6. Implement `archive_daily_data()`:
    - Mark all workblocks for the day as `is_archived = true`
    - Generate visualization data for all workblocks
    - Generate daily aggregate from all workblocks
    - Store aggregated daily summary (total workblocks, total minutes)
    - Store complete visualization JSON (workblocks + daily_aggregate) in `daily_archives.visualization_data`
7. Call `check_and_reset_daily()` on app startup in `setup()`
8. Call `check_and_reset_daily()` before starting new workblock
9. When first workblock of day starts: Call `initialize_daily_aggregate()`
10. When workblock completes: Call `update_daily_aggregate()` to add to daily aggregate
11. Create archive view UI component:
    - Load archived days from `daily_archives` table
    - Parse and display visualization JSON
    - Show individual workblock tabs with their visualizations
    - Show daily aggregate summary (combined across all workblocks)
12. Add navigation to archive in summary view

### Phase 7: Polish & UX

1. Add animations and transitions
2. Add checkmark rewards on word submission
3. System tray icon color changes
4. Smooth window fade animations
5. Error handling and edge cases
6. Testing:
    - Multiple workblocks per day
    - Day transitions
    - App close during active workblock
    - Manual stop workblock

---

## Tauri Dependencies to Add

```toml
[dependencies]
tauri-plugin-system-tray = "2"
tauri-plugin-window-state = "2"  # Optional: save window positions
chrono = "0.4"  # Date/time handling
```

---

## Frontend Dependencies to Add

```json
{
    "dependencies": {
        "recharts": "^2.10.0", // For data visualizations
        "date-fns": "^3.0.0" // Date formatting
    }
}
```

---

## File Structure

```
log15/
├── src/
│   ├── components/
│   │   ├── PromptWindow.tsx      # Overlay prompt window
│   │   ├── WorkblockControl.tsx   # Start/stop workblock UI
│   │   ├── SummaryView.tsx        # Summary with tabs and charts
│   │   ├── TimelineChart.tsx      # Timeline visualization
│   │   ├── ActivityChart.tsx      # Activity time chart
│   │   ├── WordFrequencyChart.tsx # Word frequency chart
│   │   └── ArchiveView.tsx        # Past days archive
│   ├── hooks/
│   │   ├── useWorkblock.ts        # Workblock state hook
│   │   ├── useIntervals.ts        # Interval data hook
│   │   └── useDailyData.ts        # Daily summary hook
│   ├── App.tsx                    # Main app component
│   └── main.tsx
├── src-tauri/
│   ├── src/
│   │   ├── db.rs                  # Database operations
│   │   ├── tray.rs                # System tray setup
│   │   ├── timer.rs               # Timer/interval management
│   │   ├── window_manager.rs      # Overlay window management
│   │   ├── lib.rs                 # Main Tauri setup
│   │   └── main.rs
│   └── Cargo.toml
```

---

## Key Technical Considerations

### Window Fade Animations

-   Use CSS transitions with `opacity` and `transform`
-   Tauri window API supports transparency
-   Use `requestAnimationFrame` for smooth animations
-   Consider using a library like `framer-motion` for React animations

### Background Operation

-   System tray keeps app running
-   Timers continue in background via tokio
-   Overlay windows can be shown even when main window closed
-   State persisted to database for recovery

### Daily Reset Timing

-   Check date on app startup
-   Check date before starting new workblock
-   Use `chrono` crate for reliable date handling
-   Store last reset date in database or config
-   Workblocks that start before midnight but end after midnight are counted in the day they started (based on `start_time` date)

### Auto-Away Logic

-   Start 10-minute timer when prompt window shows
-   If no response, auto-submit "Away from workspace"
-   Cancel timer if user responds
-   Update interval status to `auto_away`

### State Management

-   Active workblock state in Rust (with DB persistence)
-   Daily aggregate state in Rust (maintained in memory during active day, persisted to DB when archived)
-   React state for UI updates
-   Tauri events for communication between Rust and React
-   Local state for prompt window visibility
-   Aggregate updates incrementally: initialized on first workblock, updated on each workblock completion

### Visualization Data Storage

-   **Current Day**:
    -   Individual workblock visualizations generated on-demand from raw `workblocks` and `intervals` data
    -   Daily aggregate maintained incrementally: initialized with first workblock, updated as each workblock completes
    -   Aggregate stored in memory/state during active day
-   **Archived Days**: Pre-computed visualization data stored as JSON in `daily_archives.visualization_data`
    -   Includes individual workblock visualizations
    -   Includes daily aggregate summary (combined across all workblocks)
-   **Benefits of Pre-computation**:
    -   Fast retrieval for archived days (no recalculation needed)
    -   Consistent visualization data even if visualization logic changes
    -   Can display archived data even if raw interval data is modified
-   **Data Structure**: JSON includes:
    -   Individual workblock data: timeline, activity breakdown, word frequency
    -   Daily aggregate: combined timeline, combined activity breakdown, combined word frequency
-   **Regeneration**: If needed, visualization data can be regenerated from raw `workblocks`/`intervals` data using `generate_visualization_data()` and `generate_daily_aggregate()`

---

## Testing Checklist

-   [ ] Start workblock with various durations
-   [ ] 15-minute interval prompts appear correctly
-   [ ] Word submission works and shows checkmark
-   [ ] Auto-away triggers after 10 minutes
-   [ ] Workblock completes at end of duration
-   [ ] Summary shows correct data
-   [ ] Multiple workblocks per day work correctly
-   [ ] Daily reset on new calendar day
-   [ ] Archive view shows past days (read-only)
-   [ ] System tray icon changes states correctly
-   [ ] App continues in background when main window closed
-   [ ] Manual stop workblock works
-   [ ] App close during active workblock stops workblock
-   [ ] Timeline visualization accurate
-   [ ] Activity time calculation correct
-   [ ] Word frequency counting accurate
-   [ ] Daily aggregate initialized with first workblock
-   [ ] Daily aggregate updates correctly as each workblock completes
-   [ ] Daily aggregate combines all workblocks' data correctly
-   [ ] Workblocks spanning midnight counted in correct day (based on start_time)
-   [ ] Visualization data correctly generated and stored in archive
-   [ ] Archived visualization data correctly retrieved and displayed
-   [ ] Archived days show all workblocks with their visualizations
-   [ ] Archived days show daily aggregate summary

---

## Next Steps

Once approved, we'll implement in the order specified above, starting with Phase 1 (Database & Core Data Models).
