# Contributing to lightning-payjoin-kit

First off, thank you for considering contributing to `lightning-payjoin-kit`! This project is built as open-source public infrastructure for the Bitcoin ecosystem, and community contributions are crucial to its success.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. We expect all contributors to be respectful, collaborative, and professional in all interactions.

## Developer Certificate of Origin (DCO)

All contributions to this project must be accompanied by a Developer Certificate of Origin (DCO) sign-off. This ensures that you have the right to submit the code under the project's license.

To sign off on a commit, use the `-s` or `--signoff` flag when running `git commit`:

```bash
git commit -s -m "feat: Add UTXO selection algorithm"
```

## How to Contribute

### 1. Report Bugs
If you find a bug, please open an issue describing the problem, how to reproduce it, and the expected behavior. Include your environment details (Rust version, OS, etc.).

### 2. Suggest Enhancements
We welcome ideas for new features or improvements. Please open an issue to discuss your proposed changes before writing any code. This ensures alignment with the project roadmap and saves your time.

### 3. Submit Pull Requests
1. Fork the repository.
2. Create a new branch for your feature or bug fix (`git checkout -b feature/your-feature-name`).
3. Make your changes and ensure tests pass (`cargo test`).
4. Commit your changes with a descriptive commit message and a DCO sign-off (`git commit -s`).
5. Push to your fork and submit a Pull Request against the `main` branch.

## Development Setup

1. **Install Rust:** Ensure you have the latest stable version of Rust installed (1.75+).
2. **Clone the repo:** `git clone https://github.com/ILE-Labs/lightning-payjoin-kit.git`
3. **Run tests:** `cargo test`
4. **Format code:** We enforce standard Rust formatting. Run `cargo fmt` before committing.
5. **Lint code:** Ensure your code passes all clippy checks without warnings: `cargo clippy -- -D warnings`

## Architecture and Protocol Design

Before making significant changes, please read the [ARCHITECTURE.md](./docs/ARCHITECTURE.md) to understand the core design principles, specifically the async relay coordination and BIP-78 adaptation.

## Review Process

- All PRs require at least one approval from a core maintainer.
- CI must pass (tests, formatting, clippy).
- Documentation must be updated if relevant.

Thank you for helping us improve Bitcoin privacy!
