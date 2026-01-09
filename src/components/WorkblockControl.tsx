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
    const [hours, setHours] = useState<number>(1); // Default 1 hour
    const [minutes, setMinutes] = useState<number>(0); // Default 0 minutes
    const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
    const [loading, setLoading] = useState(false);
    const [showInfoOverlay, setShowInfoOverlay] = useState(false);

    // Calculate total duration in minutes
    const duration = hours * 60 + minutes;

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
        if (!activeWorkblock?.id) {
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

            // Explicitly reload state from backend to ensure UI updates
            await loadActiveWorkblock();
            await loadTimerState();
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
        <div
            style={{
                padding: "20px",
                maxWidth: "600px",
                margin: "0 auto",
                display: "flex",
                flexDirection: "column",
                justifyContent: "flex-start",
                alignItems: "center",
            }}
        >
            <div style={{ textAlign: "center", marginBottom: "20px", width: "100%" }}>
                <h1 style={{ margin: 0, marginBottom: "15px" }}>Log15</h1>
                <div style={{ display: "flex", gap: "10px", justifyContent: "center" }}>
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
                        width: "100%",
                        textAlign: "center",
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

                    <div style={{ marginTop: "20px", display: "flex", gap: "10px", justifyContent: "center" }}>
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
                        width: "100%",
                        textAlign: "center",
                    }}
                >
                    <h2>Start New Workblock</h2>
                    <div style={{ marginTop: "15px", display: "flex", flexDirection: "column", alignItems: "center" }}>
                        <label style={{ display: "block", marginBottom: "10px" }}>Duration:</label>
                        <div style={{ display: "flex", gap: "15px", alignItems: "center" }}>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={{ fontSize: "12px", color: "#666" }}>Hours</label>
                                <select
                                    value={hours}
                                    onChange={(e) => setHours(Number(e.target.value))}
                                    style={{
                                        padding: "8px",
                                        fontSize: "16px",
                                        borderRadius: "5px",
                                        border: "1px solid #ccc",
                                        width: "100px",
                                    }}
                                >
                                    {[0, 1, 2, 3, 4].map((h) => (
                                        <option key={h} value={h}>
                                            {h} {h === 1 ? "hr" : "hrs"}
                                        </option>
                                    ))}
                                </select>
                            </div>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={{ fontSize: "12px", color: "#666" }}>Minutes</label>
                                <select
                                    value={minutes}
                                    onChange={(e) => setMinutes(Number(e.target.value))}
                                    style={{
                                        padding: "8px",
                                        fontSize: "16px",
                                        borderRadius: "5px",
                                        border: "1px solid #ccc",
                                        width: "100px",
                                    }}
                                >
                                    {[0, 15, 30, 45].map((m) => (
                                        <option key={m} value={m}>
                                            {m} min
                                        </option>
                                    ))}
                                </select>
                            </div>
                        </div>
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

            <div style={{ marginTop: "30px", textAlign: "center", position: "relative" }}>
                <button
                    onMouseEnter={() => setShowInfoOverlay(true)}
                    onMouseLeave={() => setShowInfoOverlay(false)}
                    style={{
                        width: "24px",
                        height: "32px",
                        borderRadius: "12px",
                        border: "2px solid #4a90e2",
                        backgroundColor: "transparent",
                        color: "#4a90e2",
                        fontSize: "18px",
                        cursor: "pointer",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        margin: "0 auto",
                        padding: 0,
                    }}
                    aria-label="How it works"
                >
                    ℹ
                </button>
                {showInfoOverlay && (
                    <div
                        style={{
                            position: "absolute",
                            bottom: "100%",
                            left: "50%",
                            transform: "translateX(-50%)",
                            marginBottom: "10px",
                            padding: "15px",
                            backgroundColor: "#333",
                            color: "white",
                            borderRadius: "8px",
                            width: "300px",
                            boxShadow: "0 4px 12px rgba(0, 0, 0, 0.3)",
                            zIndex: 1000,
                            textAlign: "left",
                        }}
                        onMouseEnter={() => setShowInfoOverlay(true)}
                        onMouseLeave={() => setShowInfoOverlay(false)}
                    >
                        <h3 style={{ margin: "0 0 10px 0", fontSize: "16px", fontWeight: 600, textAlign: "left" }}>
                            How it works:
                        </h3>
                        <ul
                            style={{
                                margin: 0,
                                paddingLeft: "20px",
                                lineHeight: "1.6",
                                fontSize: "14px",
                                textAlign: "left",
                            }}
                        >
                            <li>Every 15 minutes, you'll be prompted to enter 1-2 words about what you're doing</li>
                            <li>The prompt window will show up on the top right of your screen</li>
                            <li>At the end of your workblock, you can review what you did</li>
                            <li>If you don't respond within 10 minutes, "Away from workspace" will be auto-recorded</li>
                        </ul>
                        <div
                            style={{
                                position: "absolute",
                                bottom: "-8px",
                                left: "50%",
                                transform: "translateX(-50%)",
                                width: 0,
                                height: 0,
                                borderLeft: "8px solid transparent",
                                borderRight: "8px solid transparent",
                                borderTop: "8px solid #333",
                            }}
                        />
                    </div>
                )}
            </div>
        </div>
    );
}
