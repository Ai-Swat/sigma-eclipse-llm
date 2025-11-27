# Native Messaging Integration

This document describes the Native Messaging integration that allows Chrome/Edge extensions to communicate with the Sigma Eclipse LLM application.

## Architecture

```
┌─────────────────┐         ┌──────────────────┐         ┌─────────────────┐
│   Browser       │         │  Native Message  │         │   Sigma Eclipse  │
│   Extension     │ ◄─────► │     Host         │ ◄─────► │   Application   │
│  (JavaScript)   │  stdio  │   (Rust binary)  │  IPC    │   (Optional)    │
└─────────────────┘         └──────────────────┘         └─────────────────┘
```

### Components

1. **Native Messaging Host** (`sigma-eclipse-host`)
   - Standalone Rust binary
   - Implements Chrome Native Messaging Protocol
   - Reads from stdin, writes to stdout
   - Manages LLM server process
   - Shares state via IPC file

2. **IPC State Module** (`ipc_state.rs`)
   - File-based state storage (`ipc_state.json`)
   - Allows communication between host and Tauri app
   - Tracks server status, download progress

3. **Browser Extension** (`example-extension/`)
   - Chrome extension using Manifest V3
   - Background service worker for messaging
   - Popup UI for testing

## Installation

### Step 1: Build the Native Host

```bash
cd src-tauri
cargo build --release --bin sigma-eclipse-host
```

The binary will be at: `src-tauri/target/release/sigma-eclipse-host`

### Step 2: Install the Manifest

Run the installation script:

```bash
./scripts/install-native-messaging-host.sh
```

**Manual installation:**

Create manifest file at:
- **macOS**: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.sigma_eclipse.host.json`
- **Linux**: `~/.config/google-chrome/NativeMessagingHosts/com.sigma_eclipse.host.json`
- **Windows**: Registry key (see Chrome docs)

Manifest content:
```json
{
  "name": "com.sigma_eclipse.host",
  "description": "Sigma Eclipse LLM Native Messaging Host",
  "path": "/path/to/sigma-eclipse-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
```

### Step 3: Create Your Extension

Create a simple extension for testing (see "Usage in Your Extension" section below for complete examples).

## Protocol Specification

### Message Format

Native Messaging uses a binary protocol:
- **4 bytes**: Message length (uint32, native byte order)
- **N bytes**: JSON message

### Request Format

```json
{
  "id": "unique-request-id",
  "command": "command_name",
  "params": { /* optional parameters */ }
}
```

### Response Format

```json
{
  "id": "request-id",
  "success": true,
  "data": { /* response data */ },
  "error": null
}
```

Or on error:

```json
{
  "id": "request-id",
  "success": false,
  "data": null,
  "error": "Error message"
}
```

## Available Commands

### `start_server`

Start the LLM server.

**Parameters:**
- `port` (number, optional): Server port, default 8080
- `ctx_size` (number, optional): Context size, default 8192
- `gpu_layers` (number, optional): GPU layers, default 0

**Response:**
```json
{
  "message": "Server started on port 8080 (PID: 12345)",
  "pid": 12345,
  "port": 8080
}
```

**Example:**
```javascript
port.postMessage({
  id: '1',
  command: 'start_server',
  params: { port: 8080, ctx_size: 8192, gpu_layers: 0 }
});
```

### `stop_server`

Stop the LLM server.

**Parameters:** None

**Response:**
```json
{
  "message": "Server stopped"
}
```

### `get_server_status`

Get current server status.

**Parameters:** None

**Response:**
```json
{
  "is_running": true,
  "pid": 12345,
  "port": 8080,
  "message": "Server is running"
}
```

### `isDownloading`

Check if downloads are in progress.

**Parameters:** None

**Response:**
```json
{
  "is_downloading": false,
  "progress": null
}
```

## IPC State File

The host and Tauri app communicate via a shared state file:

**Location:** `~/Library/Application Support/sigma-eclipse/ipc_state.json` (macOS)

**Format:**
```json
{
  "server_pid": 12345,
  "server_running": true,
  "is_downloading": false,
  "download_progress": null,
  "server_port": 8080,
  "server_ctx_size": 8192,
  "server_gpu_layers": 0
}
```

## Usage in Your Extension

### Minimal Example

```javascript
// Connect to native host
const port = chrome.runtime.connectNative('com.sigma_eclipse.host');

// Send command
port.postMessage({
  id: '1',
  command: 'get_server_status',
  params: {}
});

// Receive response
port.onMessage.addListener((message) => {
  console.log('Received:', message);
  if (message.success) {
    console.log('Data:', message.data);
  } else {
    console.error('Error:', message.error);
  }
});

// Handle disconnect
port.onDisconnect.addListener(() => {
  if (chrome.runtime.lastError) {
    console.error('Error:', chrome.runtime.lastError.message);
  }
});
```

### Background Service Worker (Full Example)

Complete example for Chrome Extension Manifest V3 background service worker:

```javascript
// background.js - Background service worker for Native Messaging

const HOST_NAME = 'com.sigma_eclipse.host';
let port = null;
let messageId = 0;
let pendingRequests = new Map();

// Connect to native messaging host
function connect() {
  console.log('[Background] Connecting to native host:', HOST_NAME);
  
  port = chrome.runtime.connectNative(HOST_NAME);
  
  port.onMessage.addListener((message) => {
    console.log('[Background] Received from host:', message);
    
    // Resolve pending request
    if (message.id && pendingRequests.has(message.id)) {
      const resolver = pendingRequests.get(message.id);
      pendingRequests.delete(message.id);
      
      if (message.success) {
        resolver.resolve(message.data);
      } else {
        resolver.reject(new Error(message.error || 'Unknown error'));
      }
    }
    
    // Broadcast to popup or other extension pages if needed
    chrome.runtime.sendMessage({
      type: 'native_message',
      data: message
    }).catch(() => {
      // Popup might not be open, ignore error
    });
  });
  
  port.onDisconnect.addListener(() => {
    console.log('[Background] Disconnected from native host');
    if (chrome.runtime.lastError) {
      console.error('[Background] Disconnect error:', chrome.runtime.lastError.message);
    }
    port = null;
    
    // Reject all pending requests
    for (const [id, resolver] of pendingRequests.entries()) {
      resolver.reject(new Error('Connection closed'));
      pendingRequests.delete(id);
    }
  });
  
  console.log('[Background] Connected to native host');
}

// Send command to native host (Promise-based)
function sendCommand(command, params = {}) {
  return new Promise((resolve, reject) => {
    if (!port) {
      connect();
    }
    
    if (!port) {
      reject(new Error('Failed to connect to native host'));
      return;
    }
    
    const id = `${++messageId}`;
    const message = { id, command, params };
    
    pendingRequests.set(id, { resolve, reject });
    
    console.log('[Background] Sending to host:', message);
    port.postMessage(message);
    
    // Timeout after 30 seconds
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }
    }, 30000);
  });
}

