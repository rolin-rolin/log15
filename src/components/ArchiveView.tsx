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
            // #region agent log
            fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ArchiveView.tsx:19',message:'loadArchivedDates entry',data:{},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'H4'})}).catch(()=>{});
            // #endregion
            const dates = await invoke<DailyArchive[]>("get_all_archived_dates_cmd");
            // #region agent log
            fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ArchiveView.tsx:21',message:'invoke result received',data:{count:dates.length,dates:dates.map(d=>({date:d.date,id:d.id,hasViz:!!d.visualization_data}))},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'H4'})}).catch(()=>{});
            // #endregion
            setArchivedDates(dates);
            // #region agent log
            fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ArchiveView.tsx:23',message:'setArchivedDates called',data:{count:dates.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'H4'})}).catch(()=>{});
            // #endregion
        } catch (error) {
            console.error("Failed to load archived dates:", error);
            // #region agent log
            fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ArchiveView.tsx:25',message:'loadArchivedDates error',data:{error:String(error)},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'H4'})}).catch(()=>{});
            // #endregion
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
                    {/* #region agent log */}
                    {(() => {
                        fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ArchiveView.tsx:72',message:'rendering archive list',data:{count:archivedDates.length,dates:archivedDates.map(d=>d.date)},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'H4'})}).catch(()=>{});
                        return null;
                    })()}
                    {/* #endregion */}
                    {archivedDates.map((archive) => (
                        <div key={archive.date} className="archive-item" onClick={() => setSelectedDate(archive.date)}>
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
                            <div className="archive-item-arrow">→</div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
