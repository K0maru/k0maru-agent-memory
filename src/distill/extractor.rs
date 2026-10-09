use crate::distill::model::DistilledSkill;

/// Heuristic and pattern extractor that transforms raw execution logs into a structured skill.
pub struct SkillExtractor;

impl SkillExtractor {
    /// Extract a structured `DistilledSkill` from a raw execution trace, optional context hint,
    /// and optional explicit title.
    pub fn extract(
        raw_trace: &str,
        context_hint: Option<&str>,
        explicit_title: Option<&str>,
    ) -> DistilledSkill {
        let lines: Vec<&str> = raw_trace.lines().map(|l| l.trim_end()).collect();

        // 1. Identify primary language / toolchain
        let mut tags = vec!["type/skill".to_string()];
        let has_rust = raw_trace.contains("cargo ")
            || raw_trace.contains("error[E")
            || raw_trace.contains("could not compile")
            || raw_trace.contains(".rs:");
        let has_python = raw_trace.contains("python")
            || raw_trace.contains("Traceback (most recent call last):")
            || raw_trace.contains(".py\", line")
            || raw_trace.contains("KeyError")
            || raw_trace.contains("ImportError");
        let has_node = raw_trace.contains("npm ")
            || raw_trace.contains("yarn ")
            || raw_trace.contains("node_modules")
            || raw_trace.contains(".ts:")
            || raw_trace.contains(".js:");
        let has_c = raw_trace.contains("ld: symbol(s) not found")
            || raw_trace.contains("clang")
            || raw_trace.contains("gcc")
            || raw_trace.contains("undefined reference");
        let has_go = raw_trace.contains("fatal error: all goroutines are asleep")
            || raw_trace.contains("goroutine ")
            || raw_trace.contains("go test")
            || raw_trace.contains("go run");

        if has_rust {
            tags.push("topic/rust".to_string());
        }
        if has_python {
            tags.push("topic/python".to_string());
        }
        if has_node {
            tags.push("topic/node".to_string());
        }
        if has_c {
            tags.push("topic/c-cpp".to_string());
        }
        if has_go {
            tags.push("topic/go".to_string());
        }

        // 2. Extract command history
        let mut commands_before_error = Vec::new();
        let mut commands_after_error = Vec::new();
        let mut error_seen = false;

        for line in &lines {
            let trimmed = line.trim();
            let is_cmd = trimmed.starts_with("$ ")
                || trimmed.starts_with("> ")
                || trimmed.starts_with("cargo ")
                || trimmed.starts_with("python3 ")
                || trimmed.starts_with("python ")
                || trimmed.starts_with("pytest ")
                || trimmed.starts_with("git ")
                || trimmed.starts_with("brew ");

            if is_cmd {
                let cmd_str = if trimmed.starts_with("$ ") || trimmed.starts_with("> ") {
                    trimmed[2..].trim().to_string()
                } else {
                    trimmed.to_string()
                };

                if !error_seen {
                    commands_before_error.push(cmd_str);
                } else {
                    commands_after_error.push(cmd_str);
                }
            }

            if trimmed.contains("error[E")
                || trimmed.contains("error:")
                || trimmed.contains("Traceback")
                || trimmed.contains("command not found")
                || trimmed.contains("FAILED")
                || trimmed.contains("exit status")
            {
                error_seen = true;
            }
        }

        // 3. Extract Root Cause & Error Signatures
        let mut error_lines = Vec::new();
        let mut in_error_block = false;
        let mut error_summary = String::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Rust error diagnostics
            if trimmed.starts_with("error[E") || trimmed.starts_with("error:") {
                in_error_block = true;
                if error_summary.is_empty() {
                    error_summary = trimmed.to_string();
                }
            } else if trimmed.starts_with("Traceback (most recent call last):") {
                in_error_block = true;
                // Peek ahead for actual error name
                for forward in lines.iter().skip(i).take(15) {
                    let ft = forward.trim();
                    if ft.contains("Error:") || ft.contains("Exception:") {
                        error_summary = ft.to_string();
                        break;
                    }
                }
                if error_summary.is_empty() {
                    error_summary = "Python Traceback Exception".to_string();
                }
            } else if trimmed.contains("command not found")
                || trimmed.starts_with("fatal error:")
                || trimmed.starts_with("panic:")
            {
                in_error_block = true;
                if error_summary.is_empty() {
                    error_summary = trimmed.to_string();
                }
            } else if trimmed.contains("Invalid hook call") {
                in_error_block = true;
                if error_summary.is_empty() {
                    error_summary = "React Invalid hook call".to_string();
                }
            } else if (trimmed.contains("exit status") || trimmed.contains("exit code"))
                && error_summary.is_empty()
            {
                error_summary = trimmed.to_string();
            }

            if in_error_block {
                error_lines.push(*line);
                // Cut off error block when seeing new command or success
                if (trimmed.starts_with("$ ")
                    || trimmed.starts_with("> ")
                    || trimmed.starts_with("test result:"))
                    && error_lines.len() > 2
                {
                    in_error_block = false;
                }
                if error_lines.len() >= 25 {
                    in_error_block = false;
                }
            }
        }

