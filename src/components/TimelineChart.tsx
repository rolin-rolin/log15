import type { TimelineData, AggregateTimelineData } from "../types/workblock";

interface TimelineChartProps {
    timelineData: TimelineData[] | AggregateTimelineData[];
    title?: string;
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
            <div
                style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "8px",
                    marginTop: "10px",
                }}
            >
                {timelineData.map((interval, index) => {
                    const formattedTime = new Date(interval.start_time).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                    });

                    // If this interval is cancelled, show "cancelled" as the words
                    const displayWords =
                        interval.workblock_status === "cancelled" ? "cancelled" : interval.words || "Pending";

                    return (
                        <div
                            key={index}
                            style={{
                                display: "grid",
                                gridTemplateColumns: "120px 1fr",
                                gap: "16px",
                                alignItems: "center",
                            }}
                        >
                            <div
                                style={{
                                    fontSize: "12px",
                                    color: "white",
                                    textAlign: "right",
                                }}
                            >
                                {formattedTime}
                            </div>
                            <div
                                style={{
                                    fontSize: "12px",
                                    color: "white",
                                    fontWeight: interval.workblock_status === "cancelled" ? 600 : 400,
                                }}
                            >
                                {displayWords}
                            </div>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
