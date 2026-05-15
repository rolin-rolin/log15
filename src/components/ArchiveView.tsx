import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DailyArchive } from "../types/workblock";
import SummaryView from "./SummaryView";
import "./ArchiveView.css";

export default function ArchiveView({ onBack }: { onBack?: () => void }) {
    const [archivedDates, setArchivedDates] = useState<DailyArchive[]>([]);
    const [selectedDate, setSelectedDate] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);

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

    const handleDeleteClick = (e: React.MouseEvent, date: string) => {
        e.stopPropagation();
        setConfirmingDelete(date);
    };

    const handleConfirmDelete = async (e: React.MouseEvent, date: string) => {
        e.stopPropagation();
        try {
            await invoke("delete_day_cmd", { date });
            setArchivedDates((prev) => prev.filter((a) => a.date !== date));
        } catch (error) {
            console.error("Failed to delete day:", error);
        } finally {
            setConfirmingDelete(null);
        }
    };

    const handleCancelDelete = (e: React.MouseEvent) => {
        e.stopPropagation();
        setConfirmingDelete(null);
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
                        <div
                            key={archive.date}
                            className="archive-item"
                            onClick={() => {
                                if (confirmingDelete !== archive.date) {
                                    setSelectedDate(archive.date);
                                }
                            }}
                        >
                            {confirmingDelete === archive.date ? (
                                <div className="archive-delete-confirm" onClick={(e) => e.stopPropagation()}>
                                    <span className="archive-delete-confirm-text">Delete this day?</span>
                                    <button className="archive-confirm-yes" onClick={(e) => handleConfirmDelete(e, archive.date)}>Yes</button>
                                    <button className="archive-confirm-no" onClick={handleCancelDelete}>No</button>
                                </div>
                            ) : (
                                <>
                                    <div className="archive-item-content">
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
                                    </div>
                                    <div className="archive-item-right">
                                        <button
                                            className="archive-delete-btn"
                                            onClick={(e) => handleDeleteClick(e, archive.date)}
                                            title="Delete day"
                                        >
                                            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
                                                <path d="M1 3h12M5 3V2h4v1M2 3l1 9h6l1-9H2zM5.5 6v4M8.5 6v4" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round"/>
                                            </svg>
                                        </button>
                                        <div className="archive-item-arrow">→</div>
                                    </div>
                                </>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
