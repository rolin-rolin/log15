import { useState, useEffect } from "react";
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

    useEffect(() => {
        if (intervalId) {
            setIsVisible(true);
        }
    }, [intervalId]);

    useEffect(() => {
        // Listen for hide event
        const unlisten = listen("prompt-hide", () => {
            handleFadeOut();
        });

        return () => {
            unlisten.then((fn) => fn());
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

        // Show checkmark
        setShowCheckmark(true);

        // Submit words
        try {
            await invoke("submit_interval_words", {
                intervalId: intervalId,
                words: words.trim(),
            });

            // Fade out after showing checkmark
            setTimeout(() => {
                handleFadeOut();
            }, 1000); // Show checkmark for 1 second
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

    if (!intervalId && !isVisible) {
        return null;
    }

    return (
        <div className={`prompt-container ${isVisible ? "fade-in" : "fade-out"}`}>
            {showCheckmark ? (
                <div className="checkmark-container">
                    <div className="checkmark">✓</div>
                </div>
            ) : (
                <div className="prompt-content">
                    <label htmlFor="words-input" className="prompt-label">
                        What did you do? (1-2 words)
                    </label>
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
                    <button onClick={handleSubmit} disabled={!words.trim()} className="submit-button">
                        Submit
                    </button>
                </div>
            )}
        </div>
    );
}
