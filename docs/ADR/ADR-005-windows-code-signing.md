# ADR-005: Windows Code Signing (Authenticode)

## Status
Proposed

## Context

GigaWhisper is distributed as a Windows desktop application via NSIS installers. Currently, the application is **not code-signed**, which causes several issues:

1. **Windows SmartScreen warnings**: Users see "Windows protected your PC" dialogs, requiring extra clicks to run the installer
2. **Antivirus false positives**: Unsigned executables are more likely to be flagged by security software
3. **User trust**: Unsigned applications appear less professional and trustworthy
4. **Enterprise deployment**: Many corporate environments block unsigned applications

Code signing with Microsoft Authenticode proves the software comes from a verified publisher and has not been tampered with since signing.

### Current State

The `tauri.conf.json` already has placeholder fields for code signing:

```json
"windows": {
  "certificateThumbprint": null,
  "digestAlgorithm": "sha256",
  "timestampUrl": ""
}
```

The GitHub Actions workflow (`release.yml`) uses `tauri-apps/tauri-action@v0` for building but does not currently perform code signing.

## Decision

Implement Windows Authenticode code signing for GigaWhisper releases with the following strategy:

### 1. Certificate Type Selection

| Type | Cost | SmartScreen | Validation | Recommendation |
|------|------|-------------|------------|----------------|
| **Self-Signed** | Free | No trust | None | Development only |
| **OV (Organization Validation)** | ~$100-300/year | Builds trust over time | Organization verified | **Recommended for OSS** |
| **EV (Extended Validation)** | ~$300-600/year | Immediate trust | Extensive verification + hardware token | Enterprise/commercial |

**Decision**: Start with **OV (Organization Validation)** certificate.

**Rationale**:
- EV certificates require hardware tokens (HSM/USB) which complicate CI/CD automation
- EV immediate SmartScreen trust is valuable but OV builds reputation over time
- OV certificates can be stored as PFX files, compatible with GitHub Actions secrets
- Cost-effective for an open-source project
- Can upgrade to EV later if needed

### 2. Certificate Providers (OV)

Recommended providers for OV code signing certificates:

| Provider | Price/Year | Notes |
|----------|------------|-------|
| **Certum** | ~$59-99 | Popular for OSS, good support |
| **SSL.com** | ~$149 | Reseller-friendly pricing |
| **Sectigo (Comodo)** | ~$179 | Widely recognized |
| **DigiCert** | ~$474 | Premium, fastest issuance |
| **GlobalSign** | ~$249 | Good reputation |

**Note**: Prices vary; shop for resellers. Certum Open Source Developer Certificate is the most economical option for open-source projects.

### 3. GitHub Actions Integration

#### Secrets Required

| Secret Name | Description |
|-------------|-------------|
| `WINDOWS_CERTIFICATE` | Base64-encoded PFX certificate file |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the PFX file |

#### Workflow Modifications

Update `.github/workflows/release.yml` to include code signing:

```yaml
# Add to build-windows job, before tauri-action
- name: Import Windows Certificate
  if: matrix.variant != ''  # All Windows builds
  env:
    WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
    WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
  run: |
    if [ -n "$WINDOWS_CERTIFICATE" ]; then
      echo "$WINDOWS_CERTIFICATE" | base64 -d > certificate.pfx

      # Get certificate thumbprint
      THUMBPRINT=$(openssl pkcs12 -in certificate.pfx -passin pass:"$WINDOWS_CERTIFICATE_PASSWORD" -nokeys -clcerts | openssl x509 -noout -fingerprint -sha1 | cut -d'=' -f2 | tr -d ':')
      echo "CERTIFICATE_THUMBPRINT=$THUMBPRINT" >> $GITHUB_ENV

      # Import to Windows certificate store
      certutil -f -p "$WINDOWS_CERTIFICATE_PASSWORD" -importpfx certificate.pfx

      rm certificate.pfx
    fi
  shell: bash
```

**Alternative approach using PowerShell** (more robust for Windows):

