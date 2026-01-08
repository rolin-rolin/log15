import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from "recharts";
import type { WordFrequency } from "../types/workblock";

interface WordFrequencyChartProps {
    wordFrequency: WordFrequency[];
    title?: string;
}

const COLORS = [
    "#4a90e2", "#4caf50", "#ff9800", "#e91e63", "#9c27b0",
    "#00bcd4", "#ffc107", "#795548", "#607d8b", "#f44336"
];

export default function WordFrequencyChart({ wordFrequency, title = "Word Frequency" }: WordFrequencyChartProps) {
    if (wordFrequency.length === 0) {
        return (
            <div style={{ padding: "20px", textAlign: "center", color: "#666" }}>
                <p>No word frequency data available</p>
            </div>
        );
    }

    // Sort by count (descending) and take top 15
    const sortedData = [...wordFrequency]
        .sort((a, b) => b.count - a.count)
        .slice(0, 15)
        .map((item, index) => ({
            word: item.word,
            count: item.count,
            color: COLORS[index % COLORS.length],
        }));

    return (
        <div style={{ marginTop: "20px" }}>
            <h3 style={{ marginBottom: "15px", fontSize: "18px", fontWeight: 600 }}>{title}</h3>
            <ResponsiveContainer width="100%" height={300}>
                <BarChart data={sortedData} margin={{ top: 20, right: 30, left: 20, bottom: 60 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis 
                        dataKey="word" 
                        angle={-45} 
                        textAnchor="end" 
                        height={80}
                        interval={0}
                        style={{ fontSize: "12px" }}
                    />
                    <YAxis 
                        label={{ value: "Count", angle: -90, position: "insideLeft" }}
                        style={{ fontSize: "12px" }}
                    />
                    <Tooltip
                        formatter={(value: number) => [`${value} times`, "Frequency"]}
                    />
                    <Bar dataKey="count" radius={[8, 8, 0, 0]}>
                        {sortedData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={entry.color} />
                        ))}
                    </Bar>
                </BarChart>
            </ResponsiveContainer>
            <div style={{ marginTop: "15px", fontSize: "12px", color: "#666" }}>
                Showing top {sortedData.length} most frequent words
            </div>
        </div>
    );
}
