# ADR-007: Opt-in Crash Reporting System

## Status

Proposed

## Date

2026-01-26

## Context

GigaWhisper is a desktop application for voice transcription that runs locally on users' machines. As an open-source project distributed to diverse Windows environments, debugging crashes and errors is challenging because:

1. **No visibility into production issues**: Users experience crashes without developers knowing
2. **Limited reproduction ability**: Crashes may be environment-specific (GPU drivers, audio hardware, Windows versions)
3. **Support burden**: Users must manually describe issues, often lacking technical details
4. **whisper.cpp complexity**: Native code integration increases crash surface area
5. **Silent failures**: Some errors (audio capture, model loading) may fail silently

### Requirements

Any crash reporting solution must:

1. **Be opt-in by default**: Respect user privacy, no data collection without explicit consent
2. **Never collect audio data**: Voice recordings must never leave the device
3. **Provide useful debugging context**: Stack traces, system info, error context
4. **Work with Rust + Tauri**: Compatible with the existing tech stack
5. **Be cost-effective**: Suitable for an open-source project with limited budget
6. **Support offline scenarios**: Handle cases where network is unavailable

### Privacy Constraints

GigaWhisper handles sensitive voice data. The crash reporting system must:

- Never capture audio buffers or transcription content
- Minimize PII collection (no usernames, file paths where possible)
- Allow users to review reports before sending (optional)
- Provide clear data retention and deletion policies
- Comply with GDPR and similar privacy regulations

## Options Considered

### Option 1: Sentry (SaaS)

