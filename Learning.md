# Engineering Concepts — Things Worth Remembering

---

## Rust

### Generic type parameters and default types
When you write `&AppHandle` in Rust without a type parameter, you're actually using `AppHandle<Wry<EventLoopMessage>>` — the concrete runtime is the default. This compiles fine in production code, but it means your function *only* accepts that specific runtime. To make a function testable with a mock runtime, you make it generic:

```rust
// Locked to one runtime — can't use with MockRuntime
pub fn create_session(app: &AppHandle) -> Result<Session>

// Accepts any runtime — works in both prod and tests
pub fn create_session<R: Runtime>(app: &AppHandle<R>) -> Result<Session>
```

The lesson: if you want a function to be testable, keep it generic over the things that vary between environments (runtimes, databases, file systems).

### Best-effort vs hard failure in migrations
Not every operation should use `?` to propagate errors. Migration code that runs on startup is a good example — if the column you're trying to migrate from doesn't exist (because it's a fresh install), you don't want to crash. Use `let _ =` to intentionally swallow an error when failure is an expected and safe outcome:

```rust
// Hard failure — crashes on fresh DBs that don't have this column
conn.execute("UPDATE table SET new_col = old_col", [])?;

// Best-effort — fine to skip if old_col doesn't exist
let _ = conn.execute("UPDATE table SET new_col = old_col", []);
```

The distinction: use `?` when failure means something is genuinely wrong. Use `let _ =` when failure is a known, harmless case (like a migration that only applies to old schemas).

### Test isolation with a shared resource
Rust tests run in parallel by default. If multiple tests share a resource (a database file, a port, a temp directory), they'll step on each other. Two tools to fix this:

1. **Mutex** — acquire a lock at the start of each test to force serial execution
2. **Setup function** — clean the shared resource before each test

```rust
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn my_test() {
    let _guard = TEST_MUTEX.lock().unwrap(); // serializes tests
    setup(&handle);                          // clears state
    // ... test body
}
```

The `_guard` naming matters — a plain `_` would drop the lock immediately. The underscore prefix tells Rust "keep this alive for the scope."

---

## Database Migrations

### Schema migrations are standard practice — always have been
Any time you change a database schema in a live app (rename a table, add a column, change a foreign key), you can't just update the code and ship it. Users already have data stored in the old shape. If your code expects `session_id` but the database has `workblock_id`, the app crashes or silently breaks.

The solution is a **migration** — code that runs on startup (or as a separate step) that detects the old schema and transforms it into the new one. In our case we wrote it manually in Rust:

```rust
// If old workblocks table exists, copy its rows into sessions
if sessions_empty && workblocks_exist {
    conn.execute("INSERT INTO sessions ... SELECT ... FROM workblocks", [])?;
}
```

This is the same thing you're doing in Django when you run `python manage.py makemigrations` and `python manage.py migrate` on DigitalOcean. Django just automates the process:

- `makemigrations` — Django detects what changed in your models and generates a migration *file* describing the transformation
- `migrate` — Django runs all pending migration files against the live database in order

The underlying idea is identical: you need a repeatable, tracked record of how to get from schema version A to schema version B without destroying existing data.

### Migrations have a lifetime — delete them when the transition is over
Migration code is scaffolding, not permanent infrastructure. Once every live database has been migrated (either because users have opened the app since the migration shipped, or because you're confident no old databases exist), the migration code can be removed. Keeping it forever means future readers have to understand a transition that already happened and will never happen again.

In Django projects this is less of an issue because migration files are small and tracked in version control. But in hand-written migration code (like ours in `init_db`), it's important to delete it once it's served its purpose.

### The three things a schema migration typically handles
1. **Table renames/additions** — create the new table, copy data from the old one
2. **Column renames** — add the new column, backfill from the old column, drop the old one
3. **Data transformations** — convert values to a new format (e.g. status `"completed"` → `"stopped"`)

All three appeared in our workblock→session migration.

---

## SQL

### COALESCE for backward compatibility
`COALESCE(a, b, c)` returns the first non-NULL value. It's useful for reading from tables that may have different schemas depending on when they were created:

```sql
-- Works whether the row has total_sessions or total_workblocks populated
SELECT COALESCE(total_sessions, total_workblocks, 0) FROM daily_archives
```

But be careful: if a column doesn't *exist at all* (not just NULL), the query will error. `COALESCE` handles NULL values, not missing columns. Once you're confident old schemas are gone, clean up these fallbacks.

---

## Frontend / TypeScript

### Keep frontend types aligned with backend structs
The hardest bugs to track down are mismatches between what the backend returns and what the frontend expects. When you refactor the backend, update the frontend types *first* — they're the contract between the two sides. If the types are right, TypeScript will point you to every broken callsite.

### Dead code accumulates quickly
A component, type, or function that nothing imports still takes up mental space and confuses future readers. When you remove a feature, follow the thread all the way: remove the type, the component, the import, the event listener, and the CSS. Leaving half of it behind is what creates the "what does this do?" problem six months later.

### Event names are a contract
When the backend emits `session-stopped` and the frontend listens for `workblock-complete`, nothing errors — the listener just never fires. Event name mismatches are silent failures, which makes them hard to debug. Treat event names the same way you treat API endpoints: both sides have to agree, and they should be defined in one place if possible.

---

## Tauri-specific

### The backend can own window lifecycle, or the frontend can — pick one
In `PromptWindow`, both the backend (2-second timer → `hide_prompt_window`) and the frontend (`is_last_interval` → `hide_prompt_window_cmd`) were trying to close the window. Neither was wrong on its own, but both doing it created a race condition and confused anyone reading the code. The fix was to commit to one owner — the backend — and remove the frontend's copy. When two layers share responsibility for the same side effect, bugs hide in the overlap.

---

## General Software Engineering

### Stages and checkpoints beat one big refactor
When renaming a major concept across a codebase (workblock → session here), doing it all at once is risky — you lose the ability to test incrementally. Breaking it into stages (backend, tests, frontend components, state, cleanup) means each stage can be verified before the next starts. If something breaks, you know exactly which stage introduced it.

### Migration code has a lifetime
Code that exists to handle the transition from an old schema to a new one should be marked for deletion from the moment you write it. It's not permanent — it's scaffolding. Write down *when* it's safe to remove it (e.g., "once we're confident no old DBs exist") and put it in the cleanup list. Otherwise it stays forever.

### Debug artifacts are technical debt
`console.log`, `#region agent log` HTTP fetch blocks, double-timeout hacks — these are all signs of someone debugging under pressure. They solve the immediate problem but become landmines later. Before merging or shipping, sweep for these. A good rule: if it wouldn't make sense to a reader who didn't write it, it shouldn't be there.

### The "why" matters more than the "what" in comments
A comment that says `// increment counter` is useless — the code already says that. A comment that says `// _guard naming required: plain _ drops the lock immediately` explains a non-obvious invariant a future reader would otherwise have to rediscover. Write comments for the surprising things, not the obvious ones.

### Redundant safety checks add noise, not safety
The triple `checkWindowType()` call (at 0ms, 100ms, 500ms) felt safe but actually introduced a bug — duplicate event listeners. More defensive code is not always safer code. Understand *why* a race condition exists before adding timeouts to work around it. Often the real fix is simpler.
