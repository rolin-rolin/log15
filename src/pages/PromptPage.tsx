// Prompt page for the overlay window
import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import PromptWindow from "../components/PromptWindow";
import "../components/PromptWindow.css";

export default function PromptPage() {
    const [intervalId, setIntervalId] = useState<number | null>(null);

    useEffect(() => {
        console.log("[PROMPT_PAGE] Component mounted! Setting up event listeners");
        console.log("[PROMPT_PAGE] Current URL:", window.location.href);
        console.log("[PROMPT_PAGE] Current hash:", window.location.hash);

        // Listen for interval ID from backend
        const setupListeners = async () => {
            const unlisten = await listen<number>("prompt-interval-id", (event) => {
                console.log("[PROMPT_PAGE] Received prompt-interval-id event:", event.payload);
                setIntervalId(event.payload);
            });
            return unlisten;
        };

        let unlistenPromise: Promise<() => void> | null = null;
        setupListeners().then((unlisten) => {
            unlistenPromise = Promise.resolve(unlisten);
        });

        // Listen for hide event
        const unlistenHide = listen("prompt-hide", () => {
            console.log("[PROMPT_PAGE] Received prompt-hide event");
            // Trigger fade-out, then hide
            setTimeout(() => {
                setIntervalId(null);
            }, 300);
        });

        return () => {
            unlistenPromise?.then((fn) => fn());
            unlistenHide.then((fn) => fn());
        };
    }, []);

    return (
        <div
            style={{
                width: "100vw",
                height: "100vh",
                margin: 0,
                padding: 0,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                backgroundColor: "transparent",
            }}
        >
            <PromptWindow intervalId={intervalId} />
        </div>
    );
}
