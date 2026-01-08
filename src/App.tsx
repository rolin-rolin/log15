import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import PromptPage from "./pages/PromptPage";
import WorkblockControl from "./components/WorkblockControl";
import SummaryView from "./components/SummaryView";
import ArchiveView from "./components/ArchiveView";
import "./App.css";

function App() {
    const [currentView, setCurrentView] = useState<string>("main");
    // Use a ref to track handled intervals across renders
    const handledIntervalsRef = useRef<Set<number>>(new Set());

    // Check window type on mount (for prompt window detection)
    useEffect(() => {
        let unlistenPromise: Promise<() => void> | null = null;
        let unlistenStart: Promise<() => void> | null = null;
        let unlistenSummary: Promise<() => void> | null = null;
        let unlistenLastWords: Promise<() => void> | null = null;

        const checkWindowType = async () => {
            try {
                const { getCurrentWindow } = await import("@tauri-apps/api/window");
                const currentWindow = getCurrentWindow();
                const label = currentWindow.label;
                console.log("[APP] Window type check - label:", label, "hash:", window.location.hash);

                if (label === "prompt") {
                    console.log("[APP] Detected prompt window via label, switching to prompt view");
                    setCurrentView("prompt");
                    return;
                }
            } catch (error) {
                console.error("[APP] Error in window type check:", error);
            }

            // Fallback: check hash
            const hash = window.location.hash;
            if (hash === "#/prompt" || hash === "/prompt") {
                console.log("[APP] Detected prompt window via hash");
                setCurrentView("prompt");
                return;
            }

            // If we get here, we're in the main window
            console.log("[APP] Main window detected, setting up listeners");

            // Listen for interval-complete event to show prompt window
            const setupListeners = async () => {
                const unlisten = await listen("interval-complete", async (event: any) => {
                    console.log("[FRONTEND] Received interval-complete event:", event.payload);
                    const payload = event.payload as { interval_id?: number; interval_number?: number };
                    if (payload.interval_id) {
                        // Prevent duplicate handling of the same interval
                        if (handledIntervalsRef.current.has(payload.interval_id)) {
                            console.log("[FRONTEND] Already handled interval_id:", payload.interval_id, "skipping");
                            return;
                        }
                        handledIntervalsRef.current.add(payload.interval_id);

                        console.log("[FRONTEND] Calling show_prompt_window_cmd with intervalId:", payload.interval_id);
                        try {
                            await invoke("show_prompt_window_cmd", { intervalId: payload.interval_id });
                            console.log("[FRONTEND] show_prompt_window_cmd succeeded");
                        } catch (error) {
                            console.error("[FRONTEND] Failed to show prompt window:", error);
                            // Remove from set on error so we can retry
                            handledIntervalsRef.current.delete(payload.interval_id);
                        }
                    } else {
                        console.warn("[FRONTEND] interval-complete event missing interval_id");
                    }
                });

                return unlisten;
            };

            setupListeners().then((unlisten) => {
                unlistenPromise = Promise.resolve(unlisten);
            });

            // Listen for tray navigation events
            unlistenStart = listen("tray-start-workblock", () => {
                setCurrentView("main");
            });

            unlistenSummary = listen("tray-view-summary", () => {
                setCurrentView("summary");
            });

            unlistenLastWords = listen("tray-view-last-words", () => {
                setCurrentView("main");
                // TODO: Show last words
            });
        };

        // Check immediately
        checkWindowType();

        // Also check after delays (in case React hasn't fully mounted)
        const timeoutId = setTimeout(() => {
            checkWindowType();
        }, 100);
        const timeoutId2 = setTimeout(() => {
            checkWindowType();
        }, 500);

        return () => {
            clearTimeout(timeoutId);
            clearTimeout(timeoutId2);
            unlistenPromise?.then((fn) => fn());
            unlistenStart?.then((fn) => fn());
            unlistenSummary?.then((fn) => fn());
            unlistenLastWords?.then((fn) => fn());
        };
    }, []);

    if (currentView === "prompt") {
        return <PromptPage />;
    }

    if (currentView === "summary") {
        return (
            <main className="container">
                <SummaryView onBack={() => setCurrentView("main")} />
            </main>
        );
    }

    if (currentView === "archive") {
        return (
            <main className="container">
                <ArchiveView onBack={() => setCurrentView("main")} />
            </main>
        );
    }

    return (
        <main className="container">
            <WorkblockControl 
                onNavigateToSummary={() => setCurrentView("summary")} 
                onNavigateToArchive={() => setCurrentView("archive")}
            />
        </main>
    );
}

export default App;
