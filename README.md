# Log15

A desktop application built with Tauri, React, TypeScript, and SQLite.

## Tech Stack

-   **Frontend**: React + TypeScript + Vite
-   **Backend**: Rust (Tauri)
-   **Database**: SQLite (rusqlite)

## Setup

1. **Activate the virtual environment** (if using Python venv):

    ```bash
    source .venv/bin/activate  # On macOS/Linux
    # or
    .venv\Scripts\activate  # On Windows
    ```

2. **Install Node.js dependencies**:

    ```bash
    npm install
    ```

3. **Install Rust dependencies** (automatically handled by Cargo when building):
   The Rust dependencies are defined in `src-tauri/Cargo.toml` and will be installed automatically when you run the app.

## Development

To run the application in development mode:

```bash
npm run tauri dev
```

This will:

-   Start the Vite dev server for the React frontend
-   Build and run the Tauri application
-   Open the desktop window

## Building

To build the application for production:

```bash
npm run tauri build
```

The built application will be in `src-tauri/target/release/`.

## Project Structure

```
log15/
├── src/                    # React frontend source
│   ├── App.tsx            # Main React component
│   └── main.tsx           # React entry point
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Rust entry point
│   │   ├── lib.rs         # Tauri commands and setup
│   │   └── db.rs          # SQLite database module
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── package.json           # Node.js dependencies
└── README.md             # This file
```

## Database

The SQLite database is automatically initialized when the application starts. The database file is stored in the application's data directory:

-   **macOS**: `~/Library/Application Support/com.ronaldlin.log15/log15.db`
-   **Windows**: `%APPDATA%\com.ronaldlin.log15\log15.db`
-   **Linux**: `~/.local/share/com.ronaldlin.log15/log15.db`

The database includes a `logs` table with the following schema:

-   `id`: INTEGER PRIMARY KEY
-   `title`: TEXT NOT NULL
-   `content`: TEXT
-   `created_at`: DATETIME
-   `updated_at`: DATETIME

## Recommended IDE Setup

-   [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