```yaml
- name: Import Windows Certificate
  if: matrix.variant != ''
  shell: pwsh
  env:
    WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
    WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
  run: |
    if ($env:WINDOWS_CERTIFICATE) {
      $pfxBytes = [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE)
      $certPath = Join-Path $env:RUNNER_TEMP "certificate.pfx"
      [IO.File]::WriteAllBytes($certPath, $pfxBytes)

      # Import certificate
      $securePassword = ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force
      $cert = Import-PfxCertificate -FilePath $certPath -CertStoreLocation Cert:\CurrentUser\My -Password $securePassword

      # Export thumbprint for Tauri
      $thumbprint = $cert.Thumbprint
      Write-Host "Certificate thumbprint: $thumbprint"
      echo "CERTIFICATE_THUMBPRINT=$thumbprint" >> $env:GITHUB_ENV

      # Clean up
      Remove-Item $certPath
    }
```

### 4. Tauri Configuration

Update `src-tauri/tauri.conf.json` dynamically during CI or use environment variables:

**Option A: Environment Variable (Recommended)**

Tauri v2 reads `TAURI_SIGNING_IDENTITY` environment variable. Add to workflow:

```yaml
env:
  TAURI_SIGNING_IDENTITY: ${{ env.CERTIFICATE_THUMBPRINT }}
```

**Option B: Direct Configuration**

For local development with a certificate installed:

```json
"windows": {
  "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT",
  "digestAlgorithm": "sha256",
  "timestampUrl": "http://timestamp.digicert.com"
}
```

### 5. Timestamp Server URLs

Timestamping is **critical** - it ensures signatures remain valid after the certificate expires.

Recommended timestamp servers (RFC 3161 compliant):

| Provider | URL | Notes |
|----------|-----|-------|
| DigiCert | `http://timestamp.digicert.com` | Most reliable |
| Sectigo | `http://timestamp.sectigo.com` | Good fallback |
| GlobalSign | `http://timestamp.globalsign.com/tsa/r6advanced1` | Alternative |
| SSL.com | `http://ts.ssl.com` | Works well |

**Decision**: Use `http://timestamp.digicert.com` as primary timestamp server.

### 6. Complete Workflow Integration

```yaml
build-windows:
  name: Build Windows (${{ matrix.variant }})
  needs: create-release
  runs-on: windows-latest
  strategy:
    matrix:
      include:
        - variant: cpu
          features: ""
        - variant: vulkan
          features: "--features gpu-vulkan"
        - variant: cuda
          features: "--features gpu-cuda"

  steps:
    - uses: actions/checkout@v4

    # ... existing setup steps ...

    # Code Signing Setup
    - name: Setup Code Signing Certificate
      if: env.WINDOWS_CERTIFICATE != ''
      shell: pwsh
      env:
        WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
        WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
      run: |
        $pfxBytes = [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE)
        $certPath = Join-Path $env:RUNNER_TEMP "certificate.pfx"
        [IO.File]::WriteAllBytes($certPath, $pfxBytes)

        $securePassword = ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force
        $cert = Import-PfxCertificate -FilePath $certPath -CertStoreLocation Cert:\CurrentUser\My -Password $securePassword

        echo "CERTIFICATE_THUMBPRINT=$($cert.Thumbprint)" >> $env:GITHUB_ENV

        Remove-Item $certPath

    - name: Configure Tauri Signing
      if: env.CERTIFICATE_THUMBPRINT != ''
      shell: pwsh
      run: |
        $config = Get-Content "src-tauri/tauri.conf.json" | ConvertFrom-Json
        $config.bundle.windows.certificateThumbprint = $env:CERTIFICATE_THUMBPRINT
        $config.bundle.windows.timestampUrl = "http://timestamp.digicert.com"
        $config | ConvertTo-Json -Depth 20 | Set-Content "src-tauri/tauri.conf.json"
        Write-Host "Configured code signing with thumbprint: $env:CERTIFICATE_THUMBPRINT"

    # ... rest of build steps ...
```

### 7. Local Development Signing

For testing code signing locally:

1. **Generate self-signed certificate** (development only):
   ```powershell
   New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=GigaWhisper Dev" -CertStoreLocation Cert:\CurrentUser\My
   ```

2. **Export to PFX**:
   ```powershell
   $cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -like "*GigaWhisper*" }
   Export-PfxCertificate -Cert $cert -FilePath "gigawhisper-dev.pfx" -Password (ConvertTo-SecureString -String "password" -Force -AsPlainText)
   ```

