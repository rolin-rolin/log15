import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./PromptWindow.css";

interface PromptWindowProps {
    intervalId: number | null;
}

export default function PromptWindow({ intervalId }: PromptWindowProps) {
    const [words, setWords] = useState("");
    const [showCheckmark, setShowCheckmark] = useState(false);
    const [isVisible, setIsVisible] = useState(false);
    const showCheckmarkRef = useRef(false);
    const intervalIdRef = useRef<number | null>(null);

    useEffect(() => {
        intervalIdRef.current = intervalId;
        if (intervalId) {
            setIsVisible(true);
            setShowCheckmark(false);
            showCheckmarkRef.current = false;
            setWords("");
        }
    }, [intervalId]);

    useEffect(() => {
        showCheckmarkRef.current = showCheckmark;
    }, [showCheckmark]);

    useEffect(() => {
        const unlisten = listen("prompt-hide", () => handleFadeOut());

        const unlistenAutoAway = listen("auto-away", () => {
            handleFadeOut();
            invoke("hide_prompt_window_cmd").catch(console.error);
        });

        const unlistenClose = listen("close-summary", () => handleFadeOut());

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
        }, 300);
    };

    const handleSubmit = async () => {
        if (!intervalId || !words.trim()) return;

        setShowCheckmark(true);
        showCheckmarkRef.current = true;

        try {
            await invoke("submit_interval_words", {
                intervalId,
                words: words.trim(),
            });
            // Backend closes the window after the checkmark animation
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
