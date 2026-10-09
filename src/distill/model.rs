use serde::{Deserialize, Serialize};

/// Target ecosystem / agent format for crystallized skill cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DistillTarget {
    /// Default Obsidian/SecondBrain note format with standard frontmatter.
    #[default]
    Default,
    /// Nous Research Hermes Agent dynamic skill format with actionable schema frontmatter.
    Hermes,
}

impl std::str::FromStr for DistillTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "default" => Ok(DistillTarget::Default),
            "hermes" => Ok(DistillTarget::Hermes),
            other => Err(format!(
                "Unknown distill target '{}'. Supported targets: default, hermes",
                other
            )),
        }
    }
}

impl std::fmt::Display for DistillTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistillTarget::Default => write!(f, "default"),
            DistillTarget::Hermes => write!(f, "hermes"),
        }
    }
}

/// Structured representation of a distilled skill extracted from execution traces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistilledSkill {
    /// Distinct, actionable title for the skill (e.g. "Resolve SQLite-Vec Linking Failure")
    pub title: String,
    /// Environmental and operational context where issue was encountered
    pub trigger_context: String,
    /// Root cause analysis and key error signatures/snippets
    pub root_cause: String,
    /// Concrete series of commands or actions that remediated the issue
    pub remediation: String,
    /// Defensive rules and evergreen practices for agents and developers
    pub prevention_rules: Vec<String>,
    /// Tags associated with the skill (e.g. ["topic/rust", "topic/build"])
    pub tags: Vec<String>,
    /// Related notes or wiki links for graph traversal
    pub related_notes: Vec<String>,
}

impl DistilledSkill {
    /// Format the distilled skill into standardized Markdown matching Hermes Agent
    /// and LLM-Wiki bidirectional memory crystallization standards.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Ensure type tag exists
        let mut tags = self.tags.clone();
        if !tags.iter().any(|t| t == "type/skill" || t == "#type/skill") {
            tags.insert(0, "type/skill".to_string());
        }

        // Render YAML frontmatter
        md.push_str("---\n");
        md.push_str(&format!("title: \"{}\"\n", self.title.replace('"', "\\\"")));
        md.push_str("type: skill\n");
        md.push_str("hierarchy: L3Evergreen\n");
        if !tags.is_empty() {
            let clean_tags: Vec<String> = tags
                .iter()
                .map(|t| t.trim_start_matches('#').to_string())
                .collect();
            md.push_str("tags:\n");
            for tag in clean_tags {
                md.push_str(&format!("  - {}\n", tag));
            }
        }
        md.push_str("---\n\n");

        md.push_str(&self.render_body());
        md
    }

    /// Format the distilled skill into Nous Research Hermes Agent dynamic skill Markdown format.
    ///
    /// Produces YAML frontmatter containing `name`, `description`, `trigger`, `tags`,
    /// and `parameters`, enabling Hermes to treat the note as an actionable skill.
    pub fn to_hermes_markdown(&self) -> String {
        let mut md = String::new();

        let raw_slug = crate::convention::slugify(&self.title);
        let name = if raw_slug.is_empty() {
            "distilled-skill".to_string()
        } else {
            raw_slug
        };

        let description = if self.title.trim().is_empty() {
            "Distilled skill".to_string()
        } else {
            self.title.trim().replace('\n', " ")
        };

        let trigger = if self.trigger_context.trim().is_empty() {
            "常规排障与环境初始化".to_string()
        } else {
            self.trigger_context.trim().replace('\n', " ")
        };

        let mut tags = self.tags.clone();
        if !tags.iter().any(|t| t == "type/skill" || t == "#type/skill") {
            tags.insert(0, "type/skill".to_string());
        }

        md.push_str("---\n");
        md.push_str(&format!("name: {}\n", name));
        md.push_str(&format!("description: {}\n", description));
        md.push_str(&format!("trigger: {}\n", trigger));
        md.push_str("parameters:\n  type: object\n  properties: {}\n");

        let clean_tags: Vec<String> = tags
            .iter()
            .map(|t| t.trim_start_matches('#').to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if !clean_tags.is_empty() {
            md.push_str("tags:\n");
            for tag in clean_tags {
                md.push_str(&format!("  - {}\n", tag));
            }
        }
        md.push_str("---\n\n");

        md.push_str(&self.render_body());
        md
    }

    /// Render core content sections (Trigger, Root Cause, Remediation, Prevention Rules, WikiLinks).
    pub fn render_body(&self) -> String {
        let mut md = String::new();

        // Title heading
        md.push_str(&format!("# {}\n\n", self.title));

        // Section 1: Trigger Context
        md.push_str("## 🎯 触发上下文与应用场景 (Trigger Context)\n\n");
        if self.trigger_context.trim().is_empty() {
            md.push_str("未指定具体触发上下文。适用于常规排障与环境初始化。\n\n");
        } else {
            md.push_str(self.trigger_context.trim());
            md.push_str("\n\n");
        }

        // Section 2: Root Cause & Signatures
        md.push_str("## 🔍 故障根因与错误特征 (Root Cause & Signatures)\n\n");
        if self.root_cause.trim().is_empty() {
            md.push_str("未提取到特征性错误堆栈。\n\n");
        } else {
            md.push_str(self.root_cause.trim());
            md.push_str("\n\n");
        }

        // Section 3: Remediation & Commands
        md.push_str("## 🛠️ 修复策略与执行命令 (Remediation & Commands)\n\n");
        if self.remediation.trim().is_empty() {
            md.push_str("请根据错误特征逐步回溯验证。\n\n");
        } else {
            md.push_str(self.remediation.trim());
            md.push_str("\n\n");
        }

        // Section 4: Prevention Rules
        md.push_str("## 🛡️ 防范规约与常青法则 (Prevention Rules & Best Practices)\n\n");
        if self.prevention_rules.is_empty() {
            md.push_str("- 在执行破坏性操作前始终核验系统与依赖环境状态。\n\n");
        } else {
            for rule in &self.prevention_rules {
                md.push_str(&format!("- {}\n", rule));
            }
            md.push('\n');
        }

        // Optional WikiLinks footer
        if !self.related_notes.is_empty() {
            md.push_str("## 🔗 关联知识双链 (Networked Knowledge)\n\n");
            for note in &self.related_notes {
                let clean = note.trim();
                if clean.starts_with("[[") && clean.ends_with("]]") {
                    md.push_str(&format!("- {}\n", clean));
                } else {
                    md.push_str(&format!("- [[{}]]\n", clean));
                }
            }
            md.push('\n');
        }

        md
    }
}
