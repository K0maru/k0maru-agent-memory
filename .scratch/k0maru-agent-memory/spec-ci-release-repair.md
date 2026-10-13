# 规格文档：CI/CD 发版工作流自愈与全平台矩阵修复规范 (Phase 41)

## 1. 背景与问题定义 (Problem Statement)

用户在 Linux 终端执行一键安装脚本时报错：
```bash
curl: (22) The requested URL returned error: 404
[error] Failed to download release binary for x86_64-unknown-linux-gnu (v0.8.1).
```

经全链路排查，根因包含三层：
1. **历史 Release 资产缺失**：GitHub Release `v0.8.1` 只有 macOS ARM64 和 Windows x86_64 资产，缺少 Linux 构建。因为当时 CI 目标为 `x86_64-unknown-linux-musl`，缺少 musl OpenSSL 导致编译崩溃（Exit code 101）；
2. **后续版本无法发布 Release 对象**：虽然代码库后续修复了 musl 问题并切换为 `gnu`，且打出了 `v0.8.2` ~ `v0.11.0` 等 Tag，但在 `.github/workflows/release.yml` 中直接使用 `taiki-e/upload-rust-binary-action`，缺少先置创建 Release 的步骤（`taiki-e/create-gh-release-action`），导致上传时报 `release not found` 全部失败；
3. **Runner 资源饥饿与跨平台编译器冲突**：
   - 矩阵中包含已无 runner 资源的 `macos-13`，单任务排队超时 24 小时阻断队列；
   - `aarch64-unknown-linux-gnu` 在 x86 Ubuntu 上用 cross 交叉编译时，ONNX Runtime（`libort_sys`）遇到 `std::__throw_bad_array_new_length` 与 `__libc_single_threaded` 的 C++ 运行时/glibc 符号缺失问题。

## 2. 改造方案与设计细节 (Architecture & Implementation)

### 2.1 工作流改造 (`.github/workflows/release.yml`)

1. **引入先置任务 `create-release`**：
   - 使用 `taiki-e/create-gh-release-action@v1`；
   - 提取 `CHANGELOG.md` 自动生成版本更新说明；
   - 赋予 `contents: write` 权限。
2. **重构分发任务 `distribute`**：
   - 增加 `needs: create-release` 依赖；
   - 优化构建矩阵：
     - `aarch64-apple-darwin`: `macos-latest`
     - `x86_64-apple-darwin`: `macos-latest` (由 Apple Silicon runner 原生 Clang/Rosetta 编译，废弃排队饥饿的 `macos-13`)
     - `x86_64-unknown-linux-gnu`: `ubuntu-latest`
     - `aarch64-unknown-linux-gnu`: `ubuntu-24.04-arm` (使用 GitHub Actions 官方免费原生 ARM64 Linux runner，彻底消除 cross 交叉编译的 ONNX C++ 符号缺失)
     - `x86_64-pc-windows-msvc`: `windows-latest`
   - 赋予 `contents: write` 权限。

### 2.2 客户端安装脚本防御增强 (`install.sh`)

1. 将 API 获取失败时的 fallback 版本从旧的 `v0.6.0` 升级为当前里程碑版本 `v0.11.0`；
2. 保持对 `x86_64-unknown-linux-gnu`、`aarch64-unknown-linux-gnu`、`aarch64-apple-darwin`、`x86_64-apple-darwin` 的全平台检测与校验。

## 3. 验收标准 (Verification Criteria)

1. `release.yml` 语法格式与 Action Schema 验证无误；
2. `install.sh` 本地语法校验通过（`bash -n install.sh`）；
3. 执行 `k0maru guard check --diff` 活体架构哨兵 100% 验证通过；
4. 保证 `cargo test`、`cargo clippy`、`cargo fmt` 全绿无告警；
5. 在 `dev` 分支完成修改，向人类汇报实测与发布步骤，待人类确认后再决定是否合并发版。