        // If no explicit error line was detected, fallback
        if error_summary.is_empty() {
            if let Some(hint) = context_hint {
                error_summary = hint.to_string();
            } else {
                error_summary = "执行中断或排障检查".to_string();
            }
        }

        // 4. Formulate Title
        let title = if let Some(et) = explicit_title {
            et.trim().to_string()
        } else if let Some(hint) = context_hint {
            if hint.len() <= 60 {
                hint.trim().to_string()
            } else {
                format!("排障处理: {}", &hint[..60].trim())
            }
        } else if !error_summary.is_empty() && error_summary != "执行中断或排障检查" {
            // Clean up error summary for title
            let clean = error_summary
                .trim_start_matches("error: ")
                .trim_start_matches("zsh: ")
                .trim_start_matches("bash: ")
                .trim();
            if clean.len() <= 60 {
                format!("解决 {}", clean)
            } else {
                format!("解决 {}", &clean[..57])
            }
        } else {
            "执行故障排查与经验提炼".to_string()
        };

        // 5. Formulate Trigger Context
        let mut trigger_context = String::new();
        if let Some(hint) = context_hint {
            trigger_context.push_str(hint.trim());
            trigger_context.push_str("\n\n");
        }
        if !commands_before_error.is_empty() {
            trigger_context.push_str("执行触发命令：\n```bash\n");
            for cmd in &commands_before_error {
                trigger_context.push_str(&format!("$ {}\n", cmd));
            }
            trigger_context.push_str("```\n");
        } else if trigger_context.is_empty() {
            trigger_context = "在系统执行构建或任务排障时触发。".to_string();
        }

        // 6. Formulate Root Cause
        let mut root_cause = String::new();
        root_cause.push_str(&format!("**故障特征**：`{}`\n\n", error_summary));
        if !error_lines.is_empty() {
            root_cause.push_str("```text\n");
            for el in &error_lines {
                root_cause.push_str(el);
                root_cause.push('\n');
            }
            root_cause.push_str("```\n");
        } else if !raw_trace.trim().is_empty() {
            root_cause.push_str("```text\n");
            for line in lines.iter().take(15) {
                root_cause.push_str(line);
                root_cause.push('\n');
            }
            root_cause.push_str("```\n");
        }

        // 7. Formulate Remediation & Commands
        let mut remediation = String::new();
        if !commands_after_error.is_empty() {
            remediation.push_str("执行以下修复操作以纠正问题：\n```bash\n");
            for cmd in &commands_after_error {
                remediation.push_str(&format!("$ {}\n", cmd));
            }
            remediation.push_str("```\n");
        } else {
            remediation.push_str(
                "根据上述报错特征针对性核对依赖、环境变量或代码所有权逻辑，完成复测验证。\n",
            );
        }

        // 8. Formulate Prevention Rules
        let mut prevention_rules = Vec::new();
        if has_rust {
            if error_summary.contains("E0382") || error_summary.contains("moved") {
                prevention_rules.push(
                    "注意 Rust 所有权与借用检查，避免在变量移动后继续在原位置解引用或调用；跨线程共享需显式包裹 Arc/Mutex。".to_string(),
                );
            }
            prevention_rules.push(
                "代码改动后优先运行 `cargo check` 与 `cargo test` 进行本地类型与单测拦截。"
                    .to_string(),
            );
        }
        if has_python {
            if error_summary.contains("KeyError")
                || error_summary.contains("TOKEN")
                || error_summary.contains("KEY")
            {
                prevention_rules.push("对于外部依赖环境变量，建议使用 `.get('KEY')` 优雅回退或在程序启动前执行必填环境变量自检。".to_string());
            }
            prevention_rules
                .push("在运行脚本前通过虚拟环境与 `pip freeze` 固化依赖版本。".to_string());
        }
        if error_summary.contains("command not found") {
            prevention_rules.push("在脚手架与自动化脚本中加入命令存在性前置探针（如 `which <cmd>` 或 `k0maru doctor`）。".to_string());
            prevention_rules.push(
                "在项目 README / QUICKSTART 中显式标明系统环境预装工具与 Homebrew 安装路径。"
                    .to_string(),
            );
        }
        if prevention_rules.is_empty() {
            prevention_rules
                .push("遇到非预期失败时，保存现场日志切片并核查入参条件与环境上下文。".to_string());
            prevention_rules.push("修复后补充自动化回归用例，杜绝同类故障重复发生。".to_string());
        }

        DistilledSkill {
            title,
            trigger_context,
            root_cause,
            remediation,
            prevention_rules,
            tags,
            related_notes: Vec::new(),
        }
    }
}
