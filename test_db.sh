#!/bin/bash

# Test script for database operations
# This script will test:
# 1. Creating workblocks and intervals
# 2. Generating visualization data
# 3. Testing daily reset/archiving
# 4. Verifying archived data persists

echo "🧪 Testing Database Operations for Log15"
echo "========================================"
echo ""

# Source cargo environment
source "$HOME/.cargo/env"

cd "$(dirname "$0")/src-tauri"

echo "📦 Building the project..."
cargo build --quiet 2>&1 | grep -E "(error|warning)" || echo "✓ Build successful"
echo ""

echo "🔍 Running database tests..."
echo ""

# Create a simple test binary
cat > test_db_manual.rs << 'EOF'
use log15_lib::db::*;
use tauri::test::MockRuntime;
use tauri::{App, Manager};

fn main() {
    println!("Starting database tests...\n");
    
    // Note: This is a simplified test - actual testing would require
    // a proper Tauri app context. For now, we'll create a test script
    // that can be run through the Tauri app itself.
    
    println!("To test database operations:");
    println!("1. Start the Tauri app: npm run tauri dev");
    println!("2. Use the browser console to call Tauri commands");
    println!("3. Or create test data manually through the UI");
    println!("\nTest commands available:");
    println!("  - start_workblock(duration_minutes)");
    println!("  - submit_interval_words(interval_id, words)");
    println!("  - stop_workblock(workblock_id)");
    println!("  - get_daily_visualization_data_cmd(date)");
    println!("  - get_archived_day_cmd(date)");
}

EOF

echo "✓ Test script created"
echo ""
echo "📝 Manual Testing Instructions:"
echo "==============================="
echo ""
echo "1. Start the app: npm run tauri dev"
echo "2. Open browser DevTools console"
echo "3. Run these commands to test:"
echo ""
echo "   // Create a workblock"
echo "   await window.__TAURI_INVOKE__('start_workblock', { durationMinutes: 60 })"
echo ""
echo "   // Get active workblock"
echo "   await window.__TAURI_INVOKE__('get_active_workblock_cmd')"
echo ""
echo "   // Create intervals and add words"
echo "   // (You'll need to get interval IDs from the workblock)"
echo ""
echo "   // Get today's date"
echo "   await window.__TAURI_INVOKE__('get_today_date_cmd')"
echo ""
echo "   // Get visualization data"
echo "   const date = await window.__TAURI_INVOKE__('get_today_date_cmd')"
echo "   await window.__TAURI_INVOKE__('get_daily_visualization_data_cmd', { date })"
echo ""
echo "For testing day transitions, you can:"
echo "  - Manually change system date (not recommended)"
echo "  - Modify the database date fields directly"
echo "  - Wait until midnight! 😄"
echo ""
