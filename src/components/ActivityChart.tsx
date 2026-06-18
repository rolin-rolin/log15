import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from "recharts";
import type { ActivityData } from "../types/session";

interface ActivityChartProps {
    activityData: ActivityData[];
    title?: string;
}

const COLORS = [
    "#4a90e2",
    "#4caf50",
    "#ff9800",
    "#e91e63",
    "#9c27b0",
    "#00bcd4",
    "#ffc107",
    "#795548",
    "#607d8b",
    "#f44336",
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
        value: activity.total_minutes,
        percentage: activity.percentage,
        color: COLORS[index % COLORS.length],
    }));

    // Verify percentage calculations sum to ~100%
    const totalPercentage = chartData.reduce((sum, item) => sum + item.percentage, 0);
    const totalMinutes = chartData.reduce((sum, item) => sum + item.value, 0);
    
    // Log for debugging
    console.log(`[ActivityChart] ${title}:`, {
        totalMinutes,
        totalPercentage: totalPercentage.toFixed(2) + "%",
        items: chartData.map(item => ({
            name: item.name,
            minutes: item.value,
            percentage: item.percentage.toFixed(2) + "%",
        })),
    });

    const renderCustomLabel = ({ cx, cy, midAngle, innerRadius, outerRadius, percentage }: any) => {
        const RADIAN = Math.PI / 180;
        const radius = innerRadius + (outerRadius - innerRadius) * 0.5;
        const x = cx + radius * Math.cos(-midAngle * RADIAN);
        const y = cy + radius * Math.sin(-midAngle * RADIAN);

        // Only show label if slice is large enough (>5%)
        if (percentage < 5) return null;

        return (
            <text
                x={x}
                y={y}
                fill="white"
                textAnchor={x > cx ? "start" : "end"}
                dominantBaseline="central"
                style={{ fontSize: "12px", fontWeight: 500 }}
            >
                {`${percentage.toFixed(0)}%`}
            </text>
        );
    };

    return (
        <div style={{ marginTop: "20px" }}>
            <h3 style={{ marginBottom: "15px", fontSize: "18px", fontWeight: 600 }}>{title}</h3>
            <ResponsiveContainer width="100%" height={350}>
                <PieChart>
                    <Pie
                        data={chartData}
                        cx="50%"
                        cy="50%"
                        labelLine={false}
                        label={renderCustomLabel}
                        outerRadius={100}
                        fill="#8884d8"
                        dataKey="value"
                    >
                        {chartData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={entry.color} />
                        ))}
                    </Pie>
                    <Tooltip
                        formatter={(value: number | undefined, _name?: string, props?: any) => {
                            if (value === undefined || !props?.payload) return ["", ""];
                            const data = props.payload;
                            return [
                                `${data.name}: ${value} min (${data.percentage.toFixed(1)}%)`,
                                "Time",
                            ];
                        }}
                        contentStyle={{
                            backgroundColor: "#ffffff",
                            border: "1px solid #e5e7eb",
                            borderRadius: "4px",
                            color: "#000000",
                            boxShadow: "0 8px 24px rgba(0,0,0,0.12)",
                        }}
                        labelStyle={{ color: "#000000" }}
                        itemStyle={{ color: "#000000" }}
                    />
                </PieChart>
            </ResponsiveContainer>
            <div style={{ marginTop: "15px", display: "flex", flexWrap: "wrap", gap: "10px", justifyContent: "center" }}>
                {chartData.map((activity, index) => (
                    <div
                        key={index}
                        style={{
                            display: "flex",
                            alignItems: "center",
                            gap: "5px",
                            fontSize: "12px",
                        }}
                    >
                        <div
                            style={{
                                width: "12px",
                                height: "12px",
                                backgroundColor: activity.color,
                                borderRadius: "2px",
                            }}
                        />
                        <span>
                            {activity.name}: {activity.value} min ({activity.percentage.toFixed(1)}%)
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
}
