# Issue 41: Fix CI Release Workflow and Runner Matrix

**Status**: Ready
**Blocked by**: None
**Type**: Defect / Infrastructure

## Description

Fix GitHub Actions `.github/workflows/release.yml` and `install.sh` to ensure automated multi-platform release generation, resolving:
1. "Release not found" failure in `taiki-e/upload-rust-binary-action` by adding `taiki-e/create-gh-release-action@v1`.
2. 24-hour hang on `macos-13` runner by moving `x86_64-apple-darwin` to `macos-latest`.
3. Linker symbol errors on `aarch64-unknown-linux-gnu` cross-compilation by switching runner to native `ubuntu-24.04-arm`.
4. Update `install.sh` fallback release version to `v0.11.0`.

## Acceptance Criteria
- [ ] `.github/workflows/release.yml` contains `create-release` job before `distribute`.
- [ ] Runner matrix includes `ubuntu-24.04-arm` for `aarch64-unknown-linux-gnu` and `macos-latest` for both macOS architectures.
- [ ] `install.sh` fallback version is updated and passes `bash -n install.sh`.
- [ ] `k0maru guard check --diff` passes with zero violations.
- [ ] CI suite passes (`cargo clippy`, `cargo test`, `cargo fmt`).
