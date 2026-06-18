import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Session, TimerState } from "../types/session";
import "./WorkblockControl.css";

type TimerConfigInfo = {
    dev_mode: boolean;
    interval_tick_seconds: number;
    auto_away_delay_seconds: number;
    logical_interval_minutes: number;
};

interface WorkblockControlProps {
    onNavigateToSummary?: () => void;
    onNavigateToArchive?: () => void;
}

export default function WorkblockControl({ onNavigateToSummary, onNavigateToArchive }: WorkblockControlProps) {
    const [activeSession, setActiveSession] = useState<Session | null>(null);
    const [timerState, setTimerState] = useState<TimerState | null>(null);
    const [timerConfig, setTimerConfig] = useState<TimerConfigInfo | null>(null);
    const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
    const [loading, setLoading] = useState(false);
    const [showInfo, setShowInfo] = useState(false);

    useEffect(() => {
        loadActiveSession();
        loadTimerState();
        loadTimerConfig();

        const interval = setInterval(loadTimerState, 1000);

        let unlistenPromise: Promise<() => void> | null = null;
        listen("session-stopped", async () => {
            await loadActiveSession();
            await loadTimerState();
        }).then(fn => { unlistenPromise = Promise.resolve(fn); });

        return () => {
            clearInterval(interval);
            unlistenPromise?.then(fn => fn());
        };
    }, []);

    const loadTimerConfig = async () => {
        try {
            setTimerConfig(await invoke<TimerConfigInfo>("get_timer_config_cmd"));
        } catch { /* older backend */ }
    };

    const loadActiveSession = async () => {
        try {
            setActiveSession(await invoke<Session | null>("get_active_session_cmd"));
        } catch (e) { console.error(e); }
    };

    const loadTimerState = async () => {
        try {
            const state = await invoke<TimerState>("get_timer_state");
            setTimerState(state);
            setTimeRemaining(await invoke<number | null>("get_interval_time_remaining"));
        } catch (e) { console.error(e); }
    };

    const handleStart = async () => {
        setLoading(true);
        try {
            setActiveSession(await invoke<Session>("start_session_cmd"));
            await loadTimerState();
        } catch (e) {
            alert(`Failed to start session: ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const handleStop = async () => {
        if (!activeSession?.id) return;
        setLoading(true);
        try {
            try { await invoke("hide_prompt_window_cmd"); } catch { /* ignore */ }
            await invoke("stop_session_cmd", { sessionId: activeSession.id });
            await loadActiveSession();
            await loadTimerState();
        } catch (e) {
            alert(`Failed to stop session: ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const fmt = (seconds: number) => {
        const m = Math.floor(seconds / 60);
        const s = seconds % 60;
        return `${m}:${String(s).padStart(2, "0")}`;
    };

    const fmtTime = (iso: string) =>
        new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

    return (
        <div className="wbc">
            {timerConfig?.dev_mode && (
                <div className="wbc-dev-banner">
                    <strong>Dev timers enabled.</strong>{" "}
                    Interval: {timerConfig.interval_tick_seconds}s · Auto-away: {timerConfig.auto_away_delay_seconds}s
                </div>
            )}

            <div className="wbc-header">
                <h1 className="wbc-title">Log15</h1>
                <div className="wbc-nav">
                    {onNavigateToSummary && (
                        <button className="wbc-nav-btn" onClick={onNavigateToSummary}>
                            Summary
                        </button>
                    )}
                    {onNavigateToArchive && (
                        <button className="wbc-nav-btn" onClick={onNavigateToArchive}>
                            Archive
                        </button>
                    )}
                </div>
            </div>

            {activeSession ? (
                <div className="wbc-card">
                    <p className="wbc-card-title">Active Session</p>

                    {timerState?.is_running && timeRemaining !== null && (
                        <div className="wbc-timer">{fmt(timeRemaining)}</div>
                    )}

                    <div className="wbc-rows">
                        <div className="wbc-row">
                            <span className="wbc-row-label">Started</span>
                            <span className="wbc-row-value">{fmtTime(activeSession.start_time)}</span>
                        </div>
                        {timerState?.is_running && (
                            <div className="wbc-row">
                                <span className="wbc-row-label">Interval</span>
                                <span className="wbc-row-value">
                                    <span className="wbc-interval-pill">
                                        #{timerState.current_interval_number}
                                    </span>
                                </span>
                            </div>
                        )}
                    </div>

                    <button className="wbc-btn-danger" onClick={handleStop} disabled={loading}>
                        {loading ? "Stopping…" : "Stop Session"}
                    </button>
                </div>
            ) : (
                <div className="wbc-card">
                    <p className="wbc-form-title">New Session</p>

                    <button
                        className="wbc-btn-primary"
                        onClick={handleStart}
                        disabled={loading}
                    >
                        {loading ? "Starting…" : "Start Session"}
                    </button>
                </div>
            )}

            <div className="wbc-info-wrap">
                <button
                    className="wbc-info-btn"
                    onMouseEnter={() => setShowInfo(true)}
                    onMouseLeave={() => setShowInfo(false)}
                    aria-label="How it works"
                >
                    ?
                </button>
                {showInfo && (
                    <div
                        className="wbc-tooltip"
                        onMouseEnter={() => setShowInfo(true)}
                        onMouseLeave={() => setShowInfo(false)}
                    >
                        <h4>How it works</h4>
                        <ul>
                            <li>Every 15 minutes you'll be prompted to enter 1–2 words about what you're doing</li>
                            <li>The prompt appears in the top-right corner of your screen</li>
                            <li>Stop the session when you're done to review your summary</li>
                            <li>No response within 10 minutes → "Away from workspace" is recorded</li>
                        </ul>
                        <div className="wbc-tooltip-arrow" />
                    </div>
                )}
            </div>
        </div>
    );
}
