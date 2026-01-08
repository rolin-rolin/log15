import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Workblock, TimerState } from "../types/workblock";

interface WorkblockControlProps {
    onNavigateToSummary?: () => void;
    onNavigateToArchive?: () => void;
}

export default function WorkblockControl({ onNavigateToSummary, onNavigateToArchive }: WorkblockControlProps) {
    const [activeWorkblock, setActiveWorkblock] = useState<Workblock | null>(null);
    const [timerState, setTimerState] = useState<TimerState | null>(null);
    const [duration, setDuration] = useState<number>(60); // Default 60 minutes
    const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
    const [loading, setLoading] = useState(false);

    // Load active workblock on mount
    useEffect(() => {
        loadActiveWorkblock();
        loadTimerState();

        // Set up interval to update timer state
        const interval = setInterval(() => {
            loadTimerState();
        }, 1000); // Update every second

        // Listen for workblock events
        const setupListeners = async () => {
            const unlistenComplete = await listen("workblock-complete", () => {
                loadActiveWorkblock();
                loadTimerState();
            });

            return unlistenComplete;
        };

        let unlistenPromise: Promise<() => void> | null = null;
        setupListeners().then((unlisten) => {
            unlistenPromise = Promise.resolve(unlisten);
        });

        return () => {
            clearInterval(interval);
            unlistenPromise?.then((fn) => fn());
        };
    }, []);

    const loadActiveWorkblock = async () => {
        try {
            const workblock = await invoke<Workblock | null>("get_active_workblock_cmd");
            setActiveWorkblock(workblock);
        } catch (error) {
            console.error("Failed to load active workblock:", error);
        }
    };

    const loadTimerState = async () => {
        try {
            const state = await invoke<TimerState>("get_timer_state");
            setTimerState(state);

            // Get time remaining for current interval
            const remaining = await invoke<number | null>("get_interval_time_remaining");
            setTimeRemaining(remaining);
        } catch (error) {
            console.error("Failed to load timer state:", error);
        }
    };

    const handleStartWorkblock = async () => {
        setLoading(true);
        try {
            const workblock = await invoke<Workblock>("start_workblock", {
                durationMinutes: duration,
            });
            setActiveWorkblock(workblock);
            await loadTimerState();
        } catch (error) {
            console.error("Failed to start workblock:", error);
            alert(`Failed to start workblock: ${error}`);
        } finally {
            setLoading(false);
        }
    };

    const handleCancelWorkblock = async () => {
        if (!activeWorkblock?.id) return;

        if (!confirm("Are you sure you want to cancel this workblock?")) {
            return;
        }

        setLoading(true);
        try {
            // Hide prompt window if it's open
            try {
                await invoke("hide_prompt_window_cmd");
            } catch (e) {
                // Ignore errors - window might not be open
            }

            await invoke("cancel_workblock_cmd", {
                workblockId: activeWorkblock.id,
            });
            setActiveWorkblock(null);
            setTimerState(null);
            setTimeRemaining(null);
        } catch (error) {
            console.error("Failed to cancel workblock:", error);
            alert(`Failed to cancel workblock: ${error}`);
        } finally {
            setLoading(false);
        }
    };

    const formatTime = (seconds: number) => {
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins}:${secs.toString().padStart(2, "0")}`;
    };

    const formatDuration = (minutes: number) => {
        if (minutes < 60) {
            return `${minutes} min`;
        }
        const hours = Math.floor(minutes / 60);
        const mins = minutes % 60;
        return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
    };

    return (
        <div style={{ padding: "20px", maxWidth: "600px", margin: "0 auto" }}>
            <div
                style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    marginBottom: "20px",
                    flexWrap: "wrap",
                    gap: "10px",
                }}
            >
                <h1 style={{ margin: 0 }}>Log15 - Workblock Tracker</h1>
                <div style={{ display: "flex", gap: "10px" }}>
                    {onNavigateToSummary && (
                        <button
                            onClick={onNavigateToSummary}
                            style={{
                                padding: "8px 16px",
                                backgroundColor: "#4a90e2",
                                color: "white",
                                border: "none",
                                borderRadius: "5px",
                                cursor: "pointer",
                                fontSize: "14px",
                            }}
                        >
                            View Summary
                        </button>
                    )}
                    {onNavigateToArchive && (
                        <button
                            onClick={onNavigateToArchive}
                            style={{
                                padding: "8px 16px",
                                backgroundColor: "#6c757d",
                                color: "white",
                                border: "none",
                                borderRadius: "5px",
                                cursor: "pointer",
                                fontSize: "14px",
                            }}
                        >
                            Archive
                        </button>
                    )}
                </div>
            </div>

            {activeWorkblock ? (
                <div
                    style={{
                        border: "2px solid #4a90e2",
                        borderRadius: "8px",
                        padding: "20px",
                        marginTop: "20px",
                    }}
                >
                    <h2>Active Workblock</h2>
                    <p>
                        <strong>Duration:</strong> {formatDuration(activeWorkblock.duration_minutes || 0)}
                    </p>
                    <p>
                        <strong>Started:</strong> {new Date(activeWorkblock.start_time).toLocaleTimeString()}
                    </p>

                    {timerState?.is_running && (
                        <div style={{ marginTop: "15px" }}>
                            <p>
                                <strong>Current Interval:</strong> {timerState.current_interval_number}
                            </p>
                            {timeRemaining !== null && (
                                <p>
                                    <strong>Time Remaining:</strong> {formatTime(timeRemaining)}
                                </p>
                            )}
                        </div>
                    )}

                    <div style={{ marginTop: "20px", display: "flex", gap: "10px" }}>
                        <button
                            onClick={handleCancelWorkblock}
                            disabled={loading}
                            style={{
                                padding: "10px 20px",
                                backgroundColor: "#dc3545",
                                color: "white",
                                border: "none",
                                borderRadius: "5px",
                                cursor: loading ? "not-allowed" : "pointer",
                            }}
                        >
                            Cancel Workblock
                        </button>
                    </div>
                </div>
            ) : (
                <div
                    style={{
                        border: "2px solid #e0e0e0",
                        borderRadius: "8px",
                        padding: "20px",
                        marginTop: "20px",
                    }}
                >
                    <h2>Start New Workblock</h2>
                    <div style={{ marginTop: "15px" }}>
                        <label style={{ display: "block", marginBottom: "10px" }}>
                            Duration (15-minute increments):
                        </label>
                        <select
                            value={duration}
                            onChange={(e) => setDuration(Number(e.target.value))}
                            style={{
                                padding: "8px",
                                fontSize: "16px",
                                borderRadius: "5px",
                                border: "1px solid #ccc",
                                width: "100%",
                                maxWidth: "200px",
                            }}
                        >
                            <option value={1}>1 minute (testing)</option>
                            <option value={15}>15 minutes</option>
                            <option value={30}>30 minutes</option>
                            <option value={45}>45 minutes</option>
                            <option value={60}>1 hour</option>
                            <option value={90}>1.5 hours</option>
                            <option value={120}>2 hours</option>
                            <option value={180}>3 hours</option>
                            <option value={240}>4 hours</option>
                        </select>
                    </div>
                    <button
                        onClick={handleStartWorkblock}
                        disabled={loading}
                        style={{
                            marginTop: "20px",
                            padding: "12px 24px",
                            backgroundColor: "#4caf50",
                            color: "white",
                            border: "none",
                            borderRadius: "5px",
                            fontSize: "16px",
                            cursor: loading ? "not-allowed" : "pointer",
                        }}
                    >
                        {loading ? "Starting..." : "Start Workblock"}
                    </button>
                </div>
            )}

            <div style={{ marginTop: "30px", padding: "15px", backgroundColor: "#f5f5f5", borderRadius: "5px" }}>
                <h3>How it works:</h3>
                <ul style={{ lineHeight: "1.8" }}>
                    <li>Every 15 minutes, you'll be prompted to enter 1-2 words about what you're doing</li>
                    <li>The overlay window will appear at the bottom-right corner</li>
                    <li>After the last interval, you'll see a "Summary Ready" notification</li>
                    <li>If you don't respond within 10 minutes, "Away from workspace" will be auto-recorded</li>
                </ul>
            </div>
        </div>
    );
}
