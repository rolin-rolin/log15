// Prompt page for the overlay window
import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import PromptWindow from "../components/PromptWindow";
import "../components/PromptWindow.css";

export default function PromptPage() {
    const getInitialIntervalId = (): number | null => {
        const hash = window.location.hash;
        const hashMatch = hash.match(/[?&]intervalId=(\d+)/);
        if (hashMatch) {
            const parsedId = parseInt(hashMatch[1], 10);
            return isNaN(parsedId) ? null : parsedId;
        }
        return null;
    };

    const [intervalId, setIntervalId] = useState<number | null>(getInitialIntervalId);

    useEffect(() => {
        const setupListeners = async () => {
            const unlisten = await listen<number>("prompt-interval-id", (event) => {
                setIntervalId(event.payload);
            });
            return unlisten;
        };

        let unlistenPromise: Promise<() => void> | null = null;
        setupListeners().then((unlisten) => {
            unlistenPromise = Promise.resolve(unlisten);
        });

        const unlistenHide = listen("prompt-hide", () => {
            setTimeout(() => setIntervalId(null), 300);
        });

        return () => {
            unlistenPromise?.then((fn) => fn());
            unlistenHide.then((fn) => fn());
        };
    }, []);

    return (
        <div className="prompt-window-root">
            <PromptWindow intervalId={intervalId} />
        </div>
    );
}
