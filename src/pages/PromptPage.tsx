// Prompt page for the overlay window
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import PromptWindow from "../components/PromptWindow";
import "../components/PromptWindow.css";

export default function PromptPage() {
    const [intervalId, setIntervalId] = useState<number | null>(null);

    useEffect(() => {
        // Listen for interval ID from backend
        const unlisten = listen<number>("prompt-interval-id", (event) => {
            setIntervalId(event.payload);
        });

        // Listen for hide event
        const unlistenHide = listen("prompt-hide", () => {
            // Trigger fade-out, then hide
            setTimeout(() => {
                setIntervalId(null);
            }, 300);
        });

        return () => {
            unlisten.then((fn) => fn());
            unlistenHide.then((fn) => fn());
        };
    }, []);

    return (
        <div style={{ width: "100vw", height: "100vh", margin: 0, padding: 0 }}>
            <PromptWindow intervalId={intervalId} />
        </div>
    );
}
