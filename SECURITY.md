# Security Policy

The `lightning-payjoin-kit` team takes the security of our cryptographic infrastructure and the privacy of Lightning Network users seriously. We appreciate your efforts to responsibly disclose your findings.

## Supported Versions

Currently, the library is under active development. Once stable versions are released, we will list the supported versions here.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Scope

The scope of this security policy covers the core `lightning-payjoin-kit` Rust crate, specifically:
- Payjoin protocol implementation and PSBT parsing
- UTXO selection algorithms regarding privacy leaks
- Async relay coordination mechanisms and encryption
- Any cryptographic key derivation or signature handling within the repository

## Out of Scope

- Weaknesses in the Lightning Network protocol itself (BOLTs)
- Vulnerabilities in external dependencies (e.g., `rust-bitcoin`, `ldk`) — please report these to their respective maintainers.
- Bugs in the broader Bitcoin network.

## Reporting a Vulnerability

If you discover a security vulnerability within `lightning-payjoin-kit`, please do NOT open a public issue.

Instead, please send an email to our security team at:
**contact@ilelab.org**

Please include the following information in your email:
1. A detailed description of the vulnerability and its potential impact.
2. Step-by-step instructions to reproduce the issue.
3. Any proof-of-concept code, if applicable.
4. Suggestions for mitigation or resolution, if you have any.

### Response Timeline

- We will acknowledge receipt of your vulnerability report within **48 hours**.
- We will provide a status update or an initial assessment within **1 week**.
- We strive to resolve critical issues and publish a patch within **14 days** of confirmation.

## Public Disclosure

We ask that you maintain confidentiality until we have patched the vulnerability and published a security advisory. After the fix is deployed, we will coordinate public disclosure and gladly acknowledge your contribution (unless you prefer to remain anonymous).

Thank you for helping us protect the Lightning Network ecosystem.