// Handle messages from popup or other extension pages
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('[Background] Received from extension:', request);
  
  if (request.type === 'native_command') {
    sendCommand(request.command, request.params)
      .then(data => {
        sendResponse({ success: true, data });
      })
      .catch(error => {
        sendResponse({ success: false, error: error.message });
      });
    
    return true; // Keep channel open for async response
  }
  
  if (request.type === 'connect') {
    try {
      connect();
      sendResponse({ success: true });
    } catch (error) {
      sendResponse({ success: false, error: error.message });
    }
    return false;
  }
});

// Auto-connect on startup
console.log('[Background] Service worker started');
connect();
```

### Helper Class Wrapper

Cleaner interface for using Native Messaging:

```javascript
class SigmaEclipseClient {
  constructor() {
    this.port = null;
    this.messageId = 0;
    this.pending = new Map();
  }
  
  connect() {
    this.port = chrome.runtime.connectNative('com.sigma_eclipse.host');
    
    this.port.onMessage.addListener((message) => {
      const resolve = this.pending.get(message.id);
      if (resolve) {
        this.pending.delete(message.id);
        if (message.success) {
          resolve.resolve(message.data);
        } else {
          resolve.reject(new Error(message.error));
        }
      }
    });
    
    this.port.onDisconnect.addListener(() => {
      for (const [id, resolve] of this.pending) {
        resolve.reject(new Error('Disconnected'));
      }
      this.pending.clear();
    });
  }
  
  async sendCommand(command, params = {}) {
    if (!this.port) this.connect();
    
    const id = String(++this.messageId);
    const message = { id, command, params };
    
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.port.postMessage(message);
      
      // Timeout
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error('Request timeout'));
        }
      }, 30000);
    });
  }
  
  async startServer(port = 8080, ctx_size = 8192, gpu_layers = 0) {
    return this.sendCommand('start_server', { port, ctx_size, gpu_layers });
  }
  
  async stopServer() {
    return this.sendCommand('stop_server');
  }
  
  async getStatus() {
    return this.sendCommand('get_server_status');
  }
  
  async isDownloading() {
    return this.sendCommand('isDownloading');
  }
}

// Usage
const client = new SigmaEclipseClient();
const status = await client.getStatus();
console.log('Server running:', status.is_running);
```

## Debugging

### View Host Logs

The native host logs to **stderr** (not stdout):

```bash
# Run host manually to see logs
echo '{"id":"1","command":"get_server_status","params":{}}' | \
  /path/to/sigma-eclipse-host
```

### Check Extension Logs

1. Go to `chrome://extensions/`
2. Enable "Developer mode"
3. Click "service worker" or "background page" link
4. View console

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Specified native messaging host not found" | Manifest not found or invalid | Check manifest location and JSON syntax |
| "Failed to start native messaging host" | Binary not executable or missing | Check binary path and permissions |
| "Access to the specified native messaging host is forbidden" | Extension ID not in allowed_origins | Update manifest with correct extension ID |
| "Native host has exited" | Host crashed | Check host stderr logs |

## Security Considerations

1. **Extension ID Whitelist**: Only extensions in `allowed_origins` can connect
2. **Local Communication**: Native Messaging only works with locally installed apps
3. **No Network Access**: The host binary communicates via stdio only
4. **Process Isolation**: Each connection spawns a new host process

## Building for Distribution

When distributing your app, the Native Messaging manifest should be installed:

1. **macOS**: Copy manifest to `~/Library/Application Support/[Browser]/NativeMessagingHosts/`
2. **Linux**: Copy manifest to `~/.config/[browser]/NativeMessagingHosts/`
3. **Windows**: Create registry key under `HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts\com.sigma_eclipse.host`

Include the `sigma-eclipse-host` binary in your app bundle:
- **macOS**: In `.app/Contents/MacOS/`
- **Linux**: In `/usr/local/bin/` or app directory
- **Windows**: In application directory

## References

- [Chrome Native Messaging Documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
- [Native Messaging Protocol Specification](https://chromium.googlesource.com/chromium/src/+/master/extensions/docs/native_messaging.md)

## License

Part of Sigma Eclipse LLM project.

