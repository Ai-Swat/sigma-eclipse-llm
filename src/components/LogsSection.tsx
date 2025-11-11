interface LogsSectionProps {
  logs: string[];
}

export const LogsSection = ({ logs }: LogsSectionProps) => {
  return (
    <div className="section">
      <h2 className="logs-header">Logs</h2>
      <div className="logs">
        {logs.map((log, index) => (
          <div key={index} className="log-entry">
            {log}
          </div>
        ))}
        {logs.length === 0 && <div className="log-entry empty">No logs yet...</div>}
      </div>
    </div>
  );
};
