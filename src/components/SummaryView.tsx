import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
    DailyVisualizationData,
    DailyArchive,
} from "../types/workblock";
import TimelineChart from "./TimelineChart";
import ActivityChart from "./ActivityChart";
import WordFrequencyChart from "./WordFrequencyChart";
import "./SummaryView.css";

interface SummaryViewProps {
    onBack?: () => void;
    date?: string; // If provided, show archive for this date
}

export default function SummaryView({ onBack, date }: SummaryViewProps) {
    const [activeTab, setActiveTab] = useState<string>("aggregate");
    const [vizData, setVizData] = useState<DailyVisualizationData | null>(null);
    const [archivedData, setArchivedData] = useState<DailyArchive | null>(null);
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

            // Check if this is an archived date
            const archive = await invoke<DailyArchive | null>("get_archived_day_cmd", { date: targetDate });

            if (archive && archive.visualization_data) {
                // Load archived data
                setIsArchived(true);
                setArchivedData(archive);
                const parsed = JSON.parse(archive.visualization_data) as DailyVisualizationData;
                setVizData(parsed);

            } else {
                // Load current day data
                setIsArchived(false);
                const vizDataJson = await invoke<string>("get_daily_visualization_data_cmd", { date: targetDate });
                const parsed = JSON.parse(vizDataJson) as DailyVisualizationData;
                console.log("[SummaryView] Loaded visualization data:", {
                    workblocksCount: parsed.workblocks.length,
                    workblockIds: parsed.workblocks.map(wb => wb.id),
                    hasAggregate: parsed.daily_aggregate.total_workblocks > 0
                });
                setVizData(parsed);

                // Set active tab to first workblock if aggregate is empty
                if (parsed.workblocks.length > 0 && parsed.daily_aggregate.total_workblocks === 0) {
                    setActiveTab(`workblock-${parsed.workblocks[0].id}`);
                }
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
            const date = new Date(dateStr + "T00:00:00");
            return date.toLocaleDateString("en-US", {
                weekday: "long",
                year: "numeric",
                month: "long",
                day: "numeric",
            });
        } catch {
            return dateStr;
        }
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

    if (!vizData) {
        return (
            <div style={{ padding: "40px", textAlign: "center", color: "#666" }}>
                <p>No summary data available</p>
                <p style={{ fontSize: "14px", marginTop: "10px" }}>Start a workblock to generate summary data.</p>
            </div>
        );
    }

    const displayDate = date || (archivedData?.date ? archivedData.date : "");
    const hasWorkblocks = vizData.workblocks.length > 0;
    const hasAggregate = vizData.daily_aggregate.total_workblocks > 0;

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

            {/* Show stats based on active tab */}
            {activeTab === "aggregate" && hasAggregate && (
                <div className="summary-stats">
                    <div className="stat-item">
                        <div className="stat-value">{vizData.daily_aggregate.total_workblocks}</div>
                        <div className="stat-label">Workblocks</div>
                    </div>
                    <div className="stat-item">
                        <div className="stat-value">
                            {Math.floor(vizData.daily_aggregate.total_minutes / 60)}h{" "}
                            {vizData.daily_aggregate.total_minutes % 60}m
                        </div>
                        <div className="stat-label">Total Time</div>
                    </div>
                </div>
            )}
            
            {activeTab.startsWith("workblock-") && vizData && vizData.workblocks && (() => {
                const workblockId = parseInt(activeTab.replace("workblock-", ""));
                const workblock = vizData.workblocks.find((wb) => {
                    const wbId = typeof wb.id === 'string' ? parseInt(wb.id, 10) : wb.id;
                    return wbId === workblockId;
                });
                
                if (!workblock) return null;
                
                const formatDuration = (minutes: number | undefined) => {
                    if (minutes === undefined || minutes === null) return "0m";
                    const hours = Math.floor(minutes / 60);
                    const mins = minutes % 60;
                    if (hours > 0) {
                        return `${hours}h ${mins}m`;
                    }
                    return `${mins}m`;
                };
                
                const formatStatus = (status: string | undefined) => {
                    if (!status) return "Unknown";
                    return status.charAt(0).toUpperCase() + status.slice(1);
                };
                
                return (
                    <div className="summary-stats">
                        {workblock?.status === "cancelled" ? (
                            <>
                                <div className="stat-item">
                                    <div className="stat-value">
                                        {formatDuration(workblock?.duration_set_minutes ?? workblock?.duration_minutes)}
                                    </div>
                                    <div className="stat-label">Duration Set</div>
                                </div>
                                <div className="stat-item">
                                    <div className="stat-value">
                                        {formatDuration(workblock?.duration_worked_minutes ?? workblock?.duration_minutes)}
                                    </div>
                                    <div className="stat-label">Duration Worked</div>
                                </div>
                                <div className="stat-item">
                                    <div className="stat-value">{formatStatus(workblock?.status)}</div>
                                    <div className="stat-label">Status</div>
                                </div>
                            </>
                        ) : (
                            <>
                                <div className="stat-item">
                                    <div className="stat-value">{formatDuration(workblock?.duration_minutes)}</div>
                                    <div className="stat-label">Duration</div>
                                </div>
                                <div className="stat-item">
                                    <div className="stat-value">{formatStatus(workblock?.status)}</div>
                                    <div className="stat-label">Status</div>
                                </div>
                            </>
                        )}
                    </div>
                );
            })()}

            {hasWorkblocks && (
                <div className="tabs-container">
                    {hasAggregate && (
                        <button
                            className={`tab-button ${activeTab === "aggregate" ? "active" : ""}`}
                            onClick={() => setActiveTab("aggregate")}
                        >
                            Daily Aggregate
                        </button>
                    )}
                    {vizData.workblocks.map((wb, index) => (
                        <button
                            key={wb.id}
                            className={`tab-button ${activeTab === `workblock-${wb.id}` ? "active" : ""}`}
                            onClick={() => setActiveTab(`workblock-${wb.id}`)}
                        >
                            Workblock #{index + 1}
                        </button>
                    ))}
                </div>
            )}

            <div className="summary-content">
                {activeTab === "aggregate" && hasAggregate ? (
                    <div>
                        <TimelineChart
                            timelineData={vizData.daily_aggregate.timeline_data}
                            title="Daily Timeline"
                            workblockBoundaries={vizData.daily_aggregate.workblock_boundaries}
                        />
                        <ActivityChart
                            activityData={vizData.daily_aggregate.activity_data}
                            title="Daily Activity Breakdown"
                        />
                        <WordFrequencyChart
                            wordFrequency={vizData.daily_aggregate.word_frequency}
                            title="Daily Word Frequency"
                        />
                    </div>
                ) : activeTab.startsWith("workblock-") ? (
                    (() => {
                        if (!vizData || !vizData.workblocks) {
                            console.error("[SummaryView] No vizData or workblocks available");
                            return (
                                <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                                    <p>No visualization data available</p>
                                </div>
                            );
                        }

                        const workblockIdStr = activeTab.replace("workblock-", "");
                        const workblockId = parseInt(workblockIdStr, 10);
                        
                        console.log("[SummaryView] Rendering workblock view:", {
                            activeTab,
                            workblockIdStr,
                            workblockId,
                            workblocksCount: vizData.workblocks.length,
                            workblockIds: vizData.workblocks.map(wb => ({ id: wb.id, type: typeof wb.id }))
                        });
                        
                        if (isNaN(workblockId)) {
                            return (
                                <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                                    <p>Invalid workblock ID: {workblockIdStr}</p>
                                </div>
                            );
                        }
                        
                        const workblockIndex = vizData.workblocks.findIndex((wb) => {
                            // Handle both number and string IDs
                            const wbId = typeof wb.id === 'string' ? parseInt(wb.id, 10) : wb.id;
                            return wbId === workblockId;
                        });
                        const workblock = workblockIndex >= 0 ? vizData.workblocks[workblockIndex] : null;

                        console.log("[SummaryView] Workblock lookup result:", {
                            workblockIndex,
                            found: !!workblock,
                            workblock: workblock ? {
                                id: workblock.id,
                                hasTimeline: !!workblock.timeline_data,
                                hasActivity: !!workblock.activity_data,
                                hasWordFreq: !!workblock.word_frequency,
                                timelineLength: workblock.timeline_data?.length || 0,
                                activityLength: workblock.activity_data?.length || 0,
                                wordFreqLength: workblock.word_frequency?.length || 0
                            } : null
                        });

                        if (!workblock) {
                            return (
                                <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                                    <p>Workblock not found (ID: {workblockId})</p>
                                    <p style={{ fontSize: "12px", marginTop: "10px" }}>
                                        Available IDs: {vizData.workblocks.map(wb => wb.id).join(", ")}
                                    </p>
                                </div>
                            );
                        }

                        const workblockNumber = workblockIndex + 1;

                        // Always render charts, even if empty - let the chart components handle empty data
                        return (
                            <div>
                                <TimelineChart
                                    timelineData={workblock.timeline_data || []}
                                    title={`Workblock #${workblockNumber} Timeline`}
                                />
                                <ActivityChart
                                    activityData={workblock.activity_data || []}
                                    title={`Workblock #${workblockNumber} Activity Breakdown`}
                                />
                                <WordFrequencyChart
                                    wordFrequency={workblock.word_frequency || []}
                                    title={`Workblock #${workblockNumber} Word Frequency`}
                                />
                            </div>
                        );
                    })()
                ) : (
                    <div style={{ padding: "40px", textAlign: "center", color: "#666" }}>
                        <p>No workblock data available</p>
                    </div>
                )}
            </div>
        </div>
    );
}
