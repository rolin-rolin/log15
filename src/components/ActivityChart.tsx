import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from "recharts";
import type { ActivityData } from "../types/workblock";

interface ActivityChartProps {
    activityData: ActivityData[];
    title?: string;
}

const COLORS = [
    "#4a90e2", "#4caf50", "#ff9800", "#e91e63", "#9c27b0",
    "#00bcd4", "#ffc107", "#795548", "#607d8b", "#f44336"
];

export default function ActivityChart({ activityData, title = "Activity Breakdown" }: ActivityChartProps) {
    if (activityData.length === 0) {
        return (
            <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                <p>No activity data available</p>
            </div>
        );
    }

    const chartData = activityData.map((activity, index) => ({
        name: activity.words || "Unknown",
        minutes: activity.total_minutes,
        percentage: activity.percentage,
        color: COLORS[index % COLORS.length],
    }));

    return (
        <div style={{ marginTop: "20px" }}>
            <h3 style={{ marginBottom: "15px", fontSize: "18px", fontWeight: 600 }}>{title}</h3>
            <ResponsiveContainer width="100%" height={300}>
                <BarChart data={chartData} margin={{ top: 20, right: 30, left: 20, bottom: 60 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis 
                        dataKey="name" 
                        angle={-45} 
                        textAnchor="end" 
                        height={80}
                        interval={0}
                        style={{ fontSize: "12px" }}
                    />
                    <YAxis 
                        label={{ value: "Minutes", angle: -90, position: "insideLeft" }}
                        style={{ fontSize: "12px" }}
                    />
                    <Tooltip
                        formatter={(value: number, name: string) => {
                            if (name === "minutes") {
                                const activity = chartData.find((d) => d.minutes === value);
                                return [`${value} min (${activity?.percentage.toFixed(1)}%)`, "Time"];
                            }
                            return [value, name];
                        }}
                    />
                    <Bar dataKey="minutes" radius={[8, 8, 0, 0]}>
                        {chartData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={entry.color} />
                        ))}
                    </Bar>
                </BarChart>
            </ResponsiveContainer>
            <div style={{ marginTop: "15px", display: "flex", flexWrap: "wrap", gap: "10px" }}>
                {chartData.map((activity, index) => (
                    <div 
                        key={index} 
                        style={{ 
                            display: "flex", 
                            alignItems: "center", 
                            gap: "5px",
                            fontSize: "12px"
                        }}
                    >
                        <div 
                            style={{ 
                                width: "12px", 
                                height: "12px", 
                                backgroundColor: activity.color,
                                borderRadius: "2px"
                            }} 
                        />
                        <span>{activity.name}: {activity.minutes} min ({activity.percentage.toFixed(1)}%)</span>
                    </div>
                ))}
            </div>
        </div>
    );
}
