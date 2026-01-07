import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import PromptPage from "./pages/PromptPage";
import WorkblockControl from "./components/WorkblockControl";
import "./App.css";

function App() {
    const [currentView, setCurrentView] = useState<string>("main");

    useEffect(() => {
        // Check if we're in the prompt window (check URL hash)
        const hash = window.location.hash;
        if (hash === "#/prompt") {
            setCurrentView("prompt");
            return;
        }

        // Listen for interval-complete event to show prompt window
        const setupListeners = async () => {
            const unlisten = await listen("interval-complete", async (event: any) => {
                const payload = event.payload as { interval_id?: number };
                if (payload.interval_id) {
                    try {
                        await invoke("show_prompt_window_cmd", { intervalId: payload.interval_id });
                    } catch (error) {
                        console.error("Failed to show prompt window:", error);
                    }
                }
            });

            return unlisten;
        };

        let unlistenPromise: Promise<() => void> | null = null;
        setupListeners().then((unlisten) => {
            unlistenPromise = Promise.resolve(unlisten);
        });

        // Listen for tray navigation events
        const unlistenStart = listen("tray-start-workblock", () => {
            setCurrentView("main");
        });

        const unlistenSummary = listen("tray-view-summary", () => {
            setCurrentView("main");
            // TODO: Navigate to summary view
        });

        const unlistenLastWords = listen("tray-view-last-words", () => {
            setCurrentView("main");
            // TODO: Show last words
        });

        return () => {
            unlistenPromise?.then((fn) => fn());
            unlistenStart.then((fn) => fn());
            unlistenSummary.then((fn) => fn());
            unlistenLastWords.then((fn) => fn());
        };
    }, []);

    // Check URL hash on mount
    useEffect(() => {
        const hash = window.location.hash;
        if (hash === "#/prompt") {
            setCurrentView("prompt");
        }
    }, []);

    if (currentView === "prompt") {
        return <PromptPage />;
    }

    return (
        <main className="container">
            <WorkblockControl />
        </main>
    );
}

export default App;
