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

export interface AppSettings {
  active_model: string;
  port: number;
  ctx_size: number;
  gpu_layers: number;
}

export interface RecommendedSettings {
  memory_gb: number;
  recommended_model: string;
  recommended_ctx_size: number;
  recommended_gpu_layers: number;
}
