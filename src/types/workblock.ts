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