[Sentry](https://sentry.io) is a popular error monitoring platform with official Rust SDK (`sentry-rust`).

#### Architecture

```
+-------------------+       +------------------+       +----------------+
|   GigaWhisper     |  -->  |   Sentry SDK     |  -->  |  Sentry Cloud  |
|   (Rust/Tauri)    |       |   (sentry-rust)  |       |  (sentry.io)   |
+-------------------+       +------------------+       +----------------+
                                    |
                                    v
                            +------------------+
                            | Privacy Filters  |
                            | (before_send)    |
                            +------------------+
```

#### Integration Example

```rust
// src-tauri/src/crash_reporting/mod.rs
use sentry::{ClientOptions, IntoDsn};

pub struct CrashReporter {
    enabled: bool,
    _guard: Option<sentry::ClientInitGuard>,
}

impl CrashReporter {
    pub fn init(enabled: bool, dsn: &str) -> Self {
        if !enabled {
            return Self { enabled: false, _guard: None };
        }

        let guard = sentry::init(ClientOptions {
            dsn: dsn.into_dsn().ok().flatten(),
            release: Some(env!("CARGO_PKG_VERSION").into()),
            environment: Some("production".into()),
            before_send: Some(Arc::new(|mut event| {
                // Strip potential PII from stack traces
                Self::sanitize_event(&mut event);
                Some(event)
            })),
            ..Default::default()
        });

        Self { enabled: true, _guard: Some(guard) }
    }

    fn sanitize_event(event: &mut sentry::protocol::Event) {
        // Remove user paths from stack traces
        // Filter out any audio-related data
        // Anonymize system information
    }
}
```

#### Pros

- **Mature ecosystem**: Well-documented, battle-tested SDK
- **Rich features**: Automatic panic capture, performance monitoring, release tracking
- **Web dashboard**: Easy issue triage, deduplication, alerting
- **Free tier**: 5,000 errors/month, sufficient for early-stage OSS
- **Privacy controls**: `before_send` hook for data sanitization
- **Source maps**: Support for Rust debug symbols
- **Tauri integration**: Works well with Tauri apps (both Rust and JS sides)

#### Cons

- **Third-party dependency**: Data leaves user's machine to Sentry servers
- **Ongoing cost**: May exceed free tier as user base grows ($26/month for Team plan)
- **Network required**: Cannot capture crashes when offline (queues for later)
- **DSN exposure**: Sentry DSN must be embedded in binary (not secret, but visible)
- **Limited offline**: Offline events are stored but limited in size

#### Cost Analysis

| Tier | Price | Events/Month | Notes |
|------|-------|--------------|-------|
| Developer | Free | 5,000 | Single user, limited retention |
| Team | $26/month | 50,000 | Multiple users, 90-day retention |
| Business | $80/month | 100,000+ | SSO, extended retention |

### Option 2: Crashpad (Google)

[Crashpad](https://chromium.googlesource.com/crashpad/crashpad/) is Google's crash reporting library used in Chrome and Electron. It captures minidumps on crash.

#### Architecture

```
+-------------------+       +------------------+       +------------------+
|   GigaWhisper     |  -->  |   Crashpad       |  -->  | Local Minidump   |
|   (Rust/Tauri)    |       |   Handler        |       | Directory        |
+-------------------+       +------------------+       +------------------+
                                                              |
                                                              v
                                                       +------------------+
                                                       | User-Initiated   |
                                                       | Upload (opt-in)  |
                                                       +------------------+
                                                              |
                                                              v
                                                       +------------------+
                                                       | Custom Backend   |
                                                       | (S3/GitHub/etc)  |
                                                       +------------------+
```

#### Integration Approach

Crashpad requires C++ integration and a separate handler process:

```rust
// Would require FFI bindings or subprocess management
// crashpad_handler.exe must be bundled with the app
```

#### Pros

- **Industry standard**: Used by Chrome, Firefox, Electron
- **Rich minidumps**: Full process state, memory dumps
- **Offline-first**: Dumps stored locally, uploaded when convenient
- **No SaaS dependency**: Full control over data collection and storage
- **Proven reliability**: Handles crashes even in corrupted process states

#### Cons

- **Complex integration**: C++ library, no official Rust bindings
- **Requires backend**: Must build/host own crash collection server
- **Large minidumps**: Can be several MB per crash
- **Windows complexity**: Requires bundling `crashpad_handler.exe`
- **Symbol server**: Need to host debug symbols for symbolication
- **High maintenance**: Significant infrastructure to build and maintain
- **Poor Rust support**: Not designed for Rust panic handling

#### Infrastructure Requirements

1. Crash collection server (e.g., Mozilla Socorro, custom)
2. Symbol storage (S3 or similar)
3. Symbolication service
4. Analysis dashboard

### Option 3: Custom Solution (Logging + Manual Upload)

Build a lightweight custom solution using existing logging infrastructure with optional manual report submission.

#### Architecture

```
+-------------------+       +------------------+       +------------------+
|   GigaWhisper     |  -->  |  tracing +       |  -->  | Local Log Files  |
|   (Rust/Tauri)    |       |  panic hook      |       | (rotating)       |
+-------------------+       +------------------+       +------------------+
                                                              |
                                    +-------------------------+
                                    |                         |
                                    v                         v
                            +------------------+     +------------------+
                            | In-App Report    |     | Manual Export    |
                            | Dialog (opt-in)  |     | for GitHub Issue |
                            +------------------+     +------------------+
                                    |
                                    v
                            +------------------+
                            | GitHub Issue     |
                            | (auto-created)   |
                            +------------------+
```

#### Implementation

```rust
// src-tauri/src/crash_reporting/mod.rs
use std::panic;
use tracing::{error, info};

pub struct CrashReporter {
    enabled: bool,
    log_dir: PathBuf,
}

impl CrashReporter {
    pub fn init(enabled: bool, log_dir: PathBuf) -> Self {
        let reporter = Self { enabled, log_dir };

        if enabled {
            reporter.install_panic_hook();
        }

        reporter
    }

    fn install_panic_hook(&self) {
        let log_dir = self.log_dir.clone();

        panic::set_hook(Box::new(move |panic_info| {
            let crash_report = CrashReport::from_panic(panic_info);

            // Write to crash log
            if let Err(e) = crash_report.save_to_file(&log_dir) {
                eprintln!("Failed to save crash report: {}", e);
            }

            // Log for tracing subscriber
            error!(
                location = %crash_report.location,
                message = %crash_report.message,
                "Application panic"
            );
        }));
    }
}

#[derive(Debug, Serialize)]
pub struct CrashReport {
    pub timestamp: String,
    pub version: String,
    pub os_info: OsInfo,
    pub message: String,
    pub location: String,
    pub backtrace: String,
    pub context: HashMap<String, String>,
}

impl CrashReport {
    pub fn from_panic(info: &panic::PanicInfo) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            os_info: OsInfo::collect(),
            message: info.to_string(),
            location: info.location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown".to_string()),
            backtrace: std::backtrace::Backtrace::capture().to_string(),
            context: HashMap::new(),
        }
    }

    /// Sanitize report to remove sensitive data
    pub fn sanitize(&mut self) {
        // Remove absolute paths (replace with relative)
        self.backtrace = self.backtrace
            .lines()
            .map(|line| Self::sanitize_path(line))
            .collect::<Vec<_>>()
            .join("\n");

        // Never include audio data references
        self.context.retain(|k, _| !k.contains("audio") && !k.contains("transcription"));
    }

    fn sanitize_path(line: &str) -> String {
        // Replace user home directory with ~
        // Replace absolute Windows paths with relative
        line.replace(&std::env::var("USERPROFILE").unwrap_or_default(), "~")
    }
}

#[derive(Debug, Serialize)]
pub struct OsInfo {
    pub os: String,
    pub version: String,
    pub arch: String,
    // Intentionally not collecting: username, hostname, full paths
}
```

#### UI Component (Settings)

```tsx
// src/components/CrashReportingSettings.tsx
export function CrashReportingSettings() {
    const [enabled, setEnabled] = useState(false);
    const [pendingReports, setPendingReports] = useState<CrashReport[]>([]);

    return (
        <div>
            <h3>Crash Reporting</h3>
            <p>Help improve GigaWhisper by sending anonymous crash reports.</p>

            <Toggle
                checked={enabled}
                onChange={setEnabled}
                label="Enable crash reporting (opt-in)"
            />

            <p className="text-sm text-gray-500">
                Crash reports include: error messages, stack traces, OS version.
                <br />
                Never collected: audio recordings, transcription text, personal files.
            </p>

            {pendingReports.length > 0 && (
                <div>
                    <h4>Pending Reports ({pendingReports.length})</h4>
                    <Button onClick={reviewAndSend}>Review & Send</Button>
                    <Button variant="secondary" onClick={discard}>Discard All</Button>
                </div>
            )}
        </div>
    );
}
```

#### Report Submission Flow

```rust
// Option A: GitHub Issue via gh CLI or API
pub async fn submit_via_github(report: &CrashReport) -> Result<String, Error> {
    let client = reqwest::Client::new();

    // Note: Would need a GitHub token or use gh CLI
    // For privacy, user would authenticate themselves

    let issue_body = format!(
        "## Crash Report\n\n\
         **Version**: {}\n\
         **OS**: {} {}\n\n\
         ### Error\n```\n{}\n```\n\n\
         ### Backtrace\n```\n{}\n```",
        report.version,
        report.os_info.os,
        report.os_info.version,
        report.message,
        report.backtrace
    );

    // Return URL to created issue
    Ok(issue_url)
}

// Option B: Copy to clipboard for manual GitHub issue
pub fn copy_report_to_clipboard(report: &CrashReport) -> Result<(), Error> {
    let formatted = report.format_for_github();
    arboard::Clipboard::new()?.set_text(formatted)?;
    Ok(())
}
```

#### Pros

- **Full privacy control**: All data stays local until user explicitly shares
- **No external dependencies**: Uses existing tracing infrastructure
- **Zero cost**: No SaaS subscriptions
- **User transparency**: Users can review reports before sending
- **Simple implementation**: Leverages existing Rust ecosystem
- **Works offline**: All local, upload is user-initiated
- **GDPR-friendly**: No automatic data collection

#### Cons

- **Lower report rate**: Requires user action, most crashes won't be reported
- **No automatic deduplication**: Must manually identify duplicate issues
- **No alerting**: No real-time notifications of new crashes
- **Limited context**: Panic hook captures less than minidump
- **Manual triage**: No dashboard, analysis via GitHub issues
- **Backtrace quality**: Rust backtraces can be noisy in release builds

## Decision

**Chosen Option: Option 3 - Custom Solution (Logging + Manual Upload)** with a **migration path to Sentry** for future growth.

### Rationale

1. **Privacy-first**: GigaWhisper handles voice data; users must trust we don't collect it. A fully local solution with user-initiated sharing provides maximum trust.

2. **Right-sized for current stage**: As a new OSS project, the user base is small. A custom solution is sufficient and teaches us what data we actually need.

3. **Zero ongoing cost**: No SaaS subscriptions align with OSS sustainability.

4. **User empowerment**: Users review and submit their own reports, maintaining control.

5. **Future flexibility**: If the project grows, we can migrate to Sentry (the architecture allows adding Sentry as an additional backend).

### Hybrid Approach for Future

The implementation will be designed to support multiple backends:

```rust
pub trait CrashBackend: Send + Sync {
    fn submit(&self, report: &CrashReport) -> Result<(), Error>;
}

pub struct LocalFileBackend { /* ... */ }
pub struct GitHubIssueBackend { /* ... */ }
pub struct SentryBackend { /* ... */ }  // Future

pub struct CrashReporter {
    backends: Vec<Box<dyn CrashBackend>>,
}
```

## Consequences

### Positive

- **User trust**: Clear opt-in with no hidden data collection
- **Privacy compliance**: GDPR/CCPA-compliant by design
- **Low complexity**: Integrates with existing tracing infrastructure
- **Cost-effective**: No external services required
- **Transparency**: Users see exactly what is reported
- **Community engagement**: Users actively participate in bug reporting

### Negative

- **Lower report volume**: Many crashes will go unreported
- **Manual triage**: No automatic deduplication or alerting
- **Delayed discovery**: May not learn about crashes for days/weeks
- **Limited telemetry**: No aggregate statistics on crash frequency
- **Symbol management**: Debug symbols need manual handling

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Users don't report crashes | Make reporting easy (one-click), show value of reports |
| Critical crashes go unnoticed | Add startup check for previous crash, prompt user |
| Backtraces are unsymbolicated | Ship debug symbols in a separate download, provide symbolication guide |
| Report spam/abuse | Rate limiting, GitHub authentication |
| Privacy leak via backtrace | Aggressive path sanitization, review before send |

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

1. Create `src-tauri/src/crash_reporting/mod.rs` module
2. Implement `CrashReport` struct with sanitization
3. Add panic hook for Rust panics
4. Integrate with existing `tracing-appender` for log rotation
5. Store crash reports in `%APPDATA%/GigaWhisper/crashes/`

### Phase 2: Settings Integration (Week 1-2)

1. Add `CrashReportingSettings` to `Settings` struct:
   ```rust
   pub struct CrashReportingSettings {
       pub enabled: bool,  // Default: false
       pub include_system_info: bool,  // Default: true
       pub auto_prompt_on_crash: bool,  // Default: true
   }
   ```
2. Add UI toggle in Settings panel
3. Implement crash report review dialog

### Phase 3: Submission Mechanism (Week 2)

1. Implement "Copy to Clipboard" for GitHub issue creation
2. Add "Open GitHub Issues" button with pre-filled template
3. Store report locally until user acts or dismisses

### Phase 4: Crash Detection (Week 2-3)

1. Implement startup crash detection (check for crash marker file)
2. Show "GigaWhisper crashed last time" dialog
3. Offer to send crash report

### Phase 5: Documentation (Week 3)

1. Document what data is collected
2. Add privacy policy section
3. Create GitHub issue template for crash reports

## Settings Schema Addition

```rust
// Add to Settings struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrashReportingSettings {
    /// Enable crash report collection (opt-in, default: false)
    pub enabled: bool,

    /// Include system information in reports
    pub include_system_info: bool,

    /// Show prompt after crash on next startup
    pub prompt_after_crash: bool,

    /// Maximum number of stored crash reports
    pub max_stored_reports: u32,
}

impl Default for CrashReportingSettings {
    fn default() -> Self {
        Self {
            enabled: false,  // Opt-in
            include_system_info: true,
            prompt_after_crash: true,
            max_stored_reports: 10,
        }
    }
}
```

## File Structure

```
src-tauri/src/
  crash_reporting/
    mod.rs           # Public API and CrashReporter struct
    report.rs        # CrashReport struct and serialization
    sanitizer.rs     # Privacy sanitization utilities
    storage.rs       # Local file storage for reports
    backends/
      mod.rs
      local.rs       # LocalFileBackend
      github.rs      # GitHubIssueBackend (clipboard/URL)
```

## Alternatives Considered

### Bugsplat (Commercial)

- Similar to Sentry, $99/month minimum
- Rejected: Cost prohibitive for OSS

### Backtrace.io (Now Sauce Labs)

- Enterprise-focused, minidump collection
- Rejected: Overkill, complex pricing

### Self-hosted Sentry

- Run Sentry on own infrastructure
- Rejected: High maintenance burden, requires server

### Windows Error Reporting (WER)

- Built into Windows
- Rejected: Microsoft receives data, limited control, poor Rust support

## References

- [Sentry Rust SDK](https://docs.sentry.io/platforms/rust/)
- [Crashpad Documentation](https://chromium.googlesource.com/crashpad/crashpad/+/HEAD/doc/overview_design.md)
- [Rust panic handling](https://doc.rust-lang.org/std/panic/index.html)
- [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/)
- [GDPR and Error Tracking](https://sentry.io/security/#gdpr)
- [GigaWhisper Privacy Considerations](../PRODUCTION_READINESS.md)

## Appendix: Data Collection Reference

### Collected (when enabled)

| Data | Purpose | Sensitivity |
|------|---------|-------------|
| App version | Identify affected versions | Low |
| OS version | Environment context | Low |
| Error message | Understand crash cause | Low |
| Stack trace (sanitized) | Debug location | Medium |
| CPU architecture | Build variant debugging | Low |
| GPU info (optional) | GPU-related crashes | Low |

### Never Collected

| Data | Reason |
|------|--------|
| Audio recordings | Privacy-critical |
| Transcription text | Privacy-critical |
| API keys | Security-critical |
| File paths (absolute) | PII exposure |
| Username/hostname | PII |
| Network information | Privacy |
| Other running processes | Privacy |
