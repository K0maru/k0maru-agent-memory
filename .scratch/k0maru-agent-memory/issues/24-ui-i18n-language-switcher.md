# 24 — Bilingual Internationalization and Language Switcher for UI Cockpit

**What to build:**
Implement bilingual internationalization (`i18n`) supporting Simplified Chinese (`zh-CN` / `zh`) and English (`en-US` / `en`) with an in-header language switcher pill and full DOM text translation across all 4 cockpit views.

**Blocked by:** 23 (Log Trace Inspector & Token Scoreboard)

**Status:** resolved

- [x] Header Language Switcher:
  - Add language toggle button in cockpit header (`#btn-lang-toggle` with `#lang-indicator`).
  - Style with Developer Dark Cockpit tokens (`#1B2336`, `#334155`, `#22C55E`, `#F8FAFC`, `.btn-lang`, `.btn-secondary`).
  - Persist chosen language in `localStorage` under key `k0maru_lang`.
  - Auto-detect browser default via `navigator.language` if no stored preference.
- [x] Client-side I18n Engine & Localization Dictionary:
  - Create self-contained `TRANSLATIONS` dictionary with complete `en` and `zh` translation keys for:
    - Cockpit header: brand subtitle, status pills (Vault, Docs, Vectors, Synced), Sync Vault button states (Sync Vault / 同步知识库, Syncing... / 同步中..., Synced / 已同步, Error / 同步失败).
    - Navigation tabs: Search Debugger (多路检索调试), Graph Explorer (知识图谱图鉴), Log Inspector (日志与状态机透视), Token Scoreboard (Token 节省计分板).
    - Panel 1 Search Debugger: search input placeholder, shortcut hint, mode buttons (`Hybrid / 混合检索`, `BM25 / 词法检索`, `Vector / 语义向量`), limit slider, latency, explainability cards & badges (`RRF 得分`, `BM25 排名`, `向量排名`, `+0.05 图谱加权`), empty states.
    - Panel 2 Graph Explorer: search nodes placeholder, hierarchy filters (`项目`, `L3 永恒笔记`, `L2 决策日志`, `L1 资源引用`, `L0 日志`), highlight orphans (`高亮孤立笔记`), zoom controls, node counter (`{nodes} nodes · {links} links` / `{nodes} 节点 · {links} 连线`), side drawer labels (`选择笔记节点`, `知识层级`, `标签`, `出链引用`, `反向链接 Backlinks`, `无反向链接`).
    - Panel 3 Log Inspector: search placeholder, master list empty states, node detail header, copy buttons (`复制 Node ID`, `复制原始日志`, `已复制!`), diagram viewer header, raw log viewer header, syntax highlights legend.
    - Panel 4 Token Scoreboard: KPI metric cards (`索引 Markdown 文档`, `向量索引与覆盖率`, `SQLite 缓存体积`, `已卸载日志节点`), Token Economics card (`累计节省 Token`, `上下文压缩率 TRR`, `基线评测对齐 Phase 3.8`), Sync card (`增量同步`, `就绪`), toast notifications.
  - Implement `setLanguage(lang)` function that dynamically translates both static DOM elements (`data-i18n`, `data-i18n-placeholder`, `data-i18n-title`) and dynamically generated elements (search results cards, graph drawer, logs detail, toasts, status metrics).
- [x] Integration & Static Embed Tests:
  - Added `tests/test_ui_i18n.rs` verifying embedded `index.html` attributes and `#btn-lang-toggle`, `style.css` `.btn-lang` tokens, and `app.js` `TRANSLATIONS` engine dictionaries and bilingual terms.
- [x] Quality Assurance:
  - All 144 tests pass without regression.
  - `cargo clippy --all-targets -- -D warnings` zero warnings.
  - `cargo fmt --check` zero formatting errors.
