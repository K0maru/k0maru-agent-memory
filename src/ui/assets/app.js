/**
 * K0maru Agent Memory Hub — Developer Cockpit Frontend (v1.1.0)
 * Visual Direction: Retro-Tech Low-Saturation Meadow & Warm Obsidian
 * Zero-Emoji & Zero-PII Policy Compliant
 */

(function () {
  'use strict';

  // --------------------------------------------------------------------------
  // Application State & Configuration
  // --------------------------------------------------------------------------
  let currentLang = 'en';

  const appState = {
    vaultPath: '',
    recentVaults: [],
    status: null,
    config: null,
    activeTab: 'search',
    logs: [],
    selectedLogId: null,
    guardRules: [],
    clients: [],
    timelineItems: [],
    activeTimelineFilter: 'all',
  };

  // --------------------------------------------------------------------------
  // Bilingual I18n Translations Dictionary
  // --------------------------------------------------------------------------
  const TRANSLATIONS = {
    en: {
      'brand.subtitle': 'LLM-Wiki Zero-Daemon Memory Cockpit',
      'vault.switcher_title': 'Active Vault: click to switch or bootstrap',
      'vault.dropdown_title': 'Switch Active Vault',
      'vault.recent_label': 'Recent Vaults:',
      'vault.no_recents': 'No recent vaults recorded',
      'vault.quick_placeholder': '/path/to/vault',
      'vault.btn_switch': 'Switch',
      'vault.btn_bootstrap': '[Init LLM-Wiki]',
      'status.docs': 'Docs:',
      'status.docs_title': 'Total Indexed Documents',
      'status.vectors': 'Vectors:',
      'status.vectors_title': 'Total Embedded Vectors',
      'status.cache': 'Cache:',
      'status.cache_title': 'SQLite Cache Footprint',
      'status.logs': 'Logs:',
      'status.logs_title': 'Offloaded Log Nodes',
      'status.synced': 'Synced:',
      'status.synced_title': 'Last Incremental Sync Timestamp',
      'sync.button': 'Sync Vault',
      'sync.title': 'Trigger incremental scanner and vector reindex',
      'tabs.search': 'Search Debugger',
      'tabs.timeline': 'Timeline Stream',
      'tabs.graph': 'Graph Explorer',
      'tabs.logs': 'ADR Guard & Logs',
      'tabs.settings': 'Control Center',
      'search.title': 'Hybrid Search Debugger & Explainability',
      'search.desc': 'Interactive query playground with BM25 rank, vector cosine distance, RRF fusion, and WikiLinks Graph Boost.',
      'search.latency_title': 'Search query execution latency',
      'search.placeholder': 'Type query to test retrieval ranking (e.g. memory architecture, vector index)...',
      'search.shortcut_hint': '[ / or ⌘K ]',
      'search.mode_label': 'Mode:',
      'search.mode_hybrid': 'Hybrid (RRF)',
      'search.mode_hybrid_title': 'Reciprocal Rank Fusion with WikiLinks Graph Boost',
      'search.mode_bm25': 'FTS5 BM25',
      'search.mode_bm25_title': 'Pure BM25 full-text keyword retrieval',
      'search.mode_vector': 'Vector Cosine',
      'search.mode_vector_title': 'Dense vector semantic cosine similarity',
      'search.limit_label': 'Limit:',
      'search.run_button': 'Run',
      'search.run_title': 'Execute Query immediately',
      'search.initial_text': 'Enter a query above to view explainable search ranking waterfall.',
      'search.initial_subtext': 'Press / or ⌘K to focus search • Try "memory", "storage", or "architecture"',
      'search.no_hits': 'No matching notes found for query.',
      'search.inspect_btn': '[Inspect Note]',
      'timeline.title': 'Timeline Stream & Session Continuity',
      'timeline.desc': 'Chronological vertical neighborhood of passive session observations, engineering logs, and ADR milestones.',
      'timeline.anchor_placeholder': 'Search anchor note path, date (YYYY-MM-DD), or keyword...',
      'timeline.filter_all': 'All',
      'timeline.filter_observations': 'Observations',
      'timeline.filter_logs': 'Logs',
      'timeline.filter_decisions': 'Decisions',
      'timeline.refresh_title': 'Reload timeline stream',
      'timeline.refresh_btn': 'Refresh',
      'timeline.loading': 'Loading timeline stream...',
      'timeline.empty': 'No timeline events recorded.',
      'graph.title': 'WikiLinks Knowledge Graph Explorer',
      'graph.desc': '2D force-directed knowledge graph with semantic hierarchy color-coding and orphan discovery.',
      'graph.search_placeholder': 'Search / focus node...',
      'graph.tier_project': 'Project',
      'graph.tier_project_title': 'Toggle Project notes',
      'graph.tier_l3': 'L3 Evergreen',
      'graph.tier_l3_title': 'Toggle L3 Evergreen notes',
      'graph.tier_l2': 'L2 Log',
      'graph.tier_l2_title': 'Toggle L2 Log notes',
      'graph.tier_l1': 'L1 Resource',
      'graph.tier_l1_title': 'Toggle L1 Resource notes',
      'graph.tier_l0': 'L0 Daily',
      'graph.tier_l0_title': 'Toggle L0 Daily notes',
      'graph.highlight_orphans': 'Highlight Orphans',
      'graph.highlight_orphans_title': 'Highlight nodes with 0 links and 0 backlinks',
      'graph.zoom_in_title': 'Zoom In (+)',
      'graph.zoom_out_title': 'Zoom Out (-)',
      'graph.zoom_fit': 'Fit',
      'graph.zoom_fit_title': 'Fit to View',
      'graph.drawer_title_default': 'Select a Note',
      'graph.drawer_close_title': 'Close Inspector (Esc)',
      'graph.drawer_tags': 'Tags',
      'graph.drawer_no_tags': 'No tags',
      'graph.drawer_outlinks': 'Out-Links',
      'graph.drawer_no_outlinks': 'No outgoing WikiLinks',
      'graph.drawer_backlinks': 'Inbound Backlinks',
      'graph.drawer_no_backlinks': 'No inbound backlinks',
      'graph.btn_inspect_full': '[Inspect Note Drawer]',
      'logs.title': 'Living ADR Guard Sentinel & Symbolic Log Offload',
      'logs.desc': 'Active ADR invariant rules, interactive unified diff sandbox, and visual Mermaid pipeline traces.',
      'guard.sentinel_title': 'Living ADR Guard Sentinel & Diff Sandbox',
      'guard.speed_badge': '<10ms FAST CHECK',
      'guard.active_rules_title': 'Active Living ADR Invariants',
      'guard.btn_reload': 'Reload',
      'guard.loading_rules': 'Loading active ADR rules...',
      'guard.no_rules': 'No active ADR rules found.',
      'guard.diff_sandbox_title': 'Unified Git Diff Evaluator',
      'guard.btn_load_sample': 'Load Sample Diff',
      'guard.diff_placeholder': 'Paste unified git diff here (e.g. diff --git a/src/core.rs b/src/core.rs)...',
      'guard.btn_run_check': 'Run Guard Check (<10ms)',
      'guard.initial_hint': 'Paste a git diff above and click "Run Guard Check" to evaluate invariants.',
      'guard.pass_title': '[PASS // 0 VIOLATIONS]',
      'guard.pass_desc': 'All active Living ADR invariants satisfied. Diff is safe to commit.',
      'guard.violations_title': '[DENY // {count} VIOLATION(S)]',
      'logs.search_placeholder': 'Filter by node ID or task ID...',
      'logs.empty_title': 'No offloaded logs found in .k0maru/refs/',
      'logs.empty_subtext': 'Run long commands or k0maru offload to capture logs',
      'logs.select_node': 'Select a log node',
      'logs.copy_node_id': 'Copy Node ID',
      'logs.copy_node_id_title': 'Copy Node ID to clipboard',
      'logs.copy_raw_log': 'Copy Raw Log',
      'logs.copy_raw_log_title': 'Copy Raw Log slice to clipboard',
      'logs.diagram_title': 'State Machine Pipeline Diagram',
      'logs.diagram_placeholder': 'Select a log node to render pipeline state diagram.',
      'logs.raw_slice_title': 'Raw Log Slice',
      'logs.raw_slice_legend': 'Error & Warning Highlighted',
      'logs.raw_slice_placeholder': 'Raw log output will appear here.',
      'settings.title': 'Control Center & Agent Configuration',
      'settings.desc': 'Live hot-swappable --vault management, LLM-Wiki bootstrap, engine config, and AI client matrix.',
      'settings.vault_section_title': 'Section A: Active Vault & LLM-Wiki Bootstrap',
      'settings.vault_path_label': 'Active Vault Path:',
      'settings.vault_path_placeholder': '/path/to/vault',
      'settings.btn_apply_vault': 'Apply & Hot-Swap',
      'settings.vault_help': 'Hot-swaps memory engine, SQLite cache, and vector store live without restarting cockpit.',
      'settings.recent_vaults_label': 'Recent Vaults (Click to Switch):',
      'settings.bootstrap_title': 'Bootstrap Karpathy LLM-Wiki',
      'settings.bootstrap_desc': 'Creates semantic folders (projects/, decisions/, logs/, INDEX.md) at the target path and hot-swaps to it.',
      'settings.btn_bootstrap': '[Bootstrap New LLM-Wiki Here]',
      'settings.engine_section_title': 'Section B: Engine & Retrieval Configuration',
      'settings.embedding_model_label': 'Embedding Model:',
      'settings.search_mode_label': 'Default Search Mode:',
      'settings.token_budget_label': 'Loadout Token Budget:',
      'settings.token_budget_help': 'Strict limit for k0maru loadout & SessionStart MCP hook payload to preserve agent context.',
      'settings.guard_strict_title': 'Guard Strict Invariants',
      'settings.guard_strict_desc': 'Enforce hard DENY exits on ADR rule violations during pre-commit checks.',
      'settings.passive_capture_title': 'Passive Lifecycle Capture',
      'settings.passive_capture_desc': 'Log passive session observations on SessionStart, PostToolFailure, and SessionEnd.',
      'settings.btn_save_config': 'Save Engine Configuration',
      'settings.client_matrix_title': 'Section C: AI Agent Client & Hook Matrix',
      'settings.client_matrix_desc': 'Zero-daemon FastMCP integration and passive lifecycle hook status across 8 coding assistants.',
      'settings.btn_refresh_clients': 'Refresh Status',
      'settings.btn_configure_all': '[Configure All Clients]',
      'settings.loading_clients': 'Scanning AI client configurations...',
      'drawer.close_title': 'Close Drawer (Esc)',
      'drawer.lifecycle_chain': 'ADR Lifecycle Chain',
      'drawer.supersedes': 'Supersedes:',
      'drawer.superseded_by': 'Superseded By:',
      'drawer.frontmatter_title': 'Frontmatter Metadata',
      'drawer.no_frontmatter': 'No frontmatter recorded',
      'drawer.wikilinks_title': 'WikiLinks Neighborhood',
      'drawer.incoming_links': 'Incoming WikiLinks (Backlinks):',
      'drawer.no_inbound': 'No incoming backlinks',
      'drawer.outgoing_links': 'Outbound WikiLinks:',
      'drawer.no_outbound': 'No outgoing WikiLinks',
      'drawer.document_body_title': 'Document Markdown Body',
      'drawer.loading_body': 'Loading note content...',
      'toast.vault_switched': 'Vault switched to: {path}',
      'toast.vault_bootstrapped': 'LLM-Wiki bootstrapped at: {path}',
      'toast.config_saved': 'Configuration saved to ~/.config/k0maru/config.toml',
      'toast.synced': 'Vault synchronized successfully',
      'toast.client_installed': 'Configured MCP & hooks for {client}',
      'toast.clients_all_installed': 'Configured MCP & hooks for all detected AI clients',
      'toast.error': 'Error: {message}',
      'toast.copied': 'Copied to clipboard!',
    },
    zh: {
      'brand.subtitle': 'LLM-Wiki 零后台常驻记忆中枢',
      'vault.switcher_title': '活跃知识库：点击切换或初始化',
      'vault.dropdown_title': '切换活跃知识库',
      'vault.recent_label': '最近知识库：',
      'vault.no_recents': '暂无历史记录',
      'vault.quick_placeholder': '/path/to/vault',
      'vault.btn_switch': '切换',
      'vault.btn_bootstrap': '[初始化 LLM-Wiki]',
      'status.docs': '文档:',
      'status.docs_title': '已索引文档总数',
      'status.vectors': '向量:',
      'status.vectors_title': '已嵌入向量总数',
      'status.cache': '缓存:',
      'status.cache_title': 'SQLite 缓存占用空间',
      'status.logs': '日志:',
      'status.logs_title': '已卸载终端日志节点数',
      'status.synced': '同步:',
      'status.synced_title': '最近增量同步时间戳',
      'sync.button': '同步知识库',
      'sync.title': '触发增量扫描与向量重新索引',
      'tabs.search': '多路检索调试',
      'tabs.timeline': '时间线流',
      'tabs.graph': '知识图谱图鉴',
      'tabs.logs': 'ADR 防护与日志',
      'tabs.settings': '控制中心',
      'search.title': '多路混合检索调试与可解释性瀑布流',
      'search.desc': '交互式检索试验场：支持 BM25 排名、向量余弦相似度、RRF 倒数排名融合与 WikiLinks 图谱加权。',
      'search.latency_title': '检索查询执行延迟',
      'search.placeholder': '输入检索查询词测试排序（如：memory architecture, 向量索引）...',
      'search.shortcut_hint': '[ / 或 ⌘K ]',
      'search.mode_label': '模式:',
      'search.mode_hybrid': '混合检索 (RRF)',
      'search.mode_hybrid_title': '倒数排名融合 (RRF) 搭配 WikiLinks 图谱加权',
      'search.mode_bm25': 'FTS5 BM25',
      'search.mode_bm25_title': '纯 BM25 全文关键词词法检索',
      'search.mode_vector': '语义向量',
      'search.mode_vector_title': '稠密向量语义余弦相似度检索',
      'search.limit_label': '条数:',
      'search.run_button': '运行',
      'search.run_title': '立即执行检索',
      'search.initial_text': '在上方输入检索词以查看可解释的多路排序瀑布流。',
      'search.initial_subtext': '按 / 或 ⌘K 聚焦检索框 • 尝试输入 "memory"、"storage" 或 "architecture"',
      'search.no_hits': '未找到匹配的笔记。',
      'search.inspect_btn': '[透视笔记]',
      'timeline.title': '时间线流与会话连续性',
      'timeline.desc': '被动观察记录、工程日志与 ADR 决策里程碑的纵向时间线拓扑。',
      'timeline.anchor_placeholder': '检索锚点路径、日期 (YYYY-MM-DD) 或关键词...',
      'timeline.filter_all': '全部',
      'timeline.filter_observations': '会话观察',
      'timeline.filter_logs': '工程日志',
      'timeline.filter_decisions': 'ADR 决策',
      'timeline.refresh_title': '刷新时间线流',
      'timeline.refresh_btn': '刷新',
      'timeline.loading': '正在加载时间线流...',
      'timeline.empty': '暂无时间线事件记录。',
      'graph.title': 'WikiLinks 知识图谱图鉴',
      'graph.desc': '2D 力导向知识图谱：具备语义层级色彩编码与孤立笔记发现能力。',
      'graph.search_placeholder': '检索 / 定位笔记节点...',
      'graph.tier_project': '项目',
      'graph.tier_project_title': '显示/隐藏项目笔记',
      'graph.tier_l3': 'L3 永恒笔记',
      'graph.tier_l3_title': '显示/隐藏 L3 永恒笔记',
      'graph.tier_l2': 'L2 决策日志',
      'graph.tier_l2_title': '显示/隐藏 L2 决策日志',
      'graph.tier_l1': 'L1 资源引用',
      'graph.tier_l1_title': '显示/隐藏 L1 资源引用',
      'graph.tier_l0': 'L0 日记',
      'graph.tier_l0_title': '显示/隐藏 L0 日记',
      'graph.highlight_orphans': '高亮孤立笔记',
      'graph.highlight_orphans_title': '高亮出链与入链均为 0 的孤立笔记',
      'graph.zoom_in_title': '放大 (+)',
      'graph.zoom_out_title': '缩小 (-)',
      'graph.zoom_fit': '居中',
      'graph.zoom_fit_title': '适应画布',
      'graph.drawer_title_default': '选择笔记节点',
      'graph.drawer_close_title': '关闭抽屉 (Esc)',
      'graph.drawer_tags': '标签',
      'graph.drawer_no_tags': '无标签',
      'graph.drawer_outlinks': '出链引用',
      'graph.drawer_no_outlinks': '无出链引用',
      'graph.drawer_backlinks': '反向链接 Backlinks',
      'graph.drawer_no_backlinks': '无反向链接',
      'graph.btn_inspect_full': '[打开笔记透视抽屉]',
      'logs.title': 'Living ADR 守卫与符号化日志卸载',
      'logs.desc': '活跃 ADR 不变量规则、交互式统一 diff 沙盒与可视化 Mermaid 管线轨迹。',
      'guard.sentinel_title': 'Living ADR 守卫哨兵与 Diff 沙盒',
      'guard.speed_badge': '<10ms 快速判决',
      'guard.active_rules_title': '活跃 Living ADR 不变量',
      'guard.btn_reload': '重新加载',
      'guard.loading_rules': '正在加载活跃 ADR 规则...',
      'guard.no_rules': '未检索到活跃 ADR 规则。',
      'guard.diff_sandbox_title': 'Unified Git Diff 快速沙盒',
      'guard.btn_load_sample': '载入示例 Diff',
      'guard.diff_placeholder': '在此粘贴 unified git diff（例如 diff --git a/src/core.rs b/src/core.rs）...',
      'guard.btn_run_check': '执行守卫判决 (<10ms)',
      'guard.initial_hint': '在上方粘贴 git diff 并点击“执行守卫判决”以校验架构不变量。',
      'guard.pass_title': '[PASS // 0 项违规]',
      'guard.pass_desc': '所有活跃 Living ADR 不变量均通过检验，代码安全可提交。',
      'guard.violations_title': '[DENY // 拦截 {count} 项违规]',
      'logs.search_placeholder': '按 Node ID 或 Task ID 过滤...',
      'logs.empty_title': '.k0maru/refs/ 中未找到已卸载日志',
      'logs.empty_subtext': '运行耗时命令或执行 k0maru offload 捕获日志',
      'logs.select_node': '选择日志节点',
      'logs.copy_node_id': '复制 Node ID',
      'logs.copy_node_id_title': '复制 Node ID 到剪贴板',
      'logs.copy_raw_log': '复制原始日志',
      'logs.copy_raw_log_title': '复制原始日志切片到剪贴板',
      'logs.diagram_title': '状态机管线图',
      'logs.diagram_placeholder': '选择日志节点以渲染管线状态图。',
      'logs.raw_slice_title': '原始日志切片',
      'logs.raw_slice_legend': '已高亮错误与警告',
      'logs.raw_slice_placeholder': '原始日志输出将显示在此处。',
      'settings.title': '控制中心与 Agent 矩阵配置',
      'settings.desc': '支持 live 热切换 --vault 知识库、LLM-Wiki 标准拓扑初始化、引擎配置与 8 大 AI 客户端矩阵。',
      'settings.vault_section_title': 'A 模块: 活跃知识库与 LLM-Wiki 初始化',
      'settings.vault_path_label': '当前活跃知识库路径:',
      'settings.vault_path_placeholder': '/path/to/vault',
      'settings.btn_apply_vault': '应用并热切换',
      'settings.vault_help': '在线热切换记忆引擎、SQLite 缓存与向量库，无需重启控制台服务。',
      'settings.recent_vaults_label': '历史知识库列表 (点击即刻切换):',
      'settings.bootstrap_title': '初始化 Karpathy LLM-Wiki 标准拓扑',
      'settings.bootstrap_desc': '在目标路径下创建语义目录 (projects/, decisions/, logs/, INDEX.md) 并自动热切换进入。',
      'settings.btn_bootstrap': '[在此初始化 LLM-Wiki]',
      'settings.engine_section_title': 'B 模块: 引擎与检索高级配置',
      'settings.embedding_model_label': '向量嵌入模型:',
      'settings.search_mode_label': '默认检索模式:',
      'settings.token_budget_label': 'Loadout Token 预算:',
      'settings.token_budget_help': '硬性约束 k0maru loadout 与 SessionStart 钩子注入上下文上限，保护推理预算。',
      'settings.guard_strict_title': '严格 ADR 不变量守卫',
      'settings.guard_strict_desc': '在 Git pre-commit 期间遭遇 ADR 违规时强制中断并返回硬性 DENY 错误。',
      'settings.passive_capture_title': '生命周期被动捕获',
      'settings.passive_capture_desc': '在 SessionStart、PostToolFailure 与 SessionEnd 时自动记录被动会话观察。',
      'settings.btn_save_config': '保存引擎配置',
      'settings.client_matrix_title': 'C 模块: AI Agent 客户端与 Hook 矩阵',
      'settings.client_matrix_desc': '面向 8 款主流编程助手的零守护进程 FastMCP 与生命周期 Hook 安装矩阵。',
      'settings.btn_refresh_clients': '刷新客户端状态',
      'settings.btn_configure_all': '[一键配置全部客户端]',
      'settings.loading_clients': '正在扫描 AI 客户端配置...',
      'drawer.close_title': '关闭抽屉 (Esc)',
      'drawer.lifecycle_chain': 'ADR 生命周期演进链',
      'drawer.supersedes': '废弃替代 (Supersedes):',
      'drawer.superseded_by': '已被取代 (Superseded By):',
      'drawer.frontmatter_title': 'Frontmatter 元数据',
      'drawer.no_frontmatter': '未记录元数据',
      'drawer.wikilinks_title': 'WikiLinks 图谱邻域',
      'drawer.incoming_links': '入链反向链接 (Incoming Backlinks):',
      'drawer.no_inbound': '无入向反向链接',
      'drawer.outgoing_links': '出链引用 (Outbound WikiLinks):',
      'drawer.no_outbound': '无出链引用',
      'drawer.document_body_title': '文档 Markdown 正文',
      'drawer.loading_body': '正在加载笔记内容...',
      'toast.vault_switched': '知识库已热切换至: {path}',
      'toast.vault_bootstrapped': 'LLM-Wiki 标准拓扑已在 {path} 初始化完成',
      'toast.config_saved': '配置已持久化至 ~/.config/k0maru/config.toml',
      'toast.synced': '知识库增量同步成功',
      'toast.client_installed': '已为 {client} 配置 MCP 与生命周期 Hook',
      'toast.clients_all_installed': '已为所有检测到的 AI 客户端完成配置',
      'toast.error': '错误: {message}',
      'toast.copied': '已成功复制到剪贴板！',
    }
  };

  function t(key, params = {}) {
    const lang = currentLang;
    let text = (TRANSLATIONS[lang] && TRANSLATIONS[lang][key]) ||
               (TRANSLATIONS['en'] && TRANSLATIONS['en'][key]) ||
               key;
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), v);
    }
    return text;
  }

  function setLanguage(lang) {
    currentLang = lang === 'zh' ? 'zh' : 'en';
    try {
      localStorage.setItem('k0maru_lang', currentLang);
    } catch (_e) {}

    document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
    const langIndicator = document.getElementById('lang-indicator');
    if (langIndicator) {
      langIndicator.textContent = currentLang === 'zh' ? '中文' : 'EN / 中文';
    }

    // Update DOM translated nodes
    document.querySelectorAll('[data-i18n]').forEach((el) => {
      const key = el.getAttribute('data-i18n');
      if (key) {
        el.textContent = t(key);
      }
    });

    document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
      const key = el.getAttribute('data-i18n-placeholder');
      if (key) {
        el.placeholder = t(key);
      }
    });

    document.querySelectorAll('[data-i18n-title]').forEach((el) => {
      const key = el.getAttribute('data-i18n-title');
      if (key) {
        el.title = t(key);
      }
    });
  }

  // --------------------------------------------------------------------------
  // Hardened API Fetch Client (with X-K0maru-Client Header)
  // --------------------------------------------------------------------------
  async function apiRequest(url, options = {}) {
    const headers = options.headers ? { ...options.headers } : {};

    if (options.method && options.method.toUpperCase() === 'POST') {
      headers['Content-Type'] = 'application/json';
      headers['X-K0maru-Client'] = 'cockpit';
    }

    const fetchOpts = {
      ...options,
      headers,
    };

    const resp = await fetch(url, fetchOpts);
    if (!resp.ok) {
      let errMsg = `HTTP ${resp.status} ${resp.statusText}`;
      try {
        const errJson = await resp.json();
        if (errJson && errJson.error) {
          errMsg = errJson.error;
        }
      } catch (_e) {}
      throw new Error(errMsg);
    }

    const contentType = resp.headers.get('content-type') || '';
    if (contentType.includes('application/json')) {
      return await resp.json();
    }
    return await resp.text();
  }

  function showToast(message, type = 'info') {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `toast toast-${type} mono`;
    toast.textContent = message;

    container.appendChild(toast);
    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(8px)';
      toast.style.transition = 'all 200ms ease';
      setTimeout(() => toast.remove(), 220);
    }, 3800);
  }

  function copyTextToClipboard(text) {
    if (!navigator.clipboard) {
      const ta = document.createElement('textarea');
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      ta.remove();
      showToast(t('toast.copied'), 'success');
      return;
    }
    navigator.clipboard.writeText(text).then(() => {
      showToast(t('toast.copied'), 'success');
    }).catch((err) => {
      showToast(`Copy failed: ${err.message}`, 'error');
    });
  }

  function escapeHtml(str) {
    if (str == null) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 KB';
    const k = 1024;
    const dm = 1;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  }

  function formatRelativeTime(ts) {
    if (!ts) return '--';
    const now = Math.floor(Date.now() / 1000);
    const diff = now - ts;
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  // --------------------------------------------------------------------------
  // Top Header: Telemetry & Live Vault Switcher
  // --------------------------------------------------------------------------
  async function fetchStatus() {
    try {
      const status = await apiRequest('/api/status');
      appState.status = status;
      appState.vaultPath = status.vault_path || '';
      renderStatus(status);
    } catch (err) {
      console.warn('Failed to fetch status:', err.message);
    }
  }

  async function fetchConfig() {
    try {
      const config = await apiRequest('/api/config');
      appState.config = config;
      const activeVault = config.active_vault_path || config.vault_path;
      if (activeVault) {
        appState.vaultPath = activeVault;
      }
      if (Array.isArray(config.recent_vaults)) {
        appState.recentVaults = config.recent_vaults;
      }
      renderConfigInSettings(config);
      renderRecentVaults(appState.recentVaults);
    } catch (_err) {
      // Endpoint may not be ready in earlier worktrees
    }
  }

  function renderStatus(status) {
    if (!status) return;

    // Vault Path Pill
    const vaultPathEl = document.getElementById('status-vault-path');
    if (vaultPathEl) {
      vaultPathEl.textContent = status.vault_path || '--';
      vaultPathEl.title = status.vault_path || '';
    }

    // Docs
    const docCount = status.total_documents != null ? status.total_documents : '--';
    const sDoc = document.getElementById('status-doc-count');
    const sbDoc = document.getElementById('scoreboard-doc-count');
    if (sDoc) sDoc.textContent = docCount;
    if (sbDoc) sbDoc.textContent = docCount;

    // Vectors
    const vecCount = status.total_vectors != null ? status.total_vectors : '--';
    const sVec = document.getElementById('status-vector-count');
    const sbVec = document.getElementById('scoreboard-vec-count');
    if (sVec) sVec.textContent = vecCount;
    if (sbVec) sbVec.textContent = vecCount;

    // Cache Size
    const cacheFormatted = status.cache_size_bytes != null ? formatBytes(status.cache_size_bytes) : '-- KB';
    const sCache = document.getElementById('status-cache-size');
    const sbCache = document.getElementById('scoreboard-cache-size');
    if (sCache) sCache.textContent = cacheFormatted;
    if (sbCache) sbCache.textContent = cacheFormatted;

    // Logs
    const logCount = status.total_logs != null ? status.total_logs : '--';
    const sLogs = document.getElementById('status-log-count');
    const sbLogs = document.getElementById('scoreboard-log-count');
    if (sLogs) sLogs.textContent = logCount;
    if (sbLogs) sbLogs.textContent = logCount;

    // Synced
    const sSynced = document.getElementById('status-last-sync');
    if (sSynced) {
      sSynced.textContent = formatRelativeTime(status.last_sync_time);
    }
  }

  function renderRecentVaults(recentVaults) {
    const listTop = document.getElementById('top-recent-vaults-list');
    const listSettings = document.getElementById('recent-vaults-list');

    const renderItems = (container) => {
      if (!container) return;
      container.innerHTML = '';
      if (!recentVaults || recentVaults.length === 0) {
        container.innerHTML = `<span class="text-muted text-xs mono">${t('vault.no_recents')}</span>`;
        return;
      }
      recentVaults.forEach((vPath) => {
        const item = document.createElement('div');
        item.className = 'recent-vault-pill-item mono';
        item.innerHTML = `<span class="truncate">${escapeHtml(vPath)}</span><span class="badge badge-tech">[SELECT]</span>`;
        item.addEventListener('click', () => {
          switchVault(vPath);
        });
        container.appendChild(item);
      });
    };

    renderItems(listTop);
    renderItems(listSettings);
  }

  async function switchVault(newVaultPath) {
    if (!newVaultPath || !newVaultPath.trim()) return;
    try {
      showToast(`Hot-swapping vault to ${newVaultPath}...`, 'info');
      await apiRequest('/api/config', {
        method: 'POST',
        body: JSON.stringify({ vault_path: newVaultPath.trim() }),
      });
      showToast(t('toast.vault_switched', { path: newVaultPath }), 'success');
      const dropdown = document.getElementById('vault-switcher-dropdown');
      if (dropdown) dropdown.style.display = 'none';
      await fetchStatus();
      await fetchConfig();
      if (appState.activeTab === 'graph') {
        loadGraph();
      } else if (appState.activeTab === 'logs') {
        loadLogs();
        loadGuardRules();
      } else if (appState.activeTab === 'timeline') {
        loadTimeline();
      }
    } catch (err) {
      showToast(t('toast.error', { message: err.message }), 'error');
    }
  }

  async function bootstrapVault(targetPath) {
    const path = targetPath || appState.vaultPath;
    if (!path) {
      showToast('Please specify a target vault path to bootstrap', 'warning');
      return;
    }
    try {
      showToast(`Bootstrapping LLM-Wiki at ${path}...`, 'info');
      await apiRequest('/api/vault/bootstrap', {
        method: 'POST',
        body: JSON.stringify({ path }),
      });
      showToast(t('toast.vault_bootstrapped', { path }), 'success');
      const dropdown = document.getElementById('vault-switcher-dropdown');
      if (dropdown) dropdown.style.display = 'none';
      await fetchStatus();
      await fetchConfig();
    } catch (err) {
      showToast(t('toast.error', { message: err.message }), 'error');
    }
  }

  async function syncVault() {
    const syncBtn = document.getElementById('btn-sync-vault');
    const icon = syncBtn ? syncBtn.querySelector('.sync-icon') : null;
    if (icon) icon.classList.add('spinning');
    if (syncBtn) syncBtn.disabled = true;

    try {
      showToast('Executing incremental sync & vector reindexing...', 'info');
      await apiRequest('/api/sync', { method: 'POST' });
      showToast(t('toast.synced'), 'success');
      await fetchStatus();
      if (appState.activeTab === 'graph') {
        loadGraph();
      }
    } catch (err) {
      showToast(t('toast.error', { message: err.message }), 'error');
    } finally {
      if (icon) icon.classList.remove('spinning');
      if (syncBtn) syncBtn.disabled = false;
    }
  }

  function initVaultSwitcher() {
    const activePill = document.getElementById('active-vault-pill');
    const dropdown = document.getElementById('vault-switcher-dropdown');
    const quickInput = document.getElementById('quick-vault-input');
    const switchBtn = document.getElementById('btn-quick-switch');
    const bootstrapBtn = document.getElementById('btn-quick-bootstrap');

    if (activePill && dropdown) {
      activePill.addEventListener('click', (e) => {
        e.stopPropagation();
        const isOpen = dropdown.style.display !== 'none';
        dropdown.style.display = isOpen ? 'none' : 'flex';
        if (!isOpen && quickInput) {
          quickInput.value = appState.vaultPath;
          quickInput.focus();
        }
      });

      document.addEventListener('click', (e) => {
        if (!dropdown.contains(e.target) && !activePill.contains(e.target)) {
          dropdown.style.display = 'none';
        }
      });
    }

    if (switchBtn && quickInput) {
      switchBtn.addEventListener('click', () => {
        switchVault(quickInput.value);
      });
      quickInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          switchVault(quickInput.value);
        }
      });
    }

    if (bootstrapBtn && quickInput) {
      bootstrapBtn.addEventListener('click', () => {
        bootstrapVault(quickInput.value);
      });
    }

    const syncBtn = document.getElementById('btn-sync-vault');
    if (syncBtn) {
      syncBtn.addEventListener('click', syncVault);
    }

    const langToggleBtn = document.getElementById('btn-lang-toggle');
    if (langToggleBtn) {
      langToggleBtn.addEventListener('click', () => {
        setLanguage(currentLang === 'en' ? 'zh' : 'en');
      });
    }
  }

  // --------------------------------------------------------------------------
  // Navigation Tabs Engine
  // --------------------------------------------------------------------------
  function initTabs() {
    const tabButtons = document.querySelectorAll('.tab-btn');
    const panels = document.querySelectorAll('.tab-panel');

    tabButtons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const tabId = btn.getAttribute('data-tab');
        if (!tabId) return;

        tabButtons.forEach((b) => b.classList.remove('active'));
        panels.forEach((p) => p.classList.remove('active'));

        btn.classList.add('active');
        const targetPanel = document.getElementById(`panel-${tabId}`);
        if (targetPanel) {
          targetPanel.classList.add('active');
        }

        appState.activeTab = tabId;

        // Workspace tab activations
        if (tabId === 'timeline') {
          loadTimeline();
        } else if (tabId === 'graph') {
          loadGraph();
          resizeGraphCanvas();
        } else if (tabId === 'logs') {
          loadGuardRules();
          loadLogs();
        } else if (tabId === 'settings') {
          fetchConfig();
          loadClients();
        }
      });
    });
  }

  // --------------------------------------------------------------------------
  // TAB 1: Hybrid Search Debugger & Explainability
  // --------------------------------------------------------------------------
  let searchDebounce = null;

  function initSearchConsole() {
    const input = document.getElementById('search-input');
    const runBtn = document.getElementById('btn-run-search');
    const limitSlider = document.getElementById('search-limit');
    const limitVal = document.getElementById('search-limit-val');
    const modeToggles = document.querySelectorAll('.mode-toggles .btn-mode');

    if (limitSlider && limitVal) {
      limitSlider.addEventListener('input', () => {
        limitVal.textContent = limitSlider.value;
      });
    }

    modeToggles.forEach((btn) => {
      btn.addEventListener('click', () => {
        modeToggles.forEach((b) => b.classList.remove('active'));
        btn.classList.add('active');
        executeSearch();
      });
    });

    if (input) {
      input.addEventListener('input', () => {
        clearTimeout(searchDebounce);
        searchDebounce = setTimeout(() => {
          executeSearch();
        }, 250);
      });

      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          executeSearch();
        }
      });
    }

    if (runBtn) {
      runBtn.addEventListener('click', () => {
        executeSearch();
      });
    }
  }

  async function executeSearch() {
    const input = document.getElementById('search-input');
    const resultsContainer = document.getElementById('search-results');
    const latencyBadge = document.getElementById('search-latency');
    const limitSlider = document.getElementById('search-limit');
    const activeModeBtn = document.querySelector('.mode-toggles .btn-mode.active');

    if (!input || !resultsContainer) return;
    const query = input.value.trim();
    if (!query) {
      resultsContainer.innerHTML = `
        <div class="placeholder-state" id="search-initial-state">
          <svg class="icon icon-xl icon-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <path d="m21 21-4.3-4.3"></path>
          </svg>
          <p class="placeholder-text" data-i18n="search.initial_text">${t('search.initial_text')}</p>
          <span class="placeholder-subtext mono" data-i18n="search.initial_subtext">${t('search.initial_subtext')}</span>
        </div>`;
      if (latencyBadge) latencyBadge.textContent = '-- ms';
      return;
    }

    const mode = activeModeBtn ? activeModeBtn.getAttribute('data-mode') : 'hybrid';
    const limit = limitSlider ? limitSlider.value : 5;

    const startTime = performance.now();
    try {
      const url = `/api/search?q=${encodeURIComponent(query)}&mode=${encodeURIComponent(mode)}&limit=${limit}`;
      const data = await apiRequest(url);
      const elapsed = Math.round(performance.now() - startTime);
      if (latencyBadge) latencyBadge.textContent = `${elapsed} ms`;

      renderSearchResults(data, query, resultsContainer);
    } catch (err) {
      resultsContainer.innerHTML = `
        <div class="placeholder-state">
          <span class="badge badge-danger mono">SEARCH ERROR</span>
          <p class="placeholder-text">${escapeHtml(err.message)}</p>
        </div>`;
    }
  }

  function getTierFromPath(path) {
    const p = (path || '').toLowerCase();
    if (p.includes('decision') || p.includes('adr')) {
      return { label: '[L3 // ADR]', class: 'tier-l3' };
    }
    if (p.includes('project') || p.includes('10_project')) {
      return { label: '[L3 // PROJECT]', class: 'tier-project' };
    }
    if (p.includes('log') || p.includes('observation')) {
      return { label: '[L2 // LOG]', class: 'tier-l2' };
    }
    if (p.includes('daily') || p.includes('scratch') || p.includes('inbox')) {
      return { label: '[L0 // DAILY]', class: 'tier-l0' };
    }
    return { label: '[L1 // RESOURCE]', class: 'tier-l1' };
  }

  function renderSearchResults(data, query, container) {
    const results = Array.isArray(data) ? data : (data.results || data.hits || []);
    if (!results || results.length === 0) {
      container.innerHTML = `
        <div class="placeholder-state">
          <p class="placeholder-text">${t('search.no_hits')}</p>
          <span class="placeholder-subtext mono">Query: "${escapeHtml(query)}"</span>
        </div>`;
      return;
    }

    container.innerHTML = '';
    results.forEach((hit, idx) => {
      const path = hit.path || hit.relative_path || hit.id || 'unknown.md';
      const title = hit.title || path.split('/').pop().replace(/\.md$/, '');
      const score = hit.score != null ? hit.score.toFixed(4) : (1.0 / (idx + 1)).toFixed(4);
      const tier = getTierFromPath(path);
      const snippet = hit.snippet || hit.preview || hit.content || 'No preview snippet available.';

      // RRF Decomposition estimation
      const ftsScore = hit.fts_rank ? Math.min(100, Math.round(100 / hit.fts_rank)) : (hit.score ? Math.round(hit.score * 50) : 40);
      const vecScore = hit.vector_rank ? Math.min(100, Math.round(100 / hit.vector_rank)) : (hit.score ? Math.round(hit.score * 40) : 35);
      const graphScore = hit.graph_boost ? 25 : 10;
      const totalBar = ftsScore + vecScore + graphScore || 1;
      const ftsPct = Math.round((ftsScore / totalBar) * 100);
      const vecPct = Math.round((vecScore / totalBar) * 100);
      const graphPct = 100 - ftsPct - vecPct;

      const card = document.createElement('div');
      card.className = 'search-hit-card';
      card.innerHTML = `
        <div class="search-hit-header">
          <div class="search-hit-title-group">
            <span class="badge-tier ${tier.class} mono">${tier.label}</span>
            <span class="search-hit-title">${escapeHtml(title)}</span>
            <span class="search-hit-path mono truncate">${escapeHtml(path)}</span>
          </div>
          <div class="search-hit-badges">
            <span class="badge badge-tech mono">RRF: ${score}</span>
            <button type="button" class="btn btn-xs btn-outline mono btn-inspect-hit">${t('search.inspect_btn')}</button>
          </div>
        </div>
        <p class="search-hit-snippet">${escapeHtml(snippet)}</p>
        <div class="score-decomp-wrap mono">
          <span>DECOMP:</span>
          <div class="score-decomp-bar" title="FTS5: ${ftsPct}% | Vector: ${vecPct}% | Graph Boost: ${graphPct}%">
            <div class="decomp-fts" style="width: ${ftsPct}%;"></div>
            <div class="decomp-vec" style="width: ${vecPct}%;"></div>
            <div class="decomp-graph" style="width: ${graphPct}%;"></div>
          </div>
          <span>FTS: ${ftsPct}% &bull; VEC: ${vecPct}% &bull; GRAPH: ${graphPct}%</span>
        </div>
      `;

      card.addEventListener('click', () => {
        openNoteDrawer(path);
      });

      const inspectBtn = card.querySelector('.btn-inspect-hit');
      if (inspectBtn) {
        inspectBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          openNoteDrawer(path);
        });
      }

      container.appendChild(card);
    });
  }

  // --------------------------------------------------------------------------
  // TAB 2: Timeline Stream & Session Continuity
  // --------------------------------------------------------------------------
  async function loadTimeline() {
    const container = document.getElementById('timeline-stream-container');
    const countBadge = document.getElementById('timeline-count-badge');
    const anchorInput = document.getElementById('timeline-anchor-input');
    const anchor = anchorInput ? anchorInput.value.trim() : '';

    if (!container) return;
    container.innerHTML = `<div class="placeholder-state"><p class="placeholder-text">${t('timeline.loading')}</p></div>`;

    try {
      let url = `/api/timeline?window=25`;
      if (anchor) url += `&anchor=${encodeURIComponent(anchor)}`;
      const data = await apiRequest(url);
      const items = Array.isArray(data) ? data : (data.entries || data.items || data.timeline || []);
      appState.timelineItems = items;
      if (countBadge) countBadge.textContent = `${items.length} items`;
      renderTimelineItems(items);
    } catch (_err) {
      // Fallback: build timeline items from /api/logs
      try {
        const logs = await apiRequest('/api/logs');
        const fallbackItems = (Array.isArray(logs) ? logs : (logs.logs || [])).map((l) => ({
          path: l.path || `.k0maru/refs/${l.id}.log`,
          title: `Log ${l.id}`,
          timestamp: l.created_at || new Date().toISOString(),
          kind: 'log',
          summary: `Terminal stream offload with ${l.line_count || 0} lines`,
          hierarchy: 'L2Log',
        }));
        appState.timelineItems = fallbackItems;
        if (countBadge) countBadge.textContent = `${fallbackItems.length} items`;
        renderTimelineItems(fallbackItems);
      } catch (_e) {
        container.innerHTML = `<div class="placeholder-state"><p class="placeholder-text">${t('timeline.empty')}</p></div>`;
      }
    }
  }

  function renderTimelineItems(items) {
    const container = document.getElementById('timeline-stream-container');
    if (!container) return;

    const filtered = items.filter((item) => {
      if (appState.activeTimelineFilter === 'all') return true;
      const k = (item.kind || '').toLowerCase();
      const itemPath = (item.relative_path || item.path || '').toLowerCase();
      if (appState.activeTimelineFilter === 'observation') return k.includes('obs') || itemPath.includes('observation');
      if (appState.activeTimelineFilter === 'log') return k.includes('log') || itemPath.includes('log');
      if (appState.activeTimelineFilter === 'decision') return k.includes('decision') || k.includes('adr') || itemPath.includes('decision');
      return true;
    });

    if (filtered.length === 0) {
      container.innerHTML = `<div class="placeholder-state"><p class="placeholder-text">${t('timeline.empty')}</p></div>`;
      return;
    }

    container.innerHTML = '';
    filtered.forEach((item) => {
      const itemPath = item.relative_path || item.path || '';
      const tier = getTierFromPath(itemPath);
      const card = document.createElement('div');
      card.className = 'timeline-item-card';
      card.innerHTML = `
        <div class="timeline-node-pin"></div>
        <div class="timeline-item-header">
          <div class="timeline-item-title-group">
            <span class="badge-tier ${tier.class} mono">${tier.label}</span>
            <span class="timeline-item-title">${escapeHtml(item.title || itemPath)}</span>
          </div>
          <span class="timeline-item-timestamp mono">${escapeHtml(item.timestamp || item.date || '--')}</span>
        </div>
        <div class="timeline-item-summary">${escapeHtml(item.summary || item.preview || itemPath)}</div>
      `;

      card.addEventListener('click', () => {
        openNoteDrawer(itemPath);
      });

      container.appendChild(card);
    });
  }

  function initTimeline() {
    const anchorInput = document.getElementById('timeline-anchor-input');
    const refreshBtn = document.getElementById('btn-refresh-timeline');
    const filterBtns = document.querySelectorAll('.timeline-filter-pills .btn-filter-pill');

    if (anchorInput) {
      anchorInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          loadTimeline();
        }
      });
    }

    if (refreshBtn) {
      refreshBtn.addEventListener('click', loadTimeline);
    }

    filterBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        filterBtns.forEach((b) => b.classList.remove('active'));
        btn.classList.add('active');
        appState.activeTimelineFilter = btn.getAttribute('data-timeline-filter') || 'all';
        renderTimelineItems(appState.timelineItems);
      });
    });
  }

  // --------------------------------------------------------------------------
  // TAB 3: 2D Force-Directed Knowledge Graph Explorer
  // --------------------------------------------------------------------------
  const graphState = {
    canvas: null,
    ctx: null,
    nodes: [],
    links: [],
    nodeMap: new Map(),
    animFrame: null,
    zoom: 1.0,
    panX: 0,
    panY: 0,
    isDragging: false,
    dragNode: null,
    lastMouseX: 0,
    lastMouseY: 0,
    selectedNode: null,
    highlightOrphans: false,
    activeTiers: {
      'Project': true,
      'L3 Evergreen': true,
      'L2 Log': true,
      'L1 Resource': true,
      'L0 Daily': true,
    },
    searchQuery: '',
  };

  const TIER_COLORS = {
    'Project': '#E5E1BD',      // Warm Bone
    'L3 Evergreen': '#E5E1BD', // Warm Bone
    'L2 Log': '#AEBA78',       // Warm Sage
    'L1 Resource': '#7AA2B8',  // Slate Blue
    'L0 Daily': '#686D62',     // Muted Telemetry
  };

  async function loadGraph() {
    try {
      const data = await apiRequest('/api/graph');
      buildGraphData(data);
    } catch (err) {
      console.warn('Failed to load graph data:', err.message);
    }
  }

  function buildGraphData(data) {
    const rawNodes = data.nodes || [];
    const rawEdges = data.edges || [];
    const counter = document.getElementById('graph-counter');
    if (counter) {
      counter.textContent = `${rawNodes.length} nodes • ${rawEdges.length} links`;
    }

    const width = graphState.canvas ? graphState.canvas.clientWidth : 800;
    const height = graphState.canvas ? graphState.canvas.clientHeight : 600;

    graphState.nodeMap.clear();
    graphState.nodes = rawNodes.map((n, i) => {
      const angle = (i / (rawNodes.length || 1)) * Math.PI * 2;
      const radius = 120 + Math.random() * 160;
      const hierarchy = n.hierarchy || 'L3 Evergreen';
      const node = {
        id: n.id,
        title: n.title || n.id,
        hierarchy,
        tags: n.tags || [],
        tierKey: hierarchy.includes('Project') ? 'Project' :
                 hierarchy.includes('L3') || hierarchy.includes('decision') ? 'L3 Evergreen' :
                 hierarchy.includes('L2') || hierarchy.includes('log') ? 'L2 Log' :
                 hierarchy.includes('L0') || hierarchy.includes('daily') ? 'L0 Daily' : 'L1 Resource',
        x: width / 2 + Math.cos(angle) * radius,
        y: height / 2 + Math.sin(angle) * radius,
        vx: 0,
        vy: 0,
        radius: hierarchy.includes('L3') || hierarchy.includes('Project') ? 8 : 5,
        inLinks: [],
        outLinks: [],
      };
      graphState.nodeMap.set(n.id, node);
      return node;
    });

    graphState.links = rawEdges.map((e) => {
      const src = graphState.nodeMap.get(e.source);
      const tgt = graphState.nodeMap.get(e.target);
      if (src && tgt) {
        src.outLinks.push(tgt);
        tgt.inLinks.push(src);
      }
      return { source: src, target: tgt };
    }).filter((l) => l.source && l.target);

    resetGraphView();
    startGraphSimulation();
  }

  function resetGraphView() {
    if (!graphState.canvas) return;
    graphState.zoom = 1.0;
    graphState.panX = 0;
    graphState.panY = 0;
  }

  function resizeGraphCanvas() {
    const canvas = document.getElementById('graph-canvas');
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    graphState.canvas = canvas;
    graphState.ctx = canvas.getContext('2d');
    graphState.ctx.scale(dpr, dpr);
  }

  function startGraphSimulation() {
    if (graphState.animFrame) cancelAnimationFrame(graphState.animFrame);

    let alpha = 1.0;
    function step() {
      if (appState.activeTab === 'graph') {
        updateSimulation(alpha);
        renderGraphCanvas();
        if (alpha > 0.005) {
          alpha *= 0.985;
        }
      }
      graphState.animFrame = requestAnimationFrame(step);
    }
    graphState.animFrame = requestAnimationFrame(step);
  }

  function updateSimulation(alpha) {
    const nodes = graphState.nodes;
    const links = graphState.links;
    const width = graphState.canvas ? graphState.canvas.clientWidth : 800;
    const height = graphState.canvas ? graphState.canvas.clientHeight : 600;
    const cx = width / 2;
    const cy = height / 2;

    // Repulsion
    for (let i = 0; i < nodes.length; i++) {
      const a = nodes[i];
      for (let j = i + 1; j < nodes.length; j++) {
        const b = nodes[j];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const distSq = dx * dx + dy * dy || 1;
        if (distSq < 40000) {
          const force = (alpha * 120) / distSq;
          const fx = dx * force;
          const fy = dy * force;
          a.vx -= fx;
          a.vy -= fy;
          b.vx += fx;
          b.vy += fy;
        }
      }
      // Center gravity
      a.vx += (cx - a.x) * 0.001 * alpha;
      a.vy += (cy - a.y) * 0.001 * alpha;
    }

    // Link attraction
    for (let k = 0; k < links.length; k++) {
      const link = links[k];
      const dx = link.target.x - link.source.x;
      const dy = link.target.y - link.source.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const targetDist = 70;
      const force = (dist - targetDist) * 0.05 * alpha;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;
      link.source.vx += fx;
      link.source.vy += fy;
      link.target.vx -= fx;
      link.target.vy -= fy;
    }

    // Velocity update & damping
    for (let i = 0; i < nodes.length; i++) {
      const node = nodes[i];
      if (node !== graphState.dragNode) {
        node.vx *= 0.85;
        node.vy *= 0.85;
        node.x += node.vx;
        node.y += node.vy;
      }
    }
  }

  function renderGraphCanvas() {
    const canvas = graphState.canvas;
    const ctx = graphState.ctx;
    if (!canvas || !ctx) return;

    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    ctx.clearRect(0, 0, width, height);

    ctx.save();
    ctx.translate(graphState.panX + width / 2, graphState.panY + height / 2);
    ctx.scale(graphState.zoom, graphState.zoom);
    ctx.translate(-width / 2, -height / 2);

    // Render Edges
    for (let i = 0; i < graphState.links.length; i++) {
      const link = graphState.links[i];
      const srcVisible = graphState.activeTiers[link.source.tierKey];
      const tgtVisible = graphState.activeTiers[link.target.tierKey];
      if (!srcVisible || !tgtVisible) continue;

      ctx.beginPath();
      ctx.moveTo(link.source.x, link.source.y);
      ctx.lineTo(link.target.x, link.target.y);
      ctx.strokeStyle = '#23271F';
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    // Render Nodes
    for (let i = 0; i < graphState.nodes.length; i++) {
      const node = graphState.nodes[i];
      if (!graphState.activeTiers[node.tierKey]) continue;

      const isOrphan = node.inLinks.length === 0 && node.outLinks.length === 0;
      if (graphState.highlightOrphans && isOrphan) {
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius + 4, 0, Math.PI * 2);
        ctx.strokeStyle = '#C95B5B';
        ctx.lineWidth = 2;
        ctx.stroke();
      }

      ctx.beginPath();
      ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
      ctx.fillStyle = TIER_COLORS[node.tierKey] || '#AEBA78';
      ctx.fill();

      if (graphState.selectedNode === node) {
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius + 3, 0, Math.PI * 2);
        ctx.strokeStyle = '#E5E1BD';
        ctx.lineWidth = 2;
        ctx.stroke();
      }

      // Label
      ctx.font = '10px "JetBrains Mono", monospace';
      ctx.fillStyle = '#9EA397';
      ctx.fillText(node.title, node.x + node.radius + 4, node.y + 3);
    }

    ctx.restore();
  }

  function initGraphInteractions() {
    const canvas = document.getElementById('graph-canvas');
    if (!canvas) return;

    resizeGraphCanvas();
    window.addEventListener('resize', resizeGraphCanvas);

    canvas.addEventListener('mousedown', (e) => {
      const rect = canvas.getBoundingClientRect();
      const clientX = e.clientX - rect.left;
      const clientY = e.clientY - rect.top;

      // Transform coordinates to world
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      const worldX = (clientX - (graphState.panX + width / 2)) / graphState.zoom + width / 2;
      const worldY = (clientY - (graphState.panY + height / 2)) / graphState.zoom + height / 2;

      // Find hit node
      let hit = null;
      for (let i = graphState.nodes.length - 1; i >= 0; i--) {
        const node = graphState.nodes[i];
        if (!graphState.activeTiers[node.tierKey]) continue;
        const dx = worldX - node.x;
        const dy = worldY - node.y;
        if (dx * dx + dy * dy <= (node.radius + 4) * (node.radius + 4)) {
          hit = node;
          break;
        }
      }

      if (hit) {
        graphState.dragNode = hit;
        selectGraphNode(hit);
      } else {
        graphState.isDragging = true;
        graphState.lastMouseX = e.clientX;
        graphState.lastMouseY = e.clientY;
      }
    });

    window.addEventListener('mousemove', (e) => {
      if (graphState.dragNode) {
        const rect = canvas.getBoundingClientRect();
        const clientX = e.clientX - rect.left;
        const clientY = e.clientY - rect.top;
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        graphState.dragNode.x = (clientX - (graphState.panX + width / 2)) / graphState.zoom + width / 2;
        graphState.dragNode.y = (clientY - (graphState.panY + height / 2)) / graphState.zoom + height / 2;
      } else if (graphState.isDragging) {
        const dx = e.clientX - graphState.lastMouseX;
        const dy = e.clientY - graphState.lastMouseY;
        graphState.panX += dx;
        graphState.panY += dy;
        graphState.lastMouseX = e.clientX;
        graphState.lastMouseY = e.clientY;
      }
    });

    window.addEventListener('mouseup', () => {
      graphState.dragNode = null;
      graphState.isDragging = false;
    });

    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
      graphState.zoom = Math.max(0.2, Math.min(4.0, graphState.zoom * zoomFactor));
    });

    // Zoom Controls
    const zoomInBtn = document.getElementById('btn-zoom-in');
    const zoomOutBtn = document.getElementById('btn-zoom-out');
    const zoomFitBtn = document.getElementById('btn-zoom-fit');

    if (zoomInBtn) zoomInBtn.addEventListener('click', () => { graphState.zoom = Math.min(4.0, graphState.zoom * 1.2); });
    if (zoomOutBtn) zoomOutBtn.addEventListener('click', () => { graphState.zoom = Math.max(0.2, graphState.zoom * 0.8); });
    if (zoomFitBtn) zoomFitBtn.addEventListener('click', resetGraphView);

    // Orphan Toggle
    const orphanToggle = document.getElementById('toggle-orphans');
    if (orphanToggle) {
      orphanToggle.addEventListener('change', () => {
        graphState.highlightOrphans = orphanToggle.checked;
      });
    }

    // Tier filter pills
    document.querySelectorAll('.hierarchy-filters .btn-filter-pill').forEach((btn) => {
      btn.addEventListener('click', () => {
        const tier = btn.getAttribute('data-tier');
        btn.classList.toggle('active');
        graphState.activeTiers[tier] = btn.classList.contains('active');
      });
    });

    // Graph search
    const graphSearch = document.getElementById('graph-search');
    if (graphSearch) {
      graphSearch.addEventListener('input', () => {
        const q = graphSearch.value.trim().toLowerCase();
        if (!q) return;
        const found = graphState.nodes.find((n) => n.id.toLowerCase().includes(q) || n.title.toLowerCase().includes(q));
        if (found) {
          selectGraphNode(found);
          graphState.panX = -found.x + (graphState.canvas.clientWidth / 2);
          graphState.panY = -found.y + (graphState.canvas.clientHeight / 2);
        }
      });
    }

    // Side drawer close
    const drawerCloseBtn = document.getElementById('graph-drawer-close');
    if (drawerCloseBtn) {
      drawerCloseBtn.addEventListener('click', () => {
        const drawer = document.getElementById('graph-inspector');
        if (drawer) drawer.classList.remove('open');
      });
    }

    // Open Full Note Drawer from Graph Inspector
    const openFullBtn = document.getElementById('btn-open-drawer-from-graph');
    if (openFullBtn) {
      openFullBtn.addEventListener('click', () => {
        if (graphState.selectedNode) {
          openNoteDrawer(graphState.selectedNode.id);
        }
      });
    }
  }

  function selectGraphNode(node) {
    graphState.selectedNode = node;
    const drawer = document.getElementById('graph-inspector');
    if (!drawer) return;

    drawer.classList.add('open');
    const titleEl = document.getElementById('drawer-title');
    const pathEl = document.getElementById('drawer-path');
    const tierEl = document.getElementById('drawer-hierarchy');
    const tagsContainer = document.getElementById('drawer-tags');
    const outlinksContainer = document.getElementById('drawer-outlinks');
    const backlinksContainer = document.getElementById('drawer-backlinks');

    if (titleEl) titleEl.textContent = node.title;
    if (pathEl) pathEl.textContent = node.id;
    if (tierEl) {
      tierEl.textContent = `[${node.tierKey}]`;
      tierEl.className = `badge-tier mono`;
    }

    // Tags
    if (tagsContainer) {
      tagsContainer.innerHTML = '';
      if (!node.tags || node.tags.length === 0) {
        tagsContainer.innerHTML = `<span class="drawer-empty-hint mono">${t('graph.drawer_no_tags')}</span>`;
      } else {
        node.tags.forEach((tag) => {
          const pill = document.createElement('span');
          pill.className = 'badge badge-muted mono';
          pill.textContent = `#${tag}`;
          tagsContainer.appendChild(pill);
        });
      }
    }

    // Out-Links
    if (outlinksContainer) {
      outlinksContainer.innerHTML = '';
      if (!node.outLinks || node.outLinks.length === 0) {
        outlinksContainer.innerHTML = `<li class="drawer-empty-hint mono">${t('graph.drawer_no_outlinks')}</li>`;
      } else {
        node.outLinks.forEach((tgt) => {
          const li = document.createElement('li');
          li.className = 'drawer-link-item mono truncate';
          li.textContent = `[[${tgt.title}]]`;
          li.addEventListener('click', () => selectGraphNode(tgt));
          outlinksContainer.appendChild(li);
        });
      }
    }

    // Backlinks
    if (backlinksContainer) {
      backlinksContainer.innerHTML = '';
      if (!node.inLinks || node.inLinks.length === 0) {
        backlinksContainer.innerHTML = `<li class="drawer-empty-hint mono">${t('graph.drawer_no_backlinks')}</li>`;
      } else {
        node.inLinks.forEach((src) => {
          const li = document.createElement('li');
          li.className = 'drawer-link-item mono truncate';
          li.textContent = `[[${src.title}]]`;
          li.addEventListener('click', () => selectGraphNode(src));
          backlinksContainer.appendChild(li);
        });
      }
    }
  }

  // --------------------------------------------------------------------------
  // TAB 4: Living ADR Guard Sentinel & Symbolic Log Offload
  // --------------------------------------------------------------------------
  const SAMPLE_DIFF_DENY = `diff --git a/src/core.rs b/src/core.rs
--- a/src/core.rs
+++ b/src/core.rs
@@ -10,3 +10,4 @@ pub fn initialize_runtime() {
     println!("Initializing memory core...");
    let fallback_path = "/home/developer/secrets/api_token.key";
+    println!("Mounted path: {}", fallback_path);
 }`;

  async function loadGuardRules() {
    const list = document.getElementById('guard-rules-list');
    const badge = document.getElementById('guard-rules-count-badge');
    if (!list) return;

    list.innerHTML = `<div class="placeholder-state"><span class="mono text-muted text-xs">${t('guard.loading_rules')}</span></div>`;

    try {
      const data = await apiRequest('/api/guard/rules');
      const rules = Array.isArray(data) ? data : (data.rules || []);
      appState.guardRules = rules;
      if (badge) badge.textContent = `${rules.length} rules`;

      if (rules.length === 0) {
        list.innerHTML = `<div class="placeholder-state"><span class="mono text-muted text-xs">${t('guard.no_rules')}</span></div>`;
        return;
      }

      list.innerHTML = '';
      rules.forEach((rule) => {
        const item = document.createElement('div');
        item.className = 'guard-rule-item mono';
        const isDeny = (rule.severity || '').toUpperCase() === 'DENY';
        const globPattern = rule.file_pattern || rule.file_glob || '**/*';
        const cond = rule.condition || {};
        const condText =
          cond.Forbidden ||
          cond.MustContain ||
          (cond.MaxLines ? `MaxLines: ${cond.MaxLines}` : null) ||
          rule.forbidden_pattern ||
          rule.forbidden_regex ||
          rule.pattern ||
          '.*';
        const sourcePath = rule.source_note || rule.source_path || 'decisions/adr-001-repository-guardrails.md';
        item.innerHTML = `
          <div class="guard-rule-header">
            <span class="guard-rule-id">${escapeHtml(rule.id || 'RULE')}</span>
            <span class="badge ${isDeny ? 'badge-danger' : 'badge-warning'} mono">[${escapeHtml(rule.severity || 'WARN')}]</span>
          </div>
          <div class="guard-rule-meta">
            <span>GLOB: ${escapeHtml(globPattern)}</span>
            <span>FORBIDDEN: <code>${escapeHtml(String(condText))}</code></span>
            <span>SOURCE: <span class="rule-source-link">${escapeHtml(sourcePath)}</span></span>
          </div>
        `;

        const link = item.querySelector('.rule-source-link');
        if (link && sourcePath) {
          link.addEventListener('click', (e) => {
            e.stopPropagation();
            openNoteDrawer(sourcePath);
          });
        }

        list.appendChild(item);
      });
    } catch (_err) {
      list.innerHTML = `<div class="placeholder-state"><span class="mono text-muted text-xs">${t('guard.no_rules')}</span></div>`;
    }
  }

  async function runGuardCheck() {
    const diffInput = document.getElementById('guard-diff-input');
    const resultContainer = document.getElementById('guard-check-result');
    if (!diffInput || !resultContainer) return;

    const diff = diffInput.value.trim();
    if (!diff) {
      showToast('Please enter a unified git diff to evaluate', 'warning');
      return;
    }

    resultContainer.innerHTML = `<span class="mono text-muted text-xs">Evaluating diff against Living ADR rules (<10ms)...</span>`;

    try {
      const data = await apiRequest('/api/guard/check', {
        method: 'POST',
        body: JSON.stringify({ diff }),
      });

      const passed = data.is_clean != null ? data.is_clean : (data.passed || (data.violations && data.violations.length === 0));
      const violations = data.violations || [];

      if (passed) {
        resultContainer.innerHTML = `
          <div class="guard-pass-badge mono">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
              <polyline points="22 4 12 14.01 9 11.01"></polyline>
            </svg>
            <span>${t('guard.pass_title')}</span>
          </div>
          <p class="text-xs text-muted" style="margin-top: 4px;">${t('guard.pass_desc')}</p>
        `;
      } else {
        let violationsHtml = '';
        violations.forEach((v) => {
          violationsHtml += `
            <div class="guard-violation-item mono">
              <div style="display: flex; justify-content: space-between;">
                <strong>${escapeHtml(v.rule_id || 'VIOLATION')}</strong>
                <span class="badge badge-danger mono">[${escapeHtml(v.severity || 'DENY')}]</span>
              </div>
              <div>FILE: ${escapeHtml(v.file_path || v.file || 'unknown')}:${v.line_number || v.line || 0}</div>
              <div>MATCHED: <code>${escapeHtml(v.matched_line || v.matched_pattern || v.pattern || '')}</code></div>
              <div class="violation-remediation">REMEDY: ${escapeHtml(v.remediation || 'Remove forbidden pattern')}</div>
            </div>
          `;
        });

        resultContainer.innerHTML = `
          <div class="guard-violations-list">
            <div class="mono" style="color: var(--danger); font-weight: 700; margin-bottom: 4px;">
              ${t('guard.violations_title', { count: violations.length })}
            </div>
            ${violationsHtml}
          </div>
        `;
      }
    } catch (err) {
      resultContainer.innerHTML = `
        <div class="mono text-xs" style="color: var(--danger);">
          Guard check failed: ${escapeHtml(err.message)}
        </div>
      `;
    }
  }

  // Symbolic Log Explorer Engine
  async function loadLogs() {
    const list = document.getElementById('logs-list');
    const badge = document.getElementById('logs-count-badge');
    if (!list) return;

    try {
      const data = await apiRequest('/api/logs');
      const logs = Array.isArray(data) ? data : (data.logs || []);
      appState.logs = logs;
      if (badge) badge.textContent = `${logs.length} logs`;
      renderLogsList(logs);
      if (logs.length > 0 && !appState.selectedLogId) {
        selectLogNode(logs[0].id);
      }
    } catch (_err) {
      // Empty logs
    }
  }

  function renderLogsList(logs) {
    const list = document.getElementById('logs-list');
    if (!list) return;

    if (!logs || logs.length === 0) {
      list.innerHTML = `
        <div class="placeholder-state logs-empty-state" id="logs-empty-hint">
          <p class="placeholder-text">${t('logs.empty_title')}</p>
          <span class="placeholder-subtext mono">${t('logs.empty_subtext')}</span>
        </div>`;
      return;
    }

    list.innerHTML = '';
    logs.forEach((log) => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `log-item-btn mono ${appState.selectedLogId === log.id ? 'active' : ''}`;
      btn.innerHTML = `
        <div style="display: flex; justify-content: space-between;">
          <strong class="truncate">${escapeHtml(log.id)}</strong>
          <span class="badge badge-muted">${log.line_count || 0}L</span>
        </div>
        <div class="text-xs text-muted truncate">${escapeHtml(log.created_at || '--')}</div>
      `;

      btn.addEventListener('click', () => {
        selectLogNode(log.id);
      });

      list.appendChild(btn);
    });
  }

  async function selectLogNode(logId) {
    if (!logId) return;
    appState.selectedLogId = logId;

    document.querySelectorAll('.log-item-btn').forEach((btn) => {
      btn.classList.toggle('active', btn.textContent.includes(logId));
    });

    const idBadge = document.getElementById('log-detail-id');
    const linesBadge = document.getElementById('log-detail-lines');
    const copyIdBtn = document.getElementById('btn-copy-node-id');
    const copyRawBtn = document.getElementById('btn-copy-raw-log');
    const mermaidContainer = document.getElementById('log-mermaid-viewer');
    const rawContainer = document.getElementById('log-raw-viewer');

    if (idBadge) idBadge.textContent = logId;
    if (copyIdBtn) copyIdBtn.disabled = false;
    if (copyRawBtn) copyRawBtn.disabled = false;

    if (mermaidContainer) mermaidContainer.innerHTML = `<span class="mono text-muted text-xs">Loading state diagram...</span>`;
    if (rawContainer) rawContainer.innerHTML = `<span class="mono text-muted text-xs">Loading raw log slice...</span>`;

    try {
      const detail = await apiRequest(`/api/logs/${encodeURIComponent(logId)}`);
      if (linesBadge) {
        linesBadge.style.display = 'inline-flex';
        linesBadge.textContent = `${detail.content ? detail.content.split('\n').length : 0} lines`;
      }

      // Render Mermaid
      if (mermaidContainer) {
        renderMermaidSvg(detail.mermaid || 'stateDiagram-v2\n[*] --> Active\nActive --> Done', mermaidContainer);
      }

      // Render Raw Slice
      if (rawContainer) {
        renderRawLogContent(detail.content || '', rawContainer);
      }
    } catch (err) {
      if (mermaidContainer) mermaidContainer.innerHTML = `<span class="mono text-danger text-xs">Failed: ${escapeHtml(err.message)}</span>`;
      if (rawContainer) rawContainer.innerHTML = `<span class="mono text-danger text-xs">Failed: ${escapeHtml(err.message)}</span>`;
    }
  }

  function renderMermaidSvg(mermaidCode, container) {
    container.innerHTML = `
      <div style="font-family: var(--font-mono); font-size: 11px; color: var(--accent); white-space: pre-wrap; background: #0E100D; padding: 10px; border-radius: 4px;">
${escapeHtml(mermaidCode)}
      </div>
    `;
  }

  function renderRawLogContent(content, container) {
    if (!content) {
      container.innerHTML = `<span class="text-muted text-xs mono">Log content is empty.</span>`;
      return;
    }
    const lines = content.split('\n');
    const formatted = lines.map((line) => {
      const low = line.toLowerCase();
      let cls = '';
      if (low.includes('error') || low.includes('fatal') || low.includes('fail')) {
        cls = 'raw-line-err';
      } else if (low.includes('warn')) {
        cls = 'raw-line-warn';
      }
      return `<div class="${cls}">${escapeHtml(line)}</div>`;
    }).join('');
    container.innerHTML = formatted;
  }

  function initGuardAndLogs() {
    const reloadRulesBtn = document.getElementById('btn-reload-guard-rules');
    const loadSampleBtn = document.getElementById('btn-load-sample-diff');
    const runGuardBtn = document.getElementById('btn-run-guard-check');
    const copyIdBtn = document.getElementById('btn-copy-node-id');
    const copyRawBtn = document.getElementById('btn-copy-raw-log');
    const logSearch = document.getElementById('logs-search');

    if (reloadRulesBtn) reloadRulesBtn.addEventListener('click', loadGuardRules);
    if (loadSampleBtn) {
      loadSampleBtn.addEventListener('click', () => {
        const diffInput = document.getElementById('guard-diff-input');
        if (diffInput) diffInput.value = SAMPLE_DIFF_DENY;
      });
    }
    if (runGuardBtn) runGuardBtn.addEventListener('click', runGuardCheck);

    if (copyIdBtn) {
      copyIdBtn.addEventListener('click', () => {
        if (appState.selectedLogId) {
          copyTextToClipboard(appState.selectedLogId);
        }
      });
    }

    if (copyRawBtn) {
      copyRawBtn.addEventListener('click', () => {
        const rawEl = document.getElementById('log-raw-viewer');
        if (rawEl) {
          copyTextToClipboard(rawEl.innerText);
        }
      });
    }

    if (logSearch) {
      logSearch.addEventListener('input', () => {
        const q = logSearch.value.trim().toLowerCase();
        const filtered = appState.logs.filter((l) => l.id.toLowerCase().includes(q) || (l.task_id && l.task_id.toLowerCase().includes(q)));
        renderLogsList(filtered);
      });
    }
  }

  // --------------------------------------------------------------------------
  // TAB 5: Control Center & Agent Matrix
  // --------------------------------------------------------------------------
  function renderConfigInSettings(config) {
    if (!config) return;
    const vaultInput = document.getElementById('config-vault-path');
    const modelSelect = document.getElementById('config-embedding-model');
    const modeSelect = document.getElementById('config-search-mode');
    const budgetSlider = document.getElementById('config-token-budget');
    const budgetVal = document.getElementById('config-token-budget-val');
    const guardStrict = document.getElementById('config-guard-strict');
    const passiveCapture = document.getElementById('config-passive-capture');

    const activeVault = config.active_vault_path || config.vault_path;
    if (vaultInput && activeVault) vaultInput.value = activeVault;
    if (modelSelect && config.embedding_model) modelSelect.value = config.embedding_model;
    if (modeSelect) {
      const rawMode = config.default_search_mode || 'hybrid';
      modeSelect.value = rawMode === 'bm25' ? 'fts' : rawMode;
      if (!modeSelect.value) modeSelect.value = 'hybrid';
    }
    if (budgetSlider && config.loadout_token_budget != null) {
      budgetSlider.value = config.loadout_token_budget;
      if (budgetVal) budgetVal.textContent = config.loadout_token_budget;
    }
    if (guardStrict && config.guard_strict != null) guardStrict.checked = config.guard_strict;
    if (passiveCapture && config.passive_capture != null) passiveCapture.checked = config.passive_capture;
  }

  async function saveConfig() {
    const vaultInput = document.getElementById('config-vault-path');
    const modelSelect = document.getElementById('config-embedding-model');
    const modeSelect = document.getElementById('config-search-mode');
    const budgetSlider = document.getElementById('config-token-budget');
    const guardStrict = document.getElementById('config-guard-strict');
    const passiveCapture = document.getElementById('config-passive-capture');

    const payload = {
      vault_path: vaultInput ? vaultInput.value.trim() : appState.vaultPath,
      embedding_model: modelSelect ? modelSelect.value : 'bge-small',
      default_search_mode: modeSelect ? modeSelect.value : 'hybrid',
      loadout_token_budget: budgetSlider ? parseInt(budgetSlider.value, 10) : 300,
      guard_strict: guardStrict ? guardStrict.checked : true,
      passive_capture: passiveCapture ? passiveCapture.checked : true,
    };

    try {
      showToast('Persisting configuration...', 'info');
      await apiRequest('/api/config', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      showToast(t('toast.config_saved'), 'success');
      await fetchConfig();
      await fetchStatus();
    } catch (err) {
      showToast(t('toast.error', { message: err.message }), 'error');
    }
  }

  async function loadClients() {
    const container = document.getElementById('clients-matrix-container');
    if (!container) return;

    try {
      const data = await apiRequest('/api/clients');
      const clients = Array.isArray(data) ? data : (data.clients || []);
      appState.clients = clients;
      renderClients(clients, container);
    } catch (_err) {
      // Fallback: 8 default clients
      const defaultClients = [
        { id: 'claude', name: 'Claude Code', configured: true, details: 'FastMCP configured via ~/.claude.json' },
        { id: 'cursor', name: 'Cursor IDE', configured: true, details: 'MCP server active via .cursor/mcp.json' },
        { id: 'windsurf', name: 'Windsurf', configured: false, details: 'FastMCP hook not installed' },
        { id: 'gemini', name: 'Gemini CLI', configured: false, details: 'Pre-commit hook not installed' },
        { id: 'openclaw', name: 'OpenClaw', configured: false, details: 'MCP server pending installation' },
        { id: 'cline', name: 'Cline', configured: true, details: 'FastMCP server registered' },
        { id: 'hermes', name: 'Hermes Agent', configured: true, details: 'Skill export sync active' },
        { id: 'omp', name: 'omp Cockpit', configured: true, details: 'Integrated session & hooks active' },
      ];
      renderClients(defaultClients, container);
    }
  }

  function renderClients(clients, container) {
    container.innerHTML = '';
    clients.forEach((client) => {
      const card = document.createElement('div');
      card.className = 'client-card';
      const isConf = client.configured;
      const clientId = client.id || client.client || 'all';
      const clientName = client.name || client.client || 'AI Client';
      const clientDetails = client.details || (client.config_path ? `${client.config_path}${client.vault_path ? ' -> ' + client.vault_path : ''}` : '');

      card.innerHTML = `
        <div>
          <div class="client-header">
            <span class="client-name mono">${escapeHtml(clientName)}</span>
            <span class="badge ${isConf ? 'badge-success' : 'badge-muted'} mono">
              [${isConf ? 'CONFIGURED' : 'PENDING'}]
            </span>
          </div>
          <p class="client-details" style="margin-top: 6px;">${escapeHtml(clientDetails)}</p>
        </div>
        <button type="button" class="btn btn-xs ${isConf ? 'btn-secondary' : 'btn-primary'} mono btn-install-client">
          ${isConf ? 'Reinstall Hooks' : 'Configure MCP & Hooks'}
        </button>
      `;

      const installBtn = card.querySelector('.btn-install-client');
      if (installBtn) {
        installBtn.addEventListener('click', async () => {
          try {
            showToast(`Configuring MCP & hooks for ${clientName}...`, 'info');
            await apiRequest('/api/clients/install', {
              method: 'POST',
              body: JSON.stringify({ target: clientId, client_id: clientId }),
            });
            showToast(t('toast.client_installed', { client: clientName }), 'success');
            await loadClients();
          } catch (err) {
            showToast(t('toast.error', { message: err.message }), 'error');
          }
        });
      }

      container.appendChild(card);
    });
  }

  async function configureAllClients() {
    try {
      showToast('Configuring MCP & hooks across all AI clients...', 'info');
      await apiRequest('/api/clients/install', {
        method: 'POST',
        body: JSON.stringify({ target: 'all', all: true }),
      });
      showToast(t('toast.clients_all_installed'), 'success');
      await loadClients();
    } catch (err) {
      showToast(t('toast.error', { message: err.message }), 'error');
    }
  }

  function initControlCenter() {
    const applyVaultBtn = document.getElementById('btn-apply-vault');
    const vaultInput = document.getElementById('config-vault-path');
    const bootstrapBtn = document.getElementById('btn-bootstrap-vault');
    const saveConfigBtn = document.getElementById('btn-save-config');
    const budgetSlider = document.getElementById('config-token-budget');
    const budgetVal = document.getElementById('config-token-budget-val');
    const refreshClientsBtn = document.getElementById('btn-refresh-clients');
    const configureAllBtn = document.getElementById('btn-configure-all-clients');

    if (applyVaultBtn && vaultInput) {
      applyVaultBtn.addEventListener('click', () => {
        switchVault(vaultInput.value);
      });
    }

    if (bootstrapBtn && vaultInput) {
      bootstrapBtn.addEventListener('click', () => {
        bootstrapVault(vaultInput.value);
      });
    }

    if (budgetSlider && budgetVal) {
      budgetSlider.addEventListener('input', () => {
        budgetVal.textContent = budgetSlider.value;
      });
    }

    if (saveConfigBtn) {
      saveConfigBtn.addEventListener('click', saveConfig);
    }

    if (refreshClientsBtn) {
      refreshClientsBtn.addEventListener('click', loadClients);
    }

    if (configureAllBtn) {
      configureAllBtn.addEventListener('click', configureAllClients);
    }
  }

  // --------------------------------------------------------------------------
  // Slide-Over Note Inspector Drawer (#note-drawer)
  // --------------------------------------------------------------------------
  async function openNoteDrawer(notePath) {
    if (!notePath) return;
    const drawerOverlay = document.getElementById('note-drawer');
    if (!drawerOverlay) return;

    drawerOverlay.style.display = 'flex';
    drawerOverlay.setAttribute('aria-hidden', 'false');

    const titleEl = document.getElementById('drawer-note-title');
    const pathEl = document.getElementById('drawer-note-path');
    const tierEl = document.getElementById('drawer-note-tier');
    const statusEl = document.getElementById('drawer-note-status');
    const supersedesBox = document.getElementById('drawer-supersedes-container');
    const supersedesLinks = document.getElementById('drawer-note-supersedes');
    const supersededByLinks = document.getElementById('drawer-note-superseded-by');
    const rowSupersedes = document.getElementById('row-supersedes');
    const rowSupersededBy = document.getElementById('row-superseded-by');
    const fmContainer = document.getElementById('drawer-note-frontmatter');
    const inboundContainer = document.getElementById('drawer-note-inbound');
    const outboundContainer = document.getElementById('drawer-note-outbound');
    const bodyContainer = document.getElementById('drawer-note-body');

    const tier = getTierFromPath(notePath);
    if (tierEl) {
      tierEl.textContent = tier.label;
      tierEl.className = `badge-tier ${tier.class} mono`;
    }
    if (pathEl) pathEl.textContent = notePath;
    if (titleEl) titleEl.textContent = notePath.split('/').pop().replace(/\.md$/, '');
    if (bodyContainer) bodyContainer.innerHTML = `<span class="mono text-muted text-xs">${t('drawer.loading_body')}</span>`;

    try {
      const note = await apiRequest(`/api/note?path=${encodeURIComponent(notePath)}`);
      if (titleEl) titleEl.textContent = note.title || titleEl.textContent;
      if (statusEl) {
        const isSuperseded = (note.status || '').toUpperCase() === 'SUPERSEDED';
        statusEl.textContent = isSuperseded ? '[SUPERSEDED]' : '[ACTIVE]';
        statusEl.className = `badge ${isSuperseded ? 'badge-warning' : 'badge-tech'} mono`;
      }

      // Lifecycle chain
      const hasSupersedes = Array.isArray(note.supersedes) && note.supersedes.length > 0;
      const hasSupersededBy = note.superseded_by != null;
      if (supersedesBox) {
        supersedesBox.style.display = (hasSupersedes || hasSupersededBy) ? 'block' : 'none';
      }

      if (rowSupersedes && supersedesLinks) {
        rowSupersedes.style.display = hasSupersedes ? 'flex' : 'none';
        supersedesLinks.innerHTML = '';
        if (hasSupersedes) {
          note.supersedes.forEach((p) => {
            const pill = document.createElement('span');
            pill.className = 'chain-link-pill';
            pill.textContent = p;
            pill.addEventListener('click', () => openNoteDrawer(p));
            supersedesLinks.appendChild(pill);
          });
        }
      }

      if (rowSupersededBy && supersededByLinks) {
        rowSupersededBy.style.display = hasSupersededBy ? 'flex' : 'none';
        supersededByLinks.innerHTML = '';
        if (hasSupersededBy) {
          const pill = document.createElement('span');
          pill.className = 'chain-link-pill';
          pill.textContent = note.superseded_by;
          pill.addEventListener('click', () => openNoteDrawer(note.superseded_by));
          supersededByLinks.appendChild(pill);
        }
      }

      // Frontmatter
      if (fmContainer) {
        fmContainer.innerHTML = '';
        const fm = note.frontmatter || {};
        const entries = Object.entries(fm);
        if (entries.length === 0) {
          fmContainer.innerHTML = `<span class="text-muted text-xs mono">${t('drawer.no_frontmatter')}</span>`;
        } else {
          entries.forEach(([k, v]) => {
            const row = document.createElement('div');
            row.style.display = 'contents';
            row.innerHTML = `<span class="fm-key">${escapeHtml(k)}:</span><span class="fm-val">${escapeHtml(JSON.stringify(v))}</span>`;
            fmContainer.appendChild(row);
          });
        }
      }

      // Inbound Backlinks
      if (inboundContainer) {
        inboundContainer.innerHTML = '';
        const inLinks = note.inbound_links || note.incoming_links || note.backlinks || [];
        if (inLinks.length === 0) {
          inboundContainer.innerHTML = `<span class="text-muted text-xs mono">${t('drawer.no_inbound')}</span>`;
        } else {
          inLinks.forEach((linkPath) => {
            const pill = document.createElement('span');
            pill.className = 'wikilink-pill';
            pill.textContent = `[[${linkPath}]]`;
            pill.addEventListener('click', () => openNoteDrawer(linkPath));
            inboundContainer.appendChild(pill);
          });
        }
      }

      // Outbound WikiLinks
      if (outboundContainer) {
        outboundContainer.innerHTML = '';
        const outLinks = note.outbound_links || note.outgoing_links || [];
        if (outLinks.length === 0) {
          outboundContainer.innerHTML = `<span class="text-muted text-xs mono">${t('drawer.no_outbound')}</span>`;
        } else {
          outLinks.forEach((linkPath) => {
            const pill = document.createElement('span');
            pill.className = 'wikilink-pill';
            pill.textContent = `[[${linkPath}]]`;
            pill.addEventListener('click', () => openNoteDrawer(linkPath));
            outboundContainer.appendChild(pill);
          });
        }
      }

      // Document Markdown Body
      if (bodyContainer) {
        const bodyContent = note.content || note.body || note.markdown || '';
        renderFormattedMarkdown(bodyContent, bodyContainer);
      }
    } catch (_err) {
      if (bodyContainer) {
        bodyContainer.innerHTML = `
          <div class="placeholder-state">
            <span class="badge badge-muted mono">LOCAL NOTE</span>
            <p class="placeholder-text">${escapeHtml(notePath)}</p>
          </div>
        `;
      }
    }
  }

  function renderFormattedMarkdown(markdown, container) {
    if (!markdown) {
      container.innerHTML = `<span class="text-muted text-xs mono">Empty note content.</span>`;
      return;
    }

    // Basic markdown formatting without external libraries
    let html = escapeHtml(markdown);

    // Code blocks
    html = html.replace(/```([a-zA-Z0-9_]*)\n([\s\S]*?)```/g, (_m, _lang, code) => {
      return `<pre><code>${code}</code></pre>`;
    });

    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

    // Headings
    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    // Bold & italic
    html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');

    // WikiLinks [[Target]]
    html = html.replace(/\[\[([^\]]+)\]\]/g, (_m, target) => {
      return `<span class="wikilink-pill" data-target="${target}">[[${target}]]</span>`;
    });

    // Paragraphs
    html = html.replace(/\n\n/g, '</p><p>');
    container.innerHTML = `<p>${html}</p>`;

    // Wire WikiLinks inside markdown body
    container.querySelectorAll('.wikilink-pill').forEach((pill) => {
      const target = pill.getAttribute('data-target');
      if (target) {
        pill.addEventListener('click', () => openNoteDrawer(target));
      }
    });
  }

  function closeNoteDrawer() {
    const drawerOverlay = document.getElementById('note-drawer');
    if (!drawerOverlay) return;
    drawerOverlay.style.display = 'none';
    drawerOverlay.setAttribute('aria-hidden', 'true');
  }

  function initNoteDrawer() {
    const closeBtn = document.getElementById('btn-close-note-drawer');
    const backdrop = document.getElementById('note-drawer-backdrop');

    if (closeBtn) closeBtn.addEventListener('click', closeNoteDrawer);
    if (backdrop) backdrop.addEventListener('click', closeNoteDrawer);

    window.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        closeNoteDrawer();
      }
      // Cmd+K or Ctrl+K focus search
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        const searchTab = document.getElementById('tab-btn-search');
        if (searchTab) searchTab.click();
        const searchInput = document.getElementById('search-input');
        if (searchInput) searchInput.focus();
      }
      // Slash '/' to focus search if not in textarea/input
      if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
        e.preventDefault();
        const searchTab = document.getElementById('tab-btn-search');
        if (searchTab) searchTab.click();
        const searchInput = document.getElementById('search-input');
        if (searchInput) searchInput.focus();
      }
    });
  }

  // --------------------------------------------------------------------------
  // Application Bootstrap
  // --------------------------------------------------------------------------
  async function initApp() {
    // Detect preferred language
    try {
      const savedLang = localStorage.getItem('k0maru_lang');
      if (savedLang === 'zh' || savedLang === 'en') {
        currentLang = savedLang;
      } else if (navigator.language && navigator.language.startsWith('zh')) {
        currentLang = 'zh';
      }
    } catch (_e) {}

    setLanguage(currentLang);
    initVaultSwitcher();
    initTabs();
    initSearchConsole();
    initTimeline();
    initGraphInteractions();
    initGuardAndLogs();
    initControlCenter();
    initNoteDrawer();

    // Initial telemetry fetch
    await fetchStatus();
    await fetchConfig();

    // Periodic status polling (every 12 seconds)
    setInterval(fetchStatus, 12000);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
  } else {
    initApp();
  }
})();
