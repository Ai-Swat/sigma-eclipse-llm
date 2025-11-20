# Code Signing Setup for CI/CD

This guide explains how to set up code signing for macOS and Windows builds in GitHub Actions.

## 🍎 macOS Code Signing & Notarization

### Prerequisites

1. **Apple Developer Account** ($99/year)
   - Sign up at https://developer.apple.com

2. **App Store Connect API Key** (for notarization)
   - Go to App Store Connect → Users and Access → Keys
   - Create a new API key with "Developer" role

### Step 1: Create Certificates

```bash
# On your Mac, open Keychain Access
# Certificate Assistant → Request a Certificate from a Certificate Authority
# Save it as CertificateSigningRequest.certSigningRequest

# Upload to Apple Developer Portal:
# - Go to https://developer.apple.com/account/resources/certificates/list
# - Click "+" to create new certificate
# - Choose "Developer ID Application" for macOS distribution
# - Upload your CSR file
# - Download the certificate and double-click to install in Keychain
```

### Step 2: Export Certificate for CI

```bash
# In Keychain Access:
# 1. Find your "Developer ID Application" certificate
# 2. Right-click → Export
# 3. Save as .p12 file with a strong password
# 4. Convert to base64:

base64 -i YourCertificate.p12 | pbcopy
# Now the base64 string is in your clipboard
```

### Step 3: Add GitHub Secrets

Go to GitHub repository → Settings → Secrets and variables → Actions → New repository secret

Add these secrets:

| Secret Name | Description | How to get |
|-------------|-------------|------------|
| `APPLE_CERTIFICATE` | Base64 encoded .p12 certificate | Output from `base64` command above |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the .p12 file | Password you set when exporting |
| `KEYCHAIN_PASSWORD` | Any strong password | Generate a random password |
| `APPLE_ID` | Your Apple ID email | Your Apple Developer account email |
| `APPLE_PASSWORD` | App-specific password | Generate at https://appleid.apple.com/account/manage → App-Specific Passwords |
| `APPLE_TEAM_ID` | Your Team ID | Find at https://developer.apple.com/account → Membership |
| `APPLE_SIGNING_IDENTITY` | Certificate name | Usually "Developer ID Application: Your Name (TEAM_ID)" |

### Verify Team ID

```bash
# Check your Team ID
security find-identity -v -p codesigning

# Look for output like:
# 1) XXXXX "Developer ID Application: Your Name (TEAM_ID_HERE)"
```

---

## 🪟 Windows Code Signing

### Prerequisites

1. **Code Signing Certificate** (~$100-400/year)
   - Purchase from: DigiCert, Sectigo, GlobalSign, SSL.com
   - Choose "Code Signing Certificate" (not EV)
   - Verify your organization (1-3 days)

### Option 1: Traditional Certificate (.pfx)

#### Step 1: Export Certificate

```powershell
# If you have .pfx file already, convert to base64:
[Convert]::ToBase64String([IO.File]::ReadAllBytes("certificate.pfx")) | Set-Clipboard
```

#### Step 2: Add GitHub Secrets

| Secret Name | Description |
|-------------|-------------|
| `WINDOWS_CERTIFICATE` | Base64 encoded .pfx file |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for .pfx file |

### Option 2: Tauri's Built-in Signing (Recommended)

Tauri has its own signing mechanism that's simpler:

```bash
# Generate a new private key (one-time setup)
openssl genpkey -algorithm RSA -out private-key.pem -pkeyopt rsa_keygen_bits:2048

# Convert to base64
base64 -i private-key.pem | pbcopy
```

Add to GitHub Secrets:

| Secret Name | Description |
|-------------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Base64 encoded private key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Optional password |

---

## 🐧 Linux (Optional)

Linux doesn't require code signing for most distributions. However, you can optionally sign packages:

```bash
# For .deb packages, you can use GPG signing
gpg --gen-key
gpg --export-secret-keys > private-key.gpg
base64 -i private-key.gpg | pbcopy
```

---

## 📦 Testing Locally

### Test macOS signing locally:

```bash
# Set environment variables
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAM_ID"
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"

# Build and sign
npm run tauri build
```

### Test Windows signing locally:

```powershell
# Import certificate to store first (one-time)
$password = ConvertTo-SecureString -String "your-password" -Force -AsPlainText
Import-PfxCertificate -FilePath certificate.pfx -CertStoreLocation Cert:\CurrentUser\My -Password $password

# Build
npm run tauri build
```

---

## ⚠️ Important Notes

### Security Best Practices

1. **Never commit certificates or keys to git**
2. **Use strong passwords for all certificates**
3. **Rotate secrets periodically**
4. **Limit secret access to necessary people**
5. **Use GitHub's secret scanning**

### Cost Summary

| Item | Cost | Renewal |
|------|------|---------|
| Apple Developer Program | $99 | Yearly |
| Windows Code Signing Cert | $100-400 | 1-3 years |
| Linux (Optional) | Free | N/A |

### Without Code Signing

If you don't want to pay for certificates yet:

1. **macOS**: Users need to right-click → Open (first time)
2. **Windows**: Users need to click "More info" → "Run anyway"
3. **Linux**: Works without signing

You can start without signing and add it later when ready for production.

---

## 🔧 Troubleshooting

### macOS: "Developer ID not found"

```bash
# List available identities
security find-identity -v -p codesigning

# Make sure the certificate is in the login keychain
```

### Windows: "Certificate not found"

```powershell
# List installed certificates
Get-ChildItem Cert:\CurrentUser\My

# Re-import if needed
```

### Notarization fails

- Check your Apple ID and app-specific password
- Verify Team ID is correct
- Ensure certificate is "Developer ID Application" type
- Check Apple Developer account status

---

## 📚 Additional Resources

- [Tauri Code Signing Guide](https://tauri.app/v1/guides/distribution/sign-macos)
- [Apple Notarization Process](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Windows Code Signing](https://learn.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)

