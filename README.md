# Sigma Shield

A Tauri + React + TypeScript application for managing LLM servers.

## 📋 Prerequisites

Before starting development, make sure you have the following components installed:

### Required Dependencies

1. **Node.js** (v18 or higher)
   - Download: https://nodejs.org/

2. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **System Dependencies for Tauri**
   
   **macOS:**
   ```bash
   xcode-select --install
   ```

   **Linux (Debian/Ubuntu):**
   ```bash
   sudo apt update
   sudo apt install libwebkit2gtk-4.1-dev \
     build-essential \
     curl \
     wget \
     file \
     libxdo-dev \
     libssl-dev \
     libayatana-appindicator3-dev \
     librsvg2-dev
   ```

   **Windows:**
   - Microsoft Visual Studio C++ Build Tools
   - WebView2 (usually already installed on Windows 11)

## 🚀 Development Setup

### 1. Install Dependencies

```bash
npm install
```

### 2. Run in Development Mode

```bash
npm run tauri dev
```

This command will:
- Start the Vite dev server for the frontend
- Compile the Rust backend
- Open the application with hot-reload

### 3. Alternative Launch (Frontend Only)

If you need to work only with the UI without Tauri:

```bash
npm run dev
```

## 🏗️ Building the Project

### Development Build

```bash
npm run tauri build -- --debug
```

### Production Build

```bash
npm run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`

## 📁 Project Structure

```
sigma-shield/
├── src/                    # React frontend
│   ├── components/        # React components
│   ├── hooks/            # Custom React hooks
│   ├── styles/           # CSS styles
│   └── types/            # TypeScript types
├── src-tauri/            # Rust backend
│   ├── src/
│   │   ├── main.rs       # Entry point
│   │   ├── server.rs     # Server management logic
│   │   ├── download.rs   # File download logic
│   │   └── ...
│   ├── Cargo.toml        # Rust dependencies
│   └── tauri.conf.json   # Tauri configuration
├── package.json          # Node.js dependencies
└── vite.config.ts        # Vite configuration
```

## 🛠️ Useful Commands

```bash
# Check Rust code
cd src-tauri
cargo check

# Run Rust tests
cargo test

# Format Rust code
cargo fmt

# Check TypeScript
npm run build

# Clean build artifacts
cd src-tauri
cargo clean
```

## 🔧 Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 📚 Additional Documentation

- [Tauri Documentation](https://tauri.app/)
- [Vite Documentation](https://vitejs.dev/)
- [React Documentation](https://react.dev/)
