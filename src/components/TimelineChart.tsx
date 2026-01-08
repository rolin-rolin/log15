import { useMemo } from "react";
import type { TimelineData, AggregateTimelineData } from "../types/workblock";

interface TimelineChartProps {
    timelineData: TimelineData[] | AggregateTimelineData[];
    title?: string;
}

// Color palette for activities
const COLORS = [
    "#4a90e2", "#4caf50", "#ff9800", "#e91e63", "#9c27b0",
    "#00bcd4", "#ffc107", "#795548", "#607d8b", "#f44336"
];

export default function TimelineChart({ timelineData, title = "Timeline" }: TimelineChartProps) {
    const coloredIntervals = useMemo(() => {
        // Group intervals by words to assign consistent colors
        const wordColorMap = new Map<string, string>();
        let colorIndex = 0;

        return timelineData.map((interval) => {
            const words = interval.words?.toLowerCase() || "pending";
            if (!wordColorMap.has(words)) {
                wordColorMap.set(words, COLORS[colorIndex % COLORS.length]);
                colorIndex++;
            }
            return {
                ...interval,
                color: wordColorMap.get(words) || "#cccccc",
            };
        });
    }, [timelineData]);

    const totalMinutes = useMemo(() => {
        return coloredIntervals.reduce((sum, interval) => sum + interval.duration_minutes, 0);
    }, [coloredIntervals]);

    if (timelineData.length === 0) {
        return (
            <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                <p>No timeline data available</p>
            </div>
        );
    }

    return (
        <div style={{ marginTop: "20px" }}>
            <h3 style={{ marginBottom: "15px", fontSize: "18px", fontWeight: 600 }}>{title}</h3>
            <div style={{ 
                display: "flex", 
                flexDirection: "column", 
                gap: "8px",
                marginTop: "10px"
            }}>
                {coloredIntervals.map((interval, index) => {
                    const widthPercent = (interval.duration_minutes / totalMinutes) * 100;
                    const formattedTime = new Date(interval.start_time).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                    });

                    return (
                        <div key={index} style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                            <div style={{ 
                                minWidth: "100px", 
                                fontSize: "12px", 
                                color: "#666",
                                textAlign: "right"
                            }}>
                                {formattedTime}
                            </div>
                            <div style={{ 
                                flex: 1, 
                                height: "30px", 
                                backgroundColor: interval.color,
                                borderRadius: "4px",
                                display: "flex",
                                alignItems: "center",
                                padding: "0 10px",
                                color: "white",
                                fontSize: "12px",
                                fontWeight: 500,
                                position: "relative",
                                minWidth: `${Math.max(widthPercent, 5)}%`
                            }}>
                                <span>{interval.words || "Pending"}</span>
                                <span style={{ marginLeft: "auto", fontSize: "11px" }}>
                                    {interval.duration_minutes}m
                                </span>
                            </div>
                        </div>
                    );
                })}
            </div>
            <div style={{ 
                marginTop: "15px", 
                fontSize: "12px", 
                color: "#666",
                textAlign: "right"
            }}>
                Total: {totalMinutes} minutes
            </div>
        </div>
    );
}
