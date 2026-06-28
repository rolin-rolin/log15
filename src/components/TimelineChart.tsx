import type { TimelineData } from "../types/session";

interface TimelineChartProps {
    timelineData: TimelineData[];
    title?: string;
}

function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export default function TimelineChart({ timelineData, title = "Timeline" }: TimelineChartProps) {
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
            <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginTop: "10px" }}>
                {timelineData.map((interval, index) => {
                    const start = formatTime(interval.start_time);
                    const end = interval.end_time ? formatTime(interval.end_time) : null;
                    const timeLabel = end ? `${start} – ${end}` : start;

                    return (
                        <div
                            key={index}
                            style={{
                                display: "grid",
                                gridTemplateColumns: "160px 1fr",
                                gap: "16px",
                                alignItems: "center",
                            }}
                        >
                            <div style={{ fontSize: "12px", color: "white", textAlign: "right" }}>
                                {timeLabel}
                            </div>
                            <div style={{ fontSize: "12px", color: "white" }}>
                                {interval.words ?? ""}
                            </div>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
