# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in GigaWhisper, please report it responsibly.

### How to Report

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please send an email to: **security@gigawhisper.app** (or create a private security advisory on GitHub)

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Resolution Target**: Within 30 days (depending on complexity)

### What to Expect

1. We will acknowledge receipt of your report
2. We will investigate and validate the issue
3. We will work on a fix and coordinate disclosure
4. We will credit you in the release notes (unless you prefer anonymity)

## Security Measures

GigaWhisper implements the following security measures:

### Data Privacy

- **Local-first**: Transcriptions are processed locally by default using Whisper
- **No telemetry**: We do not collect any usage data or analytics
- **Optional cloud**: Groq API is opt-in and requires user-provided API key
- **API keys stored securely**: Using OS credential manager (Windows Credential Manager)

### Application Security

- **Code signing**: Windows executables are signed (when certificate is configured)
- **Auto-updates**: Signed updates via Tauri's secure update mechanism
- **Input validation**: All user inputs are validated and sanitized
- **Path traversal protection**: File operations are restricted to allowed directories

### Dependencies

- Regular security audits via `cargo audit` and `pnpm audit`
- Automated dependency updates monitoring
- Minimal dependency footprint

## Security Best Practices for Users

1. **Keep GigaWhisper updated** to receive security patches
2. **Protect your Groq API key** - don't share it publicly
3. **Download only from official sources**: GitHub Releases or gigawhisper.app
4. **Verify installer signatures** before installation

## Scope

The following are **in scope** for security reports:

- GigaWhisper desktop application
- Official installers and update mechanism
- Data handling and storage

The following are **out of scope**:

- Third-party dependencies (report to their maintainers)
- Groq API security (report to Groq)
- Social engineering attacks
- Physical access attacks
