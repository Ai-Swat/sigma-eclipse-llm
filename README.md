<div align="center">

<img src="src/assets/logo2.png" alt="Sigma Eclipse Logo" width="128" height="128">

# Sigma Eclipse LLM

**Run powerful AI locally — no cloud, no limits, complete privacy.**

[![License](https://img.shields.io/badge/license-PolyForm%20NC-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-blue)](#installation)
[![Tauri](https://img.shields.io/badge/built%20with-Tauri%202-FFC131?logo=tauri)](https://tauri.app/)
[![llama.cpp](https://img.shields.io/badge/powered%20by-llama.cpp-green)](https://github.com/ggerganov/llama.cpp)

[Features](#-features) • [Installation](#-installation) • [How It Works](#-how-it-works) • [Development](#-development) • [License](#-license)

---

</div>

## 🌐 Part of Sigma Eclipse Ecosystem

This application is a core component of the **Sigma Eclipse** project — a privacy-focused AI ecosystem. It works seamlessly with:

- **[Sigma Browser](https://www.sigmabrowser.com/)** — A privacy-first browser with built-in AI capabilities
- **[Sigma Eclipse Extension](https://github.com/Ai-Swat/sigma-eclipse-extension)** — Browser extension that connects to this local LLM server

Together, these components provide a complete solution for running AI locally while browsing the web, ensuring your data stays on your machine.

## 🚀 What is Sigma Eclipse LLM?

Sigma Eclipse LLM is a lightweight desktop application that lets you run large language models (LLMs) locally on your machine. No API keys, no subscriptions, no data leaving your computer — just pure, private AI at your fingertips.

Built with [Tauri](https://tauri.app/) and powered by [llama.cpp](https://github.com/ggerganov/llama.cpp), Sigma Eclipse LLM combines native performance with a beautiful, intuitive interface.


## ✨ Features

### 🎯 Dead Simple
- **One-click setup** — automatically downloads everything you need
- **Zero configuration** — smart defaults that just work
- **Clean interface** — no clutter, no confusion

### 🔒 Privacy First
- **100% local** — your data never leaves your machine
- **No accounts** — no sign-ups, no tracking, no telemetry
- **Offline capable** — works without internet after initial setup

### ⚡ Powerful
- **GPU acceleration** — automatic GPU detection and optimization
- **Multiple models** — switch between models easily
- **Native performance** — Rust backend with minimal resource usage
- **Browser integration** — seamless connection with Sigma browser extension

### 🌍 Cross-Platform
- **macOS** (Apple Silicon)
- **Windows** (x64)

## 📦 Installation

### Download

Download the latest release for your platform:

| Platform | Download |
|----------|----------|
| macOS (ARM) | [Sigma Eclipse.dmg](https://github.com/ai-swat/sigma-eclipse-llm/releases/latest) |
| Windows | [Sigma Eclipse Setup.exe](https://github.com/ai-swat/sigma-eclipse-llm/releases/latest) |

### First Launch

1. **Open Sigma Eclipse**
2. **Wait for automatic setup** — the app downloads llama.cpp and the default model (~3-6 GB)
3. **Click "Start"** — your local AI server is now running!

That's it. No terminal commands, no manual downloads, no config files.

## 🔧 How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                     Sigma Eclipse                            │
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   React UI   │◄──►│  Tauri Core  │◄──►│  llama.cpp   │   │
│  │  (Frontend)  │    │    (Rust)    │    │   (Server)   │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│                              │                               │
│                              ▼                               │
│                   ┌──────────────────┐                      │
│                   │  Native Messaging │                      │
│                   │   (Browser API)   │                      │
│                   └──────────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

**Sigma Eclipse** manages a local llama.cpp server that provides an OpenAI-compatible API. This means:

- 🌐 **Local API endpoint** at `http://localhost:8080`
- 🔌 **Compatible** with any tool that supports OpenAI API
- 🧩 **Native messaging** enables browser extensions to communicate directly

## ⚙️ Configuration

Access settings via the ⚙️ gear icon:

| Setting | Description | Default |
|---------|-------------|---------|
| **Context Size** | Maximum conversation context (tokens) | Auto-detected |
| **GPU Layers** | Number of layers offloaded to GPU | Auto-detected |
| **Model** | Select from available models | Gemma 2B |

> 💡 **Tip:** Sigma Eclipse automatically detects your hardware and suggests optimal settings.

## 🛠️ Development

### Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://rustup.rs/) (latest stable)
- Platform-specific dependencies:
  - **macOS:** `xcode-select --install`
  - **Windows:** Visual Studio C++ Build Tools

### Quick Start

```bash
# Clone the repository
git clone https://github.com/ai-swat/sigma-eclipse-llm.git
cd sigma-eclipse

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

Built artifacts will be in `src-tauri/target/release/bundle/`

### Project Structure

```
sigma-eclipse/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/              # React hooks
│   ├── styles/             # CSS styles
│   └── types/              # TypeScript types
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── server.rs       # LLM server management
│   │   ├── download/       # Model & binary downloads
│   │   └── native_messaging.rs
│   └── tauri.conf.json     # Tauri configuration
└── package.json
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📜 License

This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).

**TL;DR:** Free for personal, educational, and non-commercial use. Contact us for commercial licensing.

## 🙏 Acknowledgments

- [llama.cpp](https://github.com/ggerganov/llama.cpp) — The amazing LLM inference engine
- [Tauri](https://tauri.app/) — Framework for building tiny, fast desktop apps
- [Hugging Face](https://huggingface.co/) — Model hosting and community

---

<div align="center">

**Made with ❤️ by [AI SWAT](https://github.com/ai-swat)**

[⬆ Back to Top](#sigma-eclipse-llm)

</div>
