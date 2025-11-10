import { ServerStatus } from "../types";

interface StatusPanelProps {
  status: ServerStatus;
  onStartServer: () => void;
  onStopServer: () => void;
}

export const StatusPanel = ({ status, onStartServer, onStopServer }: StatusPanelProps) => {
  return (
    <div className="status-panel">
      <div className={`status-indicator ${status.is_running ? "running" : "stopped"}`}>
        <div className="status-dot"></div>
        <span>{status.is_running ? "Running" : "Stopped"}</span>
      </div>
      <p className="status-message">{status.message}</p>
      <button 
        className="server-toggle-button"
        onClick={status.is_running ? onStopServer : onStartServer}
      >
        {status.is_running ? "Stop" : "Start"}
      </button>
    </div>
  );
};

