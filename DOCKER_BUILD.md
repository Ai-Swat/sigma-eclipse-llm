# Building Windows Version with Docker on Mac/Linux

This guide explains how to build the Windows version of Sigma Eclipse locally using Docker.

## 🐳 Prerequisites

1. **Docker Desktop** installed and running
   - Download from: https://www.docker.com/products/docker-desktop
   - Make sure Docker is running (check menu bar icon)

2. **Disk Space**: ~10GB free
   - Docker image: ~3GB
   - Build artifacts: ~2-5GB
   - Cache: ~2-3GB

## 🚀 Quick Start

### Option 1: Use Build Script (Recommended)

```bash
# Make scripts executable (first time only)
chmod +x scripts/*.sh

# Build Windows version
./scripts/build-windows-docker.sh
```

### Option 2: Manual Docker Commands

```bash
# Build the Docker image
docker-compose build build-windows

# Run the build
docker-compose run --rm build-windows

# Or start interactive shell for debugging
docker-compose run --rm shell
```

## 📁 Output Location

After successful build, you'll find Windows installers at:

```
src-tauri/target/x86_64-pc-windows-gnu/release/bundle/
├── msi/
│   └── Sigma Eclipse_0.1.0_x64_en-US.msi
└── nsis/
    └── Sigma Eclipse_0.1.0_x64-setup.exe
```

## 🔧 Troubleshooting

### Build fails with "linker error"

```bash
# Clean build cache
docker-compose down -v
docker-compose build --no-cache build-windows
```

### "Docker is not running"

1. Open Docker Desktop
2. Wait for it to fully start (green icon in menu bar)
3. Try again

### Out of disk space

```bash
# Clean up Docker
docker system prune -a --volumes

# Remove only this project's containers
docker-compose down -v
```

### Interactive debugging

```bash
# Open shell inside container
./scripts/docker-shell.sh

# Inside container, try building step by step:
npm install
cd src-tauri
cargo build --target x86_64-pc-windows-gnu
```

## 🎯 Build Options

### Debug Build (faster, larger file)

```bash
docker-compose run --rm shell
# Inside container:
npm run tauri build -- --target x86_64-pc-windows-gnu --debug
```

### Release Build (default, optimized)

```bash
./scripts/build-windows-docker.sh
```

### Custom target

```bash
# For 32-bit Windows (if needed)
docker-compose run --rm shell
rustup target add i686-pc-windows-gnu
cargo build --target i686-pc-windows-gnu
```

## 💾 Caching

Docker uses volumes for caching to speed up rebuilds:

- `cargo-cache` - Rust dependencies
- `cargo-git` - Git dependencies  
- `node-modules` - NPM packages
- `target-cache` - Build artifacts

To clear all caches:

```bash
docker-compose down -v
```

## ⚙️ How It Works

1. **Docker Image** (`Dockerfile.windows`):
   - Based on Ubuntu 22.04
   - Installs Rust, Node.js, MinGW toolchain
   - Configures cross-compilation for Windows

2. **Build Process**:
   - Frontend (React): Compiled normally
   - Backend (Rust): Cross-compiled to Windows using MinGW
   - Bundler: Creates .msi and .exe installers

3. **Volumes**:
   - Your code is mounted into container
   - Build artifacts appear in your local filesystem
   - Caches are persisted between builds

## 🔍 Technical Details

### Target Triple

- `x86_64-pc-windows-gnu` - Windows 64-bit using GNU toolchain
- Why GNU not MSVC? MinGW works better in Linux environment

### About Wine

Wine is NOT included in the Docker image to avoid compatibility issues with different architectures (i386 support). It's not needed for building - only for testing Windows binaries inside the container, which should be done on a real Windows machine anyway.

### Limitations

- **No code signing** in Docker (requires Windows or macOS with certificate)
- **Testing** must be done on real Windows machine (Wine not included)
- **Some features** might not work exactly as native build

### Advantages

- Build anywhere (Mac, Linux, even Windows!)
- Consistent build environment
- No need to install Windows toolchain on host
- Reproducible builds

## 📊 Performance

First build: ~15-30 minutes (downloading dependencies)
Subsequent builds: ~5-10 minutes (with cache)

## 🆘 Getting Help

If you encounter issues:

1. Check Docker Desktop is running
2. Try cleaning cache: `docker-compose down -v`
3. Check logs in interactive shell: `./scripts/docker-shell.sh`
4. Search for error message in GitHub issues

## 🔄 Comparison with Other Methods

| Method | Pros | Cons | Cost |
|--------|------|------|------|
| **Docker** | Works on Mac/Linux, No Windows needed, Reproducible | Slower, Larger downloads, No signing | Free |
| **GitHub Actions** | Automatic, All platforms, Free CI | Requires push, Internet needed | Free |
| **Native Windows** | Fastest, Native tools, Easy debugging | Need Windows machine | Free |
| **Cross-compile (xwin)** | Fast, Local | Complex setup, Fragile | Free |

## 📝 Example Session

```bash
$ ./scripts/build-windows-docker.sh
🔧 Building Sigma Eclipse for Windows using Docker...

✓ Docker is running

📦 Building Docker image...
[+] Building 45.3s (12/12) FINISHED
...

🔨 Starting Windows build...
    Updating crates.io index
   Compiling sigma-eclipse v0.1.0
    Finished release [optimized] target(s) in 8m 32s
    Bundling Sigma Eclipse_0.1.0_x64_en-US.msi

✓ Build completed successfully!

📁 Output files are in:
   src-tauri/target/x86_64-pc-windows-gnu/release/bundle/

You can find:
   - .msi installer
   - .exe executable
```

---

**Note**: First build will take longer as Docker downloads all dependencies. Subsequent builds are much faster thanks to caching!

