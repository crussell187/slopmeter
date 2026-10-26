# Install the Rust toolchain on this machine

Type: task
Status: resolved
Blocked by: 

## Question

This machine has no `rustc` or `cargo` (checked 2026-10-26; Node 24 and Python 3.9 are present). Install a stable Rust toolchain via rustup, confirm `cargo --version`, and add `cargo-llvm-cov` (needed for Rust coverage) and `rustfmt`/`clippy` components. Record the toolchain version in the answer. HITL if the agent may not run the installer; otherwise AFK.

## Answer

Resolved 2026-10-26, AFK. The official rustup installer ran non-interactively (`sh -s -- -y --default-toolchain stable --component rustfmt,clippy`). A fresh `zsh` sees `cargo` with no extra setup.

- **Host:** `aarch64-apple-darwin`. Default toolchain `stable-aarch64-apple-darwin`.
- **rustup** 1.29.2 (d8c92c676 2026-10-21). Homes: `~/.rustup`, `~/.cargo`.
- **rustc** 1.99.1 (58cf8f9c3 2026-10-13).
- **cargo** 1.99.1 (5f94df478 2026-08-27). Confirmed with `cargo --version`.
- **rustfmt** 1.10.0-stable (58cf8f9c35 2026-10-13), component `rustfmt-aarch64-apple-darwin`.
- **clippy** 0.1.99 (58cf8f9c35 2026-10-13), component `clippy-aarch64-apple-darwin`.
- **llvm-tools-preview** (`llvm-tools-aarch64-apple-darwin`), added because `cargo-llvm-cov` shells out to `llvm-cov` / `llvm-profdata`. `llvm-cov` reports LLVM 23.1.1-rust-1.99.1-stable. [Coverage formats: coverage.py LCOV, cargo-llvm-cov, gcov/lcov](02-coverage-formats.md) reproduced its LCOV notes against llvm-cov 21; this machine is on 23.1.1.
- **cargo-llvm-cov** 0.9.1, installed with `cargo install` to `~/.cargo/bin/cargo-llvm-cov`. `cargo llvm-cov --version` prints `cargo-llvm-cov 0.9.1`.
- **PATH:** rustup appended `. "$HOME/.cargo/env"` to `~/.profile` and wrote the same line into `~/.zshenv`, which now contains only that line. `~/.zprofile` and `~/.zshrc` were left unchanged.
