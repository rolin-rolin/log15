// Summary ready page for the overlay window
import { invoke } from "@tauri-apps/api/core";
import "../components/PromptWindow.css";

export default function SummaryReadyPage() {
    const handleClose = async () => {
        try {
            await invoke("hide_prompt_window_cmd");
        } catch (error) {
            console.error("Failed to close summary window:", error);
        }
    };

    return (
        <div className="prompt-window-root">
            <div className="prompt-container fade-in">
                <div className="summary-ready-content">
                    <div className="summary-icon">📊</div>
                    <h3 className="summary-title">Summary Ready!</h3>
                    <p className="summary-message">Your session summary is ready to view.</p>
                    <button onClick={handleClose} className="close-summary-button">
                        Close
                    </button>
                </div>
            </div>
        </div>
    );
}

