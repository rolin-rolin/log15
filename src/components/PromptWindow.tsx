import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./PromptWindow.css";

interface PromptWindowProps {
    intervalId: number | null;
}

const CHECKMARK_DURATION_MS = 2000; // 2 seconds

export default function PromptWindow({ intervalId }: PromptWindowProps) {
    const [words, setWords] = useState("");
    const [showCheckmark, setShowCheckmark] = useState(false);
    const [isVisible, setIsVisible] = useState(false);
    const [showSummaryReady, setShowSummaryReady] = useState(false);

    useEffect(() => {
        console.log("[PROMPT_WINDOW] intervalId changed:", intervalId);
        if (intervalId) {
            console.log("[PROMPT_WINDOW] Setting isVisible to true");
            setIsVisible(true);
            // Reset state when new interval comes in
            setShowCheckmark(false);
            setShowSummaryReady(false);
            setWords("");
        }
    }, [intervalId]);

    useEffect(() => {
        // Listen for hide event
        const unlisten = listen("prompt-hide", () => {
            console.log("[PROMPT_WINDOW] Received prompt-hide event");
            handleFadeOut();
        });

        // Listen for auto-away event (window should close)
        const unlistenAutoAway = listen("auto-away", () => {
            console.log("[PROMPT_WINDOW] Received auto-away event, closing window");
            handleFadeOut();
            // Also call hide command to ensure window is closed
            invoke("hide_prompt_window_cmd").catch(console.error);
        });

        // Listen for show summary ready event
        const unlistenSummary = listen("show-summary-ready", () => {
            setShowSummaryReady(true);
            setShowCheckmark(false);
            setWords("");
        });

        // Listen for close summary event
        const unlistenClose = listen("close-summary", () => {
            handleFadeOut();
        });

        return () => {
            unlisten.then((fn) => fn());
            unlistenAutoAway.then((fn) => fn());
            unlistenSummary.then((fn) => fn());
            unlistenClose.then((fn) => fn());
        };
    }, []);

    const handleFadeOut = () => {
        setIsVisible(false);
        setTimeout(() => {
            setWords("");
            setShowCheckmark(false);
        }, 300); // Wait for fade-out animation
    };

    const handleSubmit = async () => {
        if (!intervalId || !words.trim()) {
            return;
        }

        // Show checkmark immediately
        setShowCheckmark(true);

        // Submit words
        try {
            const result = await invoke<{ is_last_interval: boolean }>("submit_interval_words", {
                intervalId: intervalId,
                words: words.trim(),
            });

            // If this is the last interval, show summary ready after checkmark duration
            if (result.is_last_interval) {
                setTimeout(() => {
                    setShowSummaryReady(true);
                    setShowCheckmark(false);
                }, CHECKMARK_DURATION_MS);
            } else {
                // For non-last intervals, close window after checkmark duration
                setTimeout(() => {
                    invoke("hide_prompt_window_cmd").catch(console.error);
                }, CHECKMARK_DURATION_MS);
            }
        } catch (error) {
            console.error("Failed to submit words:", error);
            setShowCheckmark(false);
        }
    };

    const handleCloseSummary = async () => {
        try {
            await invoke("hide_prompt_window_cmd");
        } catch (error) {
            console.error("Failed to close summary window:", error);
        }
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && words.trim()) {
            handleSubmit();
        }
    };

    return (
        <div className={`prompt-container ${isVisible ? "fade-in" : "fade-out"}`}>
            {showSummaryReady ? (
                <div className="summary-ready-content">
                    <div className="summary-icon">📊</div>
                    <h3 className="summary-title">Summary Ready!</h3>
                    <p className="summary-message">Your workblock summary is ready to view.</p>
                    <button onClick={handleCloseSummary} className="close-summary-button">
                        Close
                    </button>
                </div>
            ) : showCheckmark ? (
                <div className="checkmark-container">
                    <div className="checkmark"></div>
                </div>
            ) : intervalId ? (
                <div className="prompt-content">
                    <div className="prompt-label">
                        What did you do? (1-2 words)
                    </div>
                    <input
                        id="words-input"
                        type="text"
                        value={words}
                        onChange={(e) => setWords(e.target.value)}
                        onKeyPress={handleKeyPress}
                        placeholder="e.g., coding, meeting"
                        className="words-input"
                        autoFocus
                        maxLength={50}
                    />
                </div>
            ) : (
                <div className="prompt-content">
                    <div className="loading-message">Loading...</div>
                </div>
            )}
        </div>
    );
}
