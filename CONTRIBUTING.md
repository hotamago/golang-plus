# Contributing to GoPlus

First off, thank you for considering contributing to GoPlus! We welcome all contributions, whether they are bug fixes, new features, documentation improvements, or just feedback.

## Getting Started

### Prerequisites

To develop the GoPlus compiler, you will need:
- **Rust**: We recommend using the latest stable version of Rust (1.80.0 or higher). You can install it via [rustup](https://rustup.rs/).
- **Git**: To clone the repository and manage your changes.

### Setting up the Environment

1. **Clone the repository:**
   ```bash
   git clone https://github.com/hotamago/golang-plus.git
   cd golang-plus
   ```

2. **Build the compiler:**
   ```bash
   cargo build
   ```

3. **Run tests:**
   We have a comprehensive suite of unit and integration tests. To run them:
   ```bash
   cargo test
   ```

4. **Run benchmarks:**
   ```bash
   cargo bench
   ```

## Development Workflow

1. Fork the repository and create a new branch for your feature or bug fix.
2. If you are fixing a bug, please add a test case that reproduces the bug before fixing it.
3. Ensure your code passes all existing tests (`cargo test`).
4. Ensure your code is formatted according to Rust's standard formatting (`cargo fmt`).
5. Ensure there are no linting errors (`cargo clippy`).
6. Submit a Pull Request!

## Good First Issues and Help Wanted

If you're new to the project and looking for a place to start, check out the issues labeled [`good first issue`](https://github.com/hotamago/golang-plus/labels/good%20first%20issue) or [`help wanted`](https://github.com/hotamago/golang-plus/labels/help%20wanted). These labels are used to highlight tasks that are specifically suited for new contributors. They are typically smaller in scope, well-documented, and an excellent way to get familiar with the codebase.

## Code of Conduct

Please be respectful and considerate of others when interacting with the project and its community.
