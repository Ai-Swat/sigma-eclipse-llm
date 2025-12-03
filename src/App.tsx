import { Toaster } from "sonner";
import {
  HeaderSection,
  SettingsPanel,
  StatusPanel,
  LogsSection,
  ThemeSwitcher,
  UpdateDialog,
} from "./components";
import { useApp, useUpdater } from "./hooks";
import "./App.css";

function App() {
  const {
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
    ctxSize,
    gpuLayers,
    appDataPath,

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
  } = useApp();

  const {
    updateAvailable,
    updateInfo,
    isDownloading: isDownloadingUpdate,
    downloadProgress: updateDownloadProgress,
    isInstalling,
    downloadAndInstall,
    dismissUpdate,
  } = useUpdater();

  return (
    <main className="container">
      <HeaderSection onToggleSettings={() => setIsSettingsOpen(!isSettingsOpen)} />

      <SettingsPanel
        isOpen={isSettingsOpen}
        appDataPath={appDataPath}
        baseModel={baseModel}
        isUncensored={isUncensored}
        ctxSize={ctxSize}
        gpuLayers={gpuLayers}
        isDownloadingLlama={isDownloadingLlama}
        isDownloadingModel={isDownloadingModel}
        downloadProgress={downloadProgress}
        status={status}
        onClose={() => setIsSettingsOpen(false)}
        onDownloadLlama={handleDownloadLlama}
        onDownloadModel={handleDownloadModel}
        onUncensoredChange={handleUncensoredChange}
        onCtxSizeChange={handleCtxSizeChange}
        onGpuLayersChange={handleGpuLayersChange}
        onRestoreDefaults={handleRestoreDefaults}
        onClearAllData={handleClearAllData}
        isProduction={isProduction}
      />

      <div className="content">
        <StatusPanel
          status={status}
          onStartServer={handleStartServer}
          onStopServer={handleStopServer}
          isBusy={isBusy}
        />

        {!isProduction && <LogsSection logs={logs} />}

        <div className="footer-section">
          <ThemeSwitcher theme={theme} onToggleTheme={toggleTheme} />
        </div>
      </div>

      <Toaster position="bottom-right" expand={true} richColors closeButton dir="ltr" />

      {updateAvailable && updateInfo && (
        <UpdateDialog
          isDownloading={isDownloadingUpdate}
          downloadProgress={updateDownloadProgress}
          isInstalling={isInstalling}
          onUpdate={downloadAndInstall}
          onDismiss={dismissUpdate}
        />
      )}
    </main>
  );
}

export default App;
