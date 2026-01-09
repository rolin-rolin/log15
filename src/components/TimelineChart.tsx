import type { TimelineData, AggregateTimelineData, WorkblockBoundary } from "../types/workblock";

interface TimelineChartProps {
    timelineData: TimelineData[] | AggregateTimelineData[];
    title?: string;
    workblockBoundaries?: WorkblockBoundary[];
}

export default function TimelineChart({ timelineData, title = "Timeline", workblockBoundaries }: TimelineChartProps) {
    if (timelineData.length === 0) {
        return (
            <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                <p>No timeline data available</p>
            </div>
        );
    }

    // Helper function to get workblock number from boundaries array
    const getWorkblockNumber = (workblockId: number): number | null => {
        if (!workblockBoundaries) return null;
        const index = workblockBoundaries.findIndex((wb) => wb.id === workblockId);
        return index >= 0 ? index + 1 : null;
    };

    // Helper function to format boundary label
    const formatBoundaryLabel = (endingWorkblockId: number | null, startingWorkblockId: number | null): string => {
        if (!workblockBoundaries || !startingWorkblockId) return "";

        const endingNum = endingWorkblockId ? getWorkblockNumber(endingWorkblockId) : null;
        const startingNum = getWorkblockNumber(startingWorkblockId);
        const endingBoundary = endingWorkblockId ? workblockBoundaries.find((wb) => wb.id === endingWorkblockId) : null;

        if (endingNum && startingNum) {
            const endingStatus = endingBoundary?.status === "cancelled" ? " (Cancelled)" : "";
            return `Workblock #${endingNum}${endingStatus} End / Workblock #${startingNum} Start`;
        } else if (startingNum) {
            return `Workblock #${startingNum} Start`;
        }
        return "";
    };

    // Build the display items (intervals + boundaries)
    const displayItems: Array<{ type: "interval" | "boundary"; data: any; index?: number }> = [];

    // Only show boundaries if we have workblockBoundaries and this is aggregate timeline data
    const shouldShowBoundaries =
        workblockBoundaries &&
        workblockBoundaries.length > 1 &&
        timelineData.length > 0 &&
        "workblock_id" in timelineData[0];

    let previousWorkblockId: number | null = null;
    let isFirstInterval = true;

    timelineData.forEach((interval, index) => {
        // Check if this is an AggregateTimelineData (has workblock_id)
        const currentWorkblockId =
            "workblock_id" in interval && typeof interval.workblock_id === "number" ? interval.workblock_id : null;

        // Show boundary for first workblock start (only if it's truly the first interval)
        if (shouldShowBoundaries && isFirstInterval && currentWorkblockId !== null) {
            const firstBoundary = workblockBoundaries!.find((wb) => wb.id === currentWorkblockId);
            if (firstBoundary) {
                // Check if this interval's start_time matches or is very close to the workblock's start_time
                // (allow small difference due to timing precision)
                const intervalStart = new Date(interval.start_time).getTime();
                const workblockStart = new Date(firstBoundary.start_time).getTime();
                const timeDiff = Math.abs(intervalStart - workblockStart);

                // If within 1 second, consider it the start
                if (timeDiff < 1000) {
                    const workblockNum = getWorkblockNumber(currentWorkblockId);
                    if (workblockNum) {
                        displayItems.push({
                            type: "boundary",
                            data: {
                                label: `Workblock #${workblockNum} Start`,
                            },
                        });
                    }
                }
            }
            isFirstInterval = false;
        }

        // If workblock_id changed, add a boundary divider
        if (
            shouldShowBoundaries &&
            currentWorkblockId !== null &&
            previousWorkblockId !== null &&
            currentWorkblockId !== previousWorkblockId
        ) {
            displayItems.push({
                type: "boundary",
                data: {
                    label: formatBoundaryLabel(previousWorkblockId, currentWorkblockId),
                },
            });
        }

        // Add the interval
        displayItems.push({
            type: "interval",
            data: interval,
            index,
        });

        previousWorkblockId = currentWorkblockId;
    });

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
                {displayItems.map((item, displayIndex) => {
                    if (item.type === "boundary") {
                        // Render boundary divider
                        return (
                            <div
                                key={`boundary-${displayIndex}`}
                                style={{
                                    margin: "12px 0",
                                    padding: "8px 0",
                                    borderTop: "1px solid #666",
                                    borderBottom: "1px solid #666",
                                    textAlign: "center",
                                }}
                            >
                                <span
                                    style={{
                                        fontSize: "11px",
                                        color: "#888",
                                        fontStyle: "italic",
                                        fontWeight: 500,
                                    }}
                                >
                                    ━━━ {item.data.label} ━━━
                                </span>
                            </div>
                        );
                    } else {
                        // Render interval
                        const interval = item.data;
                        const formattedTime = new Date(interval.start_time).toLocaleTimeString([], {
                            hour: "2-digit",
                            minute: "2-digit",
                        });

                        // If this interval is cancelled, show "cancelled" as the words
                        const displayWords =
                            interval.workblock_status === "cancelled" ? "cancelled" : interval.words || "Pending";

                        return (
                            <div
                                key={item.index ?? displayIndex}
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
                    }
                })}
            </div>
        </div>
    );
}
