log15 rework plan

1. Backend rename/refactor
2. Rewrite tests (db_test.rs)
3. Frontend components (update any frontend that references workblocks)
4. Frontend state/hooks
5. Cleanup (remove migration code, debug logging, dead code)
6. Final push/verify

Stage 1 is done. Here's a summary of what was changed:

- db.rs — Fully refactored. Uses Session/sessions everywhere. Has migration code to move old workblocks DB data to sessions.
- lib.rs — All Tauri commands use session naming (start_session_cmd, stop_session_cmd, etc.)
- timer.rs — Clean, uses sessions throughout.
- window_manager.rs — No workblock references (though it has leftover debug logging to a .cursor/debug.log file)
- tray.rs: Renamed start_workblock variable to start_session, updated the three TrayIconState enum comments from "workblock" to "session"
- window_manager.rs: Removed three #region agent log debug blocks that were writing to .cursor/debug.log (leftover from the previous debugging session)

Stage 2 (tests) — not started:
db_test.rs is entirely broken against the new API. It calls functions that no longer exist: create_workblock(), complete_workblock(), generate_workblock_visualization(), get_intervals_by_workblock(), generate_daily_aggregate(). These all need to be rewritten against the current session-based API.
