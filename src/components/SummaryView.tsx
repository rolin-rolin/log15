import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DailyVisualizationData, DailyArchive } from "../types/session";
import TimelineChart from "./TimelineChart";
import ActivityChart from "./ActivityChart";
import "./SummaryView.css";

interface SummaryViewProps {
    onBack?: () => void;
    date?: string;
}

export default function SummaryView({ onBack, date }: SummaryViewProps) {
    const [vizData, setVizData] = useState<DailyVisualizationData | null>(null);
    const [displayDate, setDisplayDate] = useState<string>("");
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isArchived, setIsArchived] = useState(false);

    useEffect(() => {
        loadSummaryData();
    }, [date]);

    const loadSummaryData = async () => {
        setLoading(true);
        setError(null);

        try {
            const targetDate = date || (await invoke<string>("get_today_date_cmd"));
            setDisplayDate(targetDate);

            const archive = await invoke<DailyArchive | null>("get_archived_day_cmd", { date: targetDate });

            if (archive?.visualization_data) {
                setIsArchived(true);
                setVizData(JSON.parse(archive.visualization_data) as DailyVisualizationData);
            } else {
                setIsArchived(false);
                const json = await invoke<string>("get_daily_visualization_data_cmd", { date: targetDate });
                setVizData(JSON.parse(json) as DailyVisualizationData);
            }
        } catch (err) {
            console.error("Failed to load summary data:", err);
            setError(err instanceof Error ? err.message : "Failed to load summary data");
        } finally {
            setLoading(false);
        }
    };

    const formatDate = (dateStr: string) => {
        try {
            return new Date(dateStr + "T00:00:00").toLocaleDateString("en-US", {
                weekday: "long",
                year: "numeric",
                month: "long",
                day: "numeric",
            });
        } catch {
            return dateStr;
        }
    };

    const formatDuration = (minutes: number) => {
        const h = Math.floor(minutes / 60);
        const m = minutes % 60;
        return h > 0 ? `${h}h ${m}m` : `${m}m`;
    };

    if (loading) {
        return (
            <div style={{ padding: "40px", textAlign: "center" }}>
                <p>Loading summary data...</p>
            </div>
        );
    }

    if (error) {
        return (
            <div style={{ padding: "40px", textAlign: "center", color: "#dc3545" }}>
                <p>Error: {error}</p>
                <button
                    onClick={loadSummaryData}
                    style={{
                        marginTop: "20px",
                        padding: "10px 20px",
                        backgroundColor: "#4a90e2",
                        color: "white",
                        border: "none",
                        borderRadius: "5px",
                        cursor: "pointer",
                    }}
                >
                    Retry
                </button>
            </div>
        );
    }

    if (!vizData || vizData.total_sessions === 0) {
        return (
            <div style={{ padding: "40px", textAlign: "center", color: "#666" }}>
                <p>No summary data available</p>
                <p style={{ fontSize: "14px", marginTop: "10px" }}>Start a session to generate summary data.</p>
            </div>
        );
    }

    return (
        <div className="summary-view">
            <div className="summary-header">
                {onBack && (
                    <button onClick={onBack} className="back-button">
                        ← Back
                    </button>
                )}
                <h1>Summary{displayDate ? ` - ${formatDate(displayDate)}` : ""}</h1>
                {isArchived && <span className="archived-badge">Archived</span>}
            </div>

            <div className="summary-stats">
                <div className="stat-item">
                    <div className="stat-value">{vizData.total_sessions}</div>
                    <div className="stat-label">Session{vizData.total_sessions !== 1 ? "s" : ""}</div>
                </div>
                <div className="stat-item">
                    <div className="stat-value">{formatDuration(vizData.total_minutes)}</div>
                    <div className="stat-label">Total Time</div>
                </div>
            </div>

            <div className="summary-content">
                <TimelineChart
                    timelineData={vizData.timeline_data}
                    title="Timeline"
                />
                <ActivityChart
                    activityData={vizData.activity_data}
                    title="Activity Breakdown"
                />
            </div>
        </div>
    );
}
