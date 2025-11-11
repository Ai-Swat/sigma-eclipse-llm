import { useState } from "react";

export const useLogs = () => {
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (message: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

  return { logs, addLog };
};
