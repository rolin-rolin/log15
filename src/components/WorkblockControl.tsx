import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Workblock, TimerState } from "../types/workblock";
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
    const [activeWorkblock, setActiveWorkblock] = useState<Workblock | null>(null);
    const [timerState, setTimerState] = useState<TimerState | null>(null);
    const [timerConfig, setTimerConfig] = useState<TimerConfigInfo | null>(null);
    const [hours, setHours] = useState<number>(1);
    const [minutes, setMinutes] = useState<number>(0);
    const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
    const [loading, setLoading] = useState(false);
    const [showInfo, setShowInfo] = useState(false);

    const duration = hours * 60 + minutes;
    const minuteOptions = timerConfig?.dev_mode ? [0, 1, 15, 30, 45] : [0, 15, 30, 45];

    useEffect(() => {
        loadActiveWorkblock();
        loadTimerState();
        loadTimerConfig();

        const interval = setInterval(loadTimerState, 1000);

        let unlistenPromise: Promise<() => void> | null = null;
        listen("workblock-complete", async () => {
            await loadActiveWorkblock();
            await loadTimerState();
        }).then(fn => { unlistenPromise = Promise.resolve(fn); });

        return () => {
            clearInterval(interval);
            unlistenPromise?.then(fn => fn());
        };
    }, []);

    useEffect(() => {
        if (!timerConfig?.dev_mode && minutes === 1) setMinutes(15);
    }, [timerConfig?.dev_mode, minutes]);

    const loadTimerConfig = async () => {
        try {
            setTimerConfig(await invoke<TimerConfigInfo>("get_timer_config_cmd"));
        } catch { /* older backend */ }
    };

    const loadActiveWorkblock = async () => {
        try {
            setActiveWorkblock(await invoke<Workblock | null>("get_active_workblock_cmd"));
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
            setActiveWorkblock(await invoke<Workblock>("start_workblock", { durationMinutes: duration }));
            await loadTimerState();
        } catch (e) {
            alert(`Failed to start workblock: ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const handleCancel = async () => {
        if (!activeWorkblock?.id) return;
        setLoading(true);
        try {
            try { await invoke("hide_prompt_window_cmd"); } catch { /* ignore */ }
            await invoke("cancel_workblock_cmd", { workblockId: activeWorkblock.id });
            await loadActiveWorkblock();
            await loadTimerState();
        } catch (e) {
            alert(`Failed to cancel workblock: ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const fmt = (seconds: number) => {
        const m = Math.floor(seconds / 60);
        const s = seconds % 60;
        return `${m}:${String(s).padStart(2, "0")}`;
    };

    const fmtDuration = (mins: number) => {
        if (mins < 60) return `${mins}m`;
        const h = Math.floor(mins / 60);
        const m = mins % 60;
        return m > 0 ? `${h}h ${m}m` : `${h}h`;
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

            {activeWorkblock ? (
                <div className="wbc-card">
                    <p className="wbc-card-title">Active Workblock</p>

                    {timerState?.is_running && timeRemaining !== null && (
                        <div className="wbc-timer">{fmt(timeRemaining)}</div>
                    )}

                    <div className="wbc-rows">
                        <div className="wbc-row">
                            <span className="wbc-row-label">Duration</span>
                            <span className="wbc-row-value">
                                {fmtDuration(activeWorkblock.duration_minutes || 0)}
                            </span>
                        </div>
                        <div className="wbc-row">
                            <span className="wbc-row-label">Started</span>
                            <span className="wbc-row-value">{fmtTime(activeWorkblock.start_time)}</span>
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

                    <button className="wbc-btn-danger" onClick={handleCancel} disabled={loading}>
                        {loading ? "Cancelling…" : "Cancel Workblock"}
                    </button>
                </div>
            ) : (
                <div className="wbc-card">
                    <p className="wbc-form-title">New Workblock</p>

                    <div className="wbc-duration-row">
                        <div className="wbc-duration-field">
                            <span className="wbc-duration-label">Hours</span>
                            <select
                                className="wbc-select"
                                value={hours}
                                onChange={e => setHours(Number(e.target.value))}
                            >
                                {[0, 1, 2, 3, 4].map(h => (
                                    <option key={h} value={h}>{h}h</option>
                                ))}
                            </select>
                        </div>
                        <div className="wbc-duration-field">
                            <span className="wbc-duration-label">Minutes</span>
                            <select
                                className="wbc-select"
                                value={minutes}
                                onChange={e => setMinutes(Number(e.target.value))}
                            >
                                {minuteOptions.map(m => (
                                    <option key={m} value={m}>{m}m</option>
                                ))}
                            </select>
                        </div>
                    </div>

                    <p className="wbc-duration-hint">
                        {duration === 0
                            ? "Minimum duration is 15 minutes"
                            : `${fmtDuration(duration)} · ${duration / 15} interval${duration / 15 !== 1 ? "s" : ""}`}
                    </p>

                    <button
                        className="wbc-btn-primary"
                        onClick={handleStart}
                        disabled={loading || duration === 0}
                    >
                        {loading ? "Starting…" : "Start Workblock"}
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
                            <li>At the end of your workblock, review your summary</li>
                            <li>No response within 10 minutes → "Away from workspace" is recorded</li>
                        </ul>
                        <div className="wbc-tooltip-arrow" />
                    </div>
                )}
            </div>
        </div>
    );
}
