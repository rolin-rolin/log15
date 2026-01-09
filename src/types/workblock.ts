// Type definitions for workblock-related data structures

export interface Workblock {
    id?: number;
    date: string;
    start_time: string;
    end_time?: string;
    duration_minutes?: number;
    status: "active" | "completed" | "cancelled";
    is_archived?: boolean;
    created_at?: string;
}

export interface Interval {
    id?: number;
    workblock_id: number;
    interval_number: number;
    start_time: string;
    end_time?: string;
    words?: string;
    status: "pending" | "recorded" | "auto_away";
    recorded_at?: string;
}

export interface TimerState {
    workblock_id: number | null;
    current_interval_id: number | null;
    current_interval_number: number;
    interval_start_time: string | null;
    prompt_shown_time: string | null;
    is_running: boolean;
}

// Visualization data types
export interface TimelineData {
    interval_number: number;
    start_time: string;
    end_time?: string;
    words?: string;
    duration_minutes: number;
    workblock_status?: string;
}

export interface AggregateTimelineData {
    workblock_id: number;
    interval_number: number;
    start_time: string;
    end_time?: string;
    words?: string;
    duration_minutes: number;
    workblock_status?: string;
}

export interface WorkblockBoundary {
    id: number;
    start_time: string;
    end_time?: string;
    status: "active" | "completed" | "cancelled";
}

export interface ActivityData {
    words: string;
    total_minutes: number;
    percentage: number;
}

export interface WordFrequency {
    word: string;
    count: number;
}

export interface WorkblockVisualization {
    id: number;
    timeline_data: TimelineData[];
    activity_data: ActivityData[];
    word_frequency: WordFrequency[];
}

export interface DailyAggregate {
    total_workblocks: number;
    total_minutes: number;
    timeline_data: AggregateTimelineData[];
    activity_data: ActivityData[];
    word_frequency: WordFrequency[];
    workblock_boundaries?: WorkblockBoundary[]; // Optional for backward compatibility with old archived data
}

export interface DailyVisualizationData {
    workblocks: WorkblockVisualization[];
    daily_aggregate: DailyAggregate;
}

export interface DailyArchive {
    id?: number;
    date: string;
    total_workblocks: number;
    total_minutes: number;
    visualization_data?: string; // JSON string
    archived_at?: string;
}
