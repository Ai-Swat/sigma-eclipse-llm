# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.35] - 2025-12-17

### Added
- **Download resume support**: Automatic resume of interrupted downloads for llama.cpp and models
- HTTP Range request support detection for resumable downloads
- Exponential backoff retry logic (up to 10 retries) for network errors during download
- Connection stability improvements: `connect_timeout`, `pool_idle_timeout`, `tcp_keepalive`

### Changed
- Updated README.md download links to point to new sigma-eclipse-llm repository

## [0.1.34] - 2025-12-06

### Changed
- Enhanced thread safety in native messaging host with stdout locking
- Added background status monitoring in native messaging host
- Improved logging in check_and_push_status function for better debugging

## [0.1.32] - 2025-12-06

### Added
- Binary mode support for stdin/stdout on Windows in native messaging host
- Compatibility with Native Messaging Protocol on Windows

### Changed
- Enhanced server status management in Tauri app
- IPC state is now updated before stopping server
- Simplified status change detection in native messaging host

## [0.1.30] - 2025-12-03

### Added
- **Auto-updater**: Application now checks for updates automatically on startup
- Update dialog with version info, release notes, and download progress
- Signed update artifacts for secure distribution
- `make build-signed` command for local signed builds
- CI workflow automatically generates `latest.json` for updates

### Changed
- Updated CI workflow to support updater signing with `TAURI_SIGNING_PRIVATE_KEY`
- Added `EXTENSION_ID` secret to CI for native messaging configuration

## [0.1.10] - 2025-12-02

### Added
- Tauri application for managing LLM server
- Native messaging for browser extension communication
- Server management (start/stop)
- Model downloading with progress tracking
- Theme switching (light/dark)
- Settings panel
