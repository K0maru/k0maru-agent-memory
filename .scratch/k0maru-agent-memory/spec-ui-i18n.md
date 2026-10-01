# Specification: K0maru UI Bilingual Internationalization (`i18n`)

## Problem Statement

The Developer Dark Cockpit in `k0maru ui` currently displays all navigational items, status labels, search debugger modes, graph controls, log inspector headers, and scoreboard metrics exclusively in English. 

For developers, Chinese-speaking users, and multilingual teams, this presents two friction points:
1. **Terminology Accessibility**: Users navigating semantic hierarchies (e.g. `L3 Evergreen`, `L2 Log`, `L1 Resource`, `L0 Daily`), graph inspector drawer (Inbound Backlinks, Out-links), or search explainability metrics benefit from clear localized terminology (e.g., `双向反向链接`, `知识图谱加权`, `上下文卸载`).
2. **Seamless Switching**: Users need to seamlessly switch between English (`en`) and Simplified Chinese (`zh-CN`) with a single click in the cockpit header, with their language preference persisted across browser sessions in `localStorage`.

## Solution

Implement a zero-dependency, pure client-side bilingual internationalization (`i18n`) engine integrated into the static dashboard bundle (`index.html`, `style.css`, `app.js`):

- **Header Switcher UI**: An ergonomic language toggle pill in the cockpit header (e.g., `🌐 中文 / EN`), styled to match the Developer Dark Cockpit design tokens (`#1B2336`, `#334155`, `#22C55E`, `#F8FAFC`).
- **Persistence & Auto-detection**:
  - Checks `localStorage.getItem('k0maru_lang')`.
  - If not set, checks browser preference `navigator.language` (defaults to `zh` if starting with `zh`, otherwise `en`).
  - Allows manual toggling between `zh` and `en` at any time without page reload.
  - Updates `document.documentElement.lang` accordingly.
- **Coverage**:
  - **Header & Global**: Brand subtitle, Vault path pill label, Docs label, Vectors label, Synced timestamp label, "Sync Vault" button states (Sync Vault / 同步知识库, Syncing... / 同步中..., Synced / 已同步, Error / 同步失败).
  - **Tabs**: Search Debugger (多路检索调试), Graph Explorer (知识图谱图鉴), Log Inspector (日志与状态机透视), Token Scoreboard (Token 节省计分板).
  - **Panel 1: Search Debugger**: Search placeholder, shortcut hint `[ / 或 ⌘K ]`, search modes (`Hybrid (RRF 倒数融合)`, `BM25 词法`, `Vector 语义向量`), limit label, query latency, explainability badges (`RRF 得分`, `BM25 位次`, `向量位次`, `+0.05 图谱加权`), empty states.
  - **Panel 2: Graph Explorer**: Search nodes placeholder, hierarchy filter pills (`项目`, `L3 永恒笔记`, `L2 决策日志`, `L1 资源引用`, `L0 日志`), highlight orphans (`突出孤立笔记`), zoom controls, node counter format, side drawer labels (`笔记路径`, `知识层级`, `标签`, `出链引用`, `反向链接 Backlinks`, `无反向链接`).
  - **Panel 3: Log Inspector**: Search logs placeholder, empty states, node detail header, copy buttons (`复制 Node ID`, `复制原始日志`, `已复制!`), diagram viewer header, raw log slice header, error lines legend.
  - **Panel 4: Token Scoreboard**: KPI card labels (`索引 Markdown 文档`, `向量索引与覆盖率`, `SQLite 缓存体积`, `已卸载日志节点`), Token Economics card (`累计节省 Token`, `上下文压缩率 TRR`, `基线评测对齐 Phase 3.8`), Cache Sync card (`增量同步`, `就绪`, `正在执行全量指纹比对与向量嵌入...`).
- **Engineering Baseline**:
  - Zero external CDN dependencies (100% offline self-contained dictionary in `app.js` / `i18n.js`).
  - Non-breaking: Embedded binary builds and existing automated tests remain fully compatible and green.
