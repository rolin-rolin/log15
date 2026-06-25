import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import PromptPage from "./pages/PromptPage";
import SummaryReadyPage from "./pages/SummaryReadyPage";
import SessionControl from "./components/SessionControl";
import SummaryView from "./components/SummaryView";
import ArchiveView from "./components/ArchiveView";
import type { Session } from "./types/session";
import "./App.css";

function App() {
    const [currentView, setCurrentView] = useState<string>("main");
    const [activeSession, setActiveSession] = useState<Session | null | undefined>(undefined);
    const handledIntervalsRef = useRef<Set<number>>(new Set());

    useEffect(() => {
        invoke<Session | null>("get_active_session_cmd")
            .then(setActiveSession)
            .catch(() => setActiveSession(null));
    }, []);

    useEffect(() => {
        let unlistenPromise: Promise<() => void> | null = null;
        let unlistenStart: Promise<() => void> | null = null;
        let unlistenSummary: Promise<() => void> | null = null;
        let unlistenLastWords: Promise<() => void> | null = null;

        const checkWindowType = async () => {
            try {
                const { getCurrentWindow } = await import("@tauri-apps/api/window");
                const label = getCurrentWindow().label;

                if (label === "prompt") {
                    const hash = window.location.hash;
                    setCurrentView(hash.includes("summary-ready") ? "summary-ready" : "prompt");
                    return;
                }
            } catch (error) {
                console.error("Error in window type check:", error);
            }

            const hash = window.location.hash;
            if (hash.startsWith("#/prompt") || hash.startsWith("/prompt")) {
                setCurrentView("prompt");
                return;
            }
            if (hash.startsWith("#/summary-ready") || hash.startsWith("/summary-ready")) {
                setCurrentView("summary-ready");
                return;
            }

            // Main window — set up listeners
            const setupListeners = async () => {
                const unlisten = await listen("interval-complete", async (event: any) => {
                    const payload = event.payload as { interval_id?: number };
                    if (!payload.interval_id) return;
                    if (handledIntervalsRef.current.has(payload.interval_id)) return;
                    handledIntervalsRef.current.add(payload.interval_id);
                    try {
                        await invoke("show_prompt_window_cmd", { intervalId: payload.interval_id });
                    } catch (error) {
                        console.error("Failed to show prompt window:", error);
                        handledIntervalsRef.current.delete(payload.interval_id);
                    }
                });
                return unlisten;
            };

            setupListeners().then((unlisten) => {
                unlistenPromise = Promise.resolve(unlisten);
            });

            unlistenStart = listen("tray-start-session", () => setCurrentView("main"));
            unlistenSummary = listen("tray-view-summary", () => setCurrentView("summary"));
            unlistenLastWords = listen("tray-view-last-words", () => setCurrentView("main"));
        };

        checkWindowType();

        return () => {
            unlistenPromise?.then((fn) => fn());
            unlistenStart?.then((fn) => fn());
            unlistenSummary?.then((fn) => fn());
            unlistenLastWords?.then((fn) => fn());
        };
    }, []);

    if (currentView === "prompt") return <PromptPage />;
    if (currentView === "summary-ready") return <SummaryReadyPage />;

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

    if (activeSession === undefined) return null;

    return (
        <main className="container">
            <SessionControl
                activeSession={activeSession}
                onSessionStart={setActiveSession}
                onSessionStop={() => setActiveSession(null)}
                onNavigateToSummary={() => setCurrentView("summary")}
                onNavigateToArchive={() => setCurrentView("archive")}
            />
        </main>
    );
}

export default App;
