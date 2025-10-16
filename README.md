# SecretScout

A Rust-based GitHub Action for detecting secrets, passwords, API keys, and tokens in git repositories. This is a high-performance port of gitleaks-action, compiled to both native binaries (crates) and WebAssembly (WASM).

## 📚 Documentation

Complete technical specifications and implementation guides are available in the [`/docs`](./docs) directory:

- **[DOCUMENTATION_INDEX.md](./docs/DOCUMENTATION_INDEX.md)** - Start here! Complete navigation guide
- **[INTEGRATION_README.md](./docs/INTEGRATION_README.md)** - Quick reference and lookup guide
- **[ANALYSIS_SUMMARY.md](./docs/ANALYSIS_SUMMARY.md)** - Executive summary and implementation roadmap
- **[SPARC_SPECIFICATION.md](./docs/SPARC_SPECIFICATION.md)** - Complete SPARC methodology specification

### Quick Start

1. Read the [Documentation Index](./docs/DOCUMENTATION_INDEX.md) to understand the document suite
2. Follow the [Integration README](./docs/INTEGRATION_README.md) for navigation
3. Review the [Analysis Summary](./docs/ANALYSIS_SUMMARY.md) for key findings and roadmap
4. Dive into [SPARC Specification](./docs/SPARC_SPECIFICATION.md) for complete requirements

## 🎯 Project Overview

SecretScout provides:
- **Automated secret scanning** on push, pull request, and scheduled events
- **SARIF-formatted security reports** for standardized output
- **Inline PR comments** for detected secrets
- **GitHub Actions job summaries** with detailed findings
- **Rust performance** with WASM portability

## 🚀 Status

**Phase**: Specification Complete ✅

The SPARC Specification phase has been completed. All functional requirements, technical specifications, integration points, and deployment requirements have been documented.

## 📖 Documentation Stats

- **Total Documentation**: 216KB across 8 documents
- **Total Content**: ~46,000 words, 150+ pages
- **Coverage**: 100% of original action functionality
- **Diagrams**: 13 detailed flow diagrams
- **Examples**: 50+ code examples and test cases

## 🛠️ Technology Stack

- **Language**: Rust 2021 edition
- **Targets**: Native binaries + WebAssembly (wasm32-unknown-unknown)
- **Runtime**: Node.js 20/24 (GitHub Actions)
- **Distribution**: Crates, WASM, npm package

## 📋 Next Steps

1. Review and approve specification
2. Set up Rust project structure
3. Implement Phase 1: Core functionality
4. Follow the 5-phase implementation roadmap (see ANALYSIS_SUMMARY.md)

## 📄 License

See [LICENSE](./LICENSE) file for details.

## 🤝 Contributing

Please refer to the [documentation](./docs) for implementation guidelines and technical specifications.

---

**Documentation Version**: 1.0
**Last Updated**: October 15, 2025
**Status**: Specification Phase Complete
