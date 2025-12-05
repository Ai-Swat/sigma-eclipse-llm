# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
