export interface Session {
    id?: number;
    date: string;
    start_time: string;
    end_time?: string;
    status: "active" | "stopped";
}

export interface Interval {
    id?: number;
    session_id: number;
    interval_number: number;
    start_time: string;
    end_time?: string;
    words?: string;
    status: "pending" | "recorded" | "auto_away";
    recorded_at?: string;
}

export interface TimerState {
    session_id: number | null;
    current_interval_id: number | null;
    current_interval_number: number;
    interval_start_time: string | null;
    prompt_shown_time: string | null;
    is_running: boolean;
}

export interface TimelineData {
    interval_number: number;
    start_time: string;
    end_time?: string;
    words?: string;
    duration_minutes: number;
}

export interface ActivityData {
    words: string;
    total_minutes: number;
    percentage: number;
}

export interface DailyVisualizationData {
    total_sessions: number;
    total_minutes: number;
    words_entered: number;
    timeline_data: TimelineData[];
    activity_data: ActivityData[];
}

export interface DailyArchive {
    id?: number;
    date: string;
    total_sessions: number;
    total_minutes: number;
    visualization_data?: string; // JSON string of DailyVisualizationData
    archived_at?: string;
}
