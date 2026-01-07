# Overlay Window Feature Test Guide

## Test Scenarios

### 1. Normal Interval Prompt (Non-Last Interval)

1. Start a workblock (e.g., 60 minutes = 4 intervals)
2. Wait for the first 15-minute interval to complete
3. **Expected**:

    - Overlay window appears at bottom-right corner
    - Window shows "What did you do? (1-2 words)" prompt
    - Input field is focused
    - Window has fade-in animation

4. Type words (e.g., "coding") and press Enter or click Submit
5. **Expected**:
    - Green checkmark appears with animation
    - After 1 second, window fades out
    - Window closes completely

### 2. Last Interval → Summary Ready

1. Start a workblock (e.g., 30 minutes = 2 intervals)
2. Complete the first interval normally
3. Wait for the second (last) interval to complete
4. **Expected**:

    - Overlay window appears with prompt
    - Input field is focused

5. Type words and submit
6. **Expected**:

    - Green checkmark appears
    - After 1 second, window transitions to "Summary Ready" view
    - Shows: 📊 icon, "Summary Ready!" title, message, and "Close" button
    - Tray icon state changes to "SummaryReady"

7. Click "Close" button
8. **Expected**:
    - Window fades out
    - Window closes
    - Tray icon state returns to "Idle"

### 3. Auto-Away Feature

1. Start a workblock
2. Wait for interval prompt to appear
3. **Don't submit anything** - wait 10 minutes
4. **Expected**:
    - After 10 minutes, "Away from workspace" is auto-recorded
    - Window closes automatically
    - If it was the last interval, summary window should still appear

### 4. Window Positioning

1. Trigger any interval prompt
2. **Expected**:
    - Window appears at bottom-right corner
    - 20px margin from screen edges
    - Window size: 300x120px (may need adjustment for summary view)

### 5. Keyboard Shortcuts

1. When prompt appears, test:
    - **Enter key**: Submits the form (if words are entered)
    - **Tab**: Should navigate between input and button
    - **Escape**: (Not implemented yet, but could be added)

### 6. Multiple Rapid Intervals

1. Start a short workblock (15 minutes = 1 interval)
2. Complete it quickly
3. **Expected**:
    - Summary window appears immediately after submission
    - No conflicts with previous window state

## Known Issues to Watch For

-   [ ] Window size might be too small for summary view
-   [ ] Window might not position correctly on multi-monitor setups
-   [ ] Fade animations might be choppy
-   [ ] Tray state might not update correctly
-   [ ] Summary window might not appear if workblock completes via timer

## Debugging Tips

If the overlay window doesn't appear:

1. Check browser console for errors
2. Check Rust console output for errors
3. Verify `interval-complete` event is being emitted
4. Check that `show_prompt_window_cmd` is being called
5. Verify window is not being created off-screen

If summary window doesn't appear:

1. Check if `is_last_interval` is being calculated correctly
2. Verify workblock duration and interval count
3. Check that `show_summary_ready()` is being called
4. Verify tray state updates

## Quick Test Commands

To test with a short workblock:

-   Create a 15-minute workblock (1 interval) - summary appears immediately
-   Create a 30-minute workblock (2 intervals) - test normal + summary
-   Create a 60-minute workblock (4 intervals) - test multiple normal prompts
