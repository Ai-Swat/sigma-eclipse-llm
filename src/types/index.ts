export interface ServerStatus {
  is_running: boolean;
  message: string;
}

export interface DownloadProgress {
  downloaded: number;
  total: number | null;
  percentage: number | null;
  message: string;
}

