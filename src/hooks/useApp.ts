import { useState } from "react";
import { toast } from "sonner";
import { useTheme } from "./useTheme";
import { useServerStatus } from "./useServerStatus";
import { useLogs } from "./useLogs";
import { useDownloadProgress } from "./useDownloadProgress";
import { useAutoDownload } from "./useAutoDownload";
import { useSettings } from "./useSettings";
import { useServerControl } from "./useServerControl";
import { useModelDownload } from "./useModelDownload";

export const useApp = () => {
  const { theme, toggleTheme } = useTheme();
  const status = useServerStatus();
  const { logs, addLog } = useLogs();
  const isProduction = import.meta.env.PROD;

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  // Settings management
  const {
    baseModel,
    isUncensored,
    setIsUncensored,
    port,
    ctxSize,
    gpuLayers,
    appDataPath,
    currentModel,
    handleCtxSizeChange,
    handleGpuLayersChange,
    handleRestoreDefaults: restoreDefaults,
  } = useSettings({ addLog });

  // Download progress
  const { downloadProgress, setCurrentToastId, setDownloadProgress } = useDownloadProgress(addLog);

  // Auto-download on startup
  const {
    isDownloadingLlama,
    isDownloadingModel,
    isModelAlreadyDownloaded,
    isLlamaAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
  } = useAutoDownload({
    modelName: currentModel,
    addLog,
    setCurrentToastId,
    setDownloadProgress,
  });

  // Server control
  const { isBusy, handleStartServer, handleStopServer, handleClearAllData } = useServerControl({
    addLog,
    port,
    ctxSize,
    gpuLayers,
    status,
  });

  // Model download handlers
  const { handleDownloadLlama, handleDownloadModel, handleUncensoredChange } = useModelDownload({
    baseModel,
    setIsUncensored,
    currentModel,
    isLlamaAlreadyDownloaded,
    isModelAlreadyDownloaded,
    setIsDownloadingLlama,
    setIsDownloadingModel,
    setDownloadProgress,
    setCurrentToastId,
    addLog,
  });

  // Wrap restoreDefaults to handle toast
  const handleRestoreDefaults = async () => {
    try {
      await restoreDefaults();
      toast.success("Settings restored to defaults");
    } catch (error) {
      toast.error(`Error restoring defaults: ${error}`);
    }
  };

  return {
    // Theme
    theme,
    toggleTheme,

    // Status
    status,

    // Logs
    logs,
    isProduction,

    // Settings panel
    isSettingsOpen,
    setIsSettingsOpen,

    // Settings values
    baseModel,
    isUncensored,
    port,
    ctxSize,
    gpuLayers,
    appDataPath,
    currentModel,

    // Download state
    isDownloadingLlama,
    isDownloadingModel,
    downloadProgress,

    // Server control
    isBusy,

    // Handlers
    handleCtxSizeChange,
    handleGpuLayersChange,
    handleRestoreDefaults,
    handleStartServer,
    handleStopServer,
    handleClearAllData,
    handleDownloadLlama,
    handleDownloadModel,
    handleUncensoredChange,
  };
};

