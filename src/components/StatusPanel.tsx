import { ServerStatus } from "../types";
import { StartButton } from "./StartButton.tsx";
import "./StatusPanel.css";

interface StatusPanelProps {
  status: ServerStatus;
  onStartServer: () => void;
  onStopServer: () => void;
  isBusy: boolean;
}

export const StatusPanel = ({ status, onStartServer, onStopServer, isBusy }: StatusPanelProps) => {
  return (
    <div className="status-panel">
      <div className="status-text-wrapper">
        <div className="status-indicator">
          <span>{status.is_running ? "Running" : "Stopped"}</span>
        </div>
        <p className="status-message">{status.message}</p>
      </div>

      <StartButton
        isRunning={status.is_running}
        handleClick={status.is_running ? onStopServer : onStartServer}
        isBusy={isBusy}
      />
    </div>
  );
};
