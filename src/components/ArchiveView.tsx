import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DailyArchive } from "../types/workblock";
import SummaryView from "./SummaryView";
import "./ArchiveView.css";

export default function ArchiveView({ onBack }: { onBack?: () => void }) {
    const [archivedDates, setArchivedDates] = useState<DailyArchive[]>([]);
    const [selectedDate, setSelectedDate] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        loadArchivedDates();
    }, []);

    const loadArchivedDates = async () => {
        setLoading(true);
        try {
            const dates = await invoke<DailyArchive[]>("get_all_archived_dates_cmd");
            setArchivedDates(dates);
        } catch (error) {
            console.error("Failed to load archived dates:", error);
        } finally {
            setLoading(false);
        }
    };

    const formatDate = (dateStr: string) => {
        try {
            const date = new Date(dateStr + "T00:00:00");
            return date.toLocaleDateString("en-US", {
                weekday: "short",
                year: "numeric",
                month: "short",
                day: "numeric",
            });
        } catch {
            return dateStr;
        }
    };

    if (selectedDate) {
        return <SummaryView date={selectedDate} onBack={() => setSelectedDate(null)} />;
    }

    if (loading) {
        return (
            <div style={{ padding: "40px", textAlign: "center" }}>
                <p>Loading archived dates...</p>
            </div>
        );
    }

    return (
        <div className="archive-view">
            <div className="archive-header">
                {onBack && (
                    <button onClick={onBack} className="back-button">
                        ← Back
                    </button>
                )}
                <h1>Archive</h1>
            </div>

            {archivedDates.length === 0 ? (
                <div className="archive-empty">
                    <p>No archived data available yet.</p>
                    <p className="archive-empty-subtitle">Archived data appears here after day transitions.</p>
                </div>
            ) : (
                <div className="archive-list">
                    {archivedDates.map((archive) => (
                        <div key={archive.date} className="archive-item" onClick={() => setSelectedDate(archive.date)}>
                            <div className="archive-item-date">{formatDate(archive.date)}</div>
                            <div className="archive-item-stats">
                                <span>
                                    {archive.total_workblocks} workblock{archive.total_workblocks !== 1 ? "s" : ""}
                                </span>
                                <span>•</span>
                                <span>
                                    {Math.floor(archive.total_minutes / 60)}h {archive.total_minutes % 60}m
                                </span>
                            </div>
                            <div className="archive-item-arrow">→</div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