3. **Configure `tauri.conf.json`** with the thumbprint (visible in Certificate Manager)

### 8. Windows Notarization (Not Applicable)

Unlike macOS, Windows does **not** have a notarization process. Code signing with a valid certificate is sufficient for:
- Removing SmartScreen warnings (after reputation builds)
- Proving publisher identity
- Ensuring code integrity

**SmartScreen Reputation Building**:
- New certificates start with no reputation
- Reputation builds as more users download and run the software
- EV certificates have immediate reputation; OV certificates need time
- Typical time to build reputation: 2-4 weeks with moderate download volume

## Consequences

### Positive
- **Eliminates SmartScreen warnings** (after reputation builds or immediately with EV)
- **Reduces antivirus false positives** significantly
- **Builds user trust** with verified publisher identity
- **Enables enterprise deployment** in corporate environments
- **Protects users** from tampered binaries

### Negative
- **Annual cost** (~$100-300/year for OV certificate)
- **Initial setup complexity** with certificate management
- **Reputation delay** with OV certificates (2-4 weeks)
- **Secret management** overhead in CI/CD

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Certificate exposed in logs | Use GitHub encrypted secrets, never echo certificate data |
| Certificate expires | Set calendar reminders, monitor expiration |
| Wrong certificate used | Validate thumbprint in CI logs |
| Timestamping fails | Use multiple fallback timestamp servers |

## Implementation Plan

### Phase 1: Certificate Acquisition (Week 1)
1. Research and select certificate provider (recommend: Certum or SSL.com)
2. Complete organization validation process
3. Obtain OV code signing certificate
4. Export as PFX file with secure password

### Phase 2: GitHub Actions Setup (Week 2)
1. Add `WINDOWS_CERTIFICATE` secret (base64-encoded PFX)
2. Add `WINDOWS_CERTIFICATE_PASSWORD` secret
3. Update `release.yml` with certificate import and signing configuration
4. Test with a pre-release build

### Phase 3: Validation (Week 2-3)
1. Verify signatures using `signtool verify /pa /v GigaWhisper_x64-setup.exe`
2. Test installation on clean Windows VM
3. Monitor SmartScreen behavior over time
4. Document signing verification in PRODUCTION_READINESS.md

### Phase 4: Monitoring (Ongoing)
1. Track certificate expiration (set renewal reminders)
2. Monitor user feedback on installation experience
3. Consider EV upgrade if reputation building is too slow

## Alternatives Considered

### 1. EV Certificate from Day One
- **Rejected because**: Requires hardware token (HSM), complicates CI/CD
- **Advantage not retained**: Immediate SmartScreen trust

### 2. Self-Signed Certificate for Releases
- **Rejected because**: Provides no trust benefit, still triggers SmartScreen
- **Advantage not retained**: Free, immediate availability

### 3. Azure Trusted Signing (Preview)
- **Considered for future**: Microsoft's cloud-based signing service
- **Not chosen now**: Still in preview, limited availability
- **Revisit when**: Generally available and pricing is clear

### 4. SignPath.io (Free for OSS)
- **Considered**: Free code signing service for open-source projects
- **Concerns**: External dependency, queue times, less control
- **Revisit if**: Budget is a major constraint

## References

- [Tauri Code Signing Documentation](https://v2.tauri.app/distribute/sign/windows/)
- [Microsoft Authenticode Overview](https://docs.microsoft.com/en-us/windows-hardware/drivers/install/authenticode)
- [GitHub Actions Encrypted Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [SmartScreen Reputation](https://docs.microsoft.com/en-us/windows/security/threat-protection/microsoft-defender-smartscreen/microsoft-defender-smartscreen-overview)
- [Certum Open Source Code Signing](https://shop.certum.eu/open-source-code-signing-certificate.html)

## Appendix: Certificate Management Commands

### Encode PFX for GitHub Secret
```bash
base64 -i certificate.pfx | tr -d '\n' > certificate_base64.txt
```

### Verify Signature (Windows)
```powershell
signtool verify /pa /v "GigaWhisper_1.0.0_x64-cpu-setup.exe"
```

### Check Certificate Details
```powershell
Get-AuthenticodeSignature "GigaWhisper_1.0.0_x64-cpu-setup.exe" | Format-List *
```

### List Installed Certificates
```powershell
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert
```
