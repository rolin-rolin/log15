import { useState, useEffect, useRef } from "react";
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
    const showCheckmarkRef = useRef(false);
    const intervalIdRef = useRef<number | null>(null);

    useEffect(() => {
        console.log("[PROMPT_WINDOW] intervalId changed:", intervalId);
        intervalIdRef.current = intervalId;
        if (intervalId) {
            console.log("[PROMPT_WINDOW] Setting isVisible to true");
            setIsVisible(true);
            // Reset state when new interval comes in
            setShowCheckmark(false);
            showCheckmarkRef.current = false;
            setWords("");
        }
    }, [intervalId]);

    // Keep ref in sync with state
    useEffect(() => {
        showCheckmarkRef.current = showCheckmark;
    }, [showCheckmark]);

    useEffect(() => {
        // #region agent log
        fetch('http://127.0.0.1:7243/ingest/e1aff560-78fd-4480-b3b6-3bd988b7d39c',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'PromptWindow.tsx:39','message':'Setting up event listeners',data:{intervalId:intervalId},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'E'})}).catch(()=>{});
        // #endregion
        // Listen for hide event
        const unlisten = listen("prompt-hide", () => {
            console.log("[PROMPT_WINDOW] Received prompt-hide event");
            handleFadeOut();
        });

        // Listen for auto-away event (window should close for non-last intervals)
        const unlistenAutoAway = listen("auto-away", () => {
            console.log("[PROMPT_WINDOW] Received auto-away event, closing window");
            handleFadeOut();
            // Also call hide command to ensure window is closed
            invoke("hide_prompt_window_cmd").catch(console.error);
        });

        // No longer need to listen for show-summary-ready - backend closes and reopens window

        // Listen for close summary event
        const unlistenClose = listen("close-summary", () => {
            handleFadeOut();
        });

        return () => {
            unlisten.then((fn) => fn());
            unlistenAutoAway.then((fn) => fn());
            unlistenClose.then((fn) => fn());
        };
    }, []);

    const handleFadeOut = () => {
        setIsVisible(false);
        setTimeout(() => {
            setWords("");
            setShowCheckmark(false);
            showCheckmarkRef.current = false;
        }, 300); // Wait for fade-out animation
    };

    const handleSubmit = async () => {
        if (!intervalId || !words.trim()) {
            return;
        }

        // Show checkmark immediately
        setShowCheckmark(true);
        showCheckmarkRef.current = true;

        // Submit words
        try {
            await invoke<{ is_last_interval: boolean }>("submit_interval_words", {
                intervalId: intervalId,
                words: words.trim(),
            });

            // For all intervals (including last), close window after checkmark duration
            // For last interval, backend will open summary-ready window
            setTimeout(() => {
                invoke("hide_prompt_window_cmd").catch(console.error);
            }, CHECKMARK_DURATION_MS);
        } catch (error) {
            console.error("Failed to submit words:", error);
            setShowCheckmark(false);
        }
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && words.trim()) {
            handleSubmit();
        }
    };

    return (
        <div className={`prompt-container ${isVisible ? "fade-in" : "fade-out"}`}>
            {showCheckmark ? (
                <div className="checkmark-container">
                    <div className="checkmark"></div>
                </div>
            ) : intervalId ? (
                <div className="prompt-content">
                    <div className="prompt-label">What did you do? (1-2 words)</div>
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
