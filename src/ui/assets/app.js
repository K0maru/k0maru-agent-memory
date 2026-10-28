/**
 * K0maru Agent Memory Hub — Developer Dark Cockpit Frontend
 * Vanilla ES6 Application Logic & Bilingual I18n Engine
 */

let currentLang = 'en';

const TRANSLATIONS = {
  en: {
    'brand.subtitle': 'Obsidian & LLM-Wiki Zero-Daemon Memory Core',
    'status.vault': 'Vault:',
    'status.vault_title': 'Active Vault Path',
    'status.docs': 'Docs:',
    'status.docs_title': 'Total Indexed Documents',
    'status.vectors': 'Vectors:',
    'status.vectors_title': 'Total Embedded Vectors',
    'status.synced': 'Synced:',
    'status.synced_title': 'Last Incremental Sync Timestamp',
    'status.connecting': 'Connecting...',
    'status.disconnected': 'Disconnected',
    'status.never': 'Never',
    'status.unknown': 'Unknown',
    'sync.button': 'Sync Vault',
    'sync.syncing': 'Syncing...',
    'sync.synced': 'Synced',
    'sync.error': 'Error',
    'sync.ready': 'Ready',
    'sync.title': 'Trigger incremental scanner and vector reindex',
    'sync.toggle_lang_title': 'Switch Language (Current: English)',
    'tabs.search': 'Search Debugger',
    'tabs.graph': 'Graph Explorer',
    'tabs.logs': 'Log Inspector',
    'tabs.scoreboard': 'Token Scoreboard',
    'search.title': 'Hybrid Search Debugger & Explainability',
    'search.desc': 'Interactive query playground with BM25 rank, vector cosine distance, RRF fusion, and WikiLinks Graph Boost.',
    'search.latency_title': 'Search query execution latency',
    'search.placeholder': 'Type query to test retrieval ranking (e.g. memory architecture, vector index)...',
    'search.shortcut_hint': '[ / or ⌘K ]',
    'search.mode_label': 'Mode:',
    'search.mode_hybrid': 'Hybrid (RRF)',
    'search.mode_hybrid_title': 'Reciprocal Rank Fusion with WikiLinks Graph Boost',
    'search.mode_bm25': 'BM25 Lexical',
    'search.mode_bm25_title': 'Pure BM25 full-text keyword retrieval',
    'search.mode_vector': 'Vector Dense',
    'search.mode_vector_title': 'Dense vector semantic cosine similarity',
    'search.limit_label': 'Limit:',
    'search.run_button': 'Run',
    'search.run_title': 'Execute Query immediately',
    'search.initial_text': 'Enter a query above to view explainable search ranking waterfall.',
    'search.initial_subtext': 'Press / or ⌘K to focus search • Try "memory", "storage", or "architecture"',
    'search.no_hits_title': 'No matching notes found',
    'search.no_hits_desc': 'Zero notes matched "<strong>{query}</strong>" in <strong>{mode}</strong> mode.',
    'search.no_hits_sug1': 'Try broader keywords, partial stems, or check for typos.',
    'search.no_hits_sug2': 'Switch mode to Vector Dense to find semantically related notes.',
    'search.no_hits_sug3': 'Click Sync Vault in the top header to ensure recent files are indexed.',
    'search.score_label': 'Score:',
    'search.score_title': 'Fused RRF Score',
    'search.bm25_hit_title': 'BM25 rank in candidate pool',
    'search.bm25_miss_title': 'Not ranked in BM25 lexical pool',
    'search.vector_badge': 'Vector #{rank}',
    'search.vector_miss': 'Vector --',
    'search.vector_hit_title': 'Vector nearest neighbor rank',
    'search.vector_miss_title': 'Not ranked in Vector dense pool',
    'search.graph_boost': 'Graph Boost',
    'search.graph_boost_title': 'Elevated by 1-hop WikiLinks graph connectivity',
    'search.failed': 'Search execution failed: {error}',
    'search.failed_sub': 'Check backend status or adjust query parameters',
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
    'graph.counter': '{nodes} nodes · {links} links',
    'graph.drawer_title_default': 'Select a Note',
    'graph.drawer_close_title': 'Close Inspector (Esc)',
    'graph.drawer_tags': 'Tags',
    'graph.drawer_no_tags': 'No tags',
    'graph.drawer_outlinks': 'Out-Links',
    'graph.drawer_no_outlinks': 'No outgoing WikiLinks',
    'graph.drawer_backlinks': 'Inbound Backlinks',
    'graph.drawer_no_backlinks': 'No inbound backlinks',
    'logs.title': 'Log & Trace Inspector',
    'logs.desc': 'Visual Mermaid state machine diagram renderer and collapsible stack trace viewer for offloaded nodes.',
    'logs.badge': '{count} logs',
    'logs.search_placeholder': 'Filter by node ID or task ID...',
    'logs.empty_title': 'No offloaded logs found in .scratch/refs/',
    'logs.empty_subtext': 'Run long commands or k0maru offload to capture logs',
    'logs.no_match': 'No logs matching "{filter}"',
    'logs.no_match_sub': 'Try adjusting search query',
    'logs.lines': '{count} lines',
    'logs.select_node': 'Select a log node',
    'logs.copy_node_id': 'Copy Node ID',
    'logs.copy_node_id_title': 'Copy Node ID to clipboard',
    'logs.copy_raw_log': 'Copy Raw Log',
    'logs.copy_raw_log_title': 'Copy Raw Log slice to clipboard',
    'logs.copied': 'Copied!',
    'logs.diagram_title': 'State Machine Pipeline Diagram',
    'logs.diagram_placeholder': 'Select a log node to render pipeline state diagram.',
    'logs.raw_slice_title': 'Raw Log Slice',
    'logs.raw_slice_legend': 'Error & Warning Highlighted',
    'logs.raw_slice_placeholder': 'Raw log output will appear here.',
    'logs.log_empty': 'Log is empty.',
    'logs.loading_diagram': 'Loading trace diagram...',
    'logs.loading_raw': 'Loading raw log slice...',
    'logs.diagram_failed': 'Failed to load diagram: {error}',
    'logs.raw_failed': 'Failed to load log content: {error}',
    'scoreboard.title': 'Cache Health & Token Scoreboard',
    'scoreboard.desc': 'Real-time stats on documents, vector coverage, cache file size, and cumulative Token Reduction Ratio (TRR).',
    'scoreboard.status_healthy': 'Healthy',
    'scoreboard.kpi_docs_label': 'Vault Documents',
    'scoreboard.kpi_docs_sub': 'Parsed Markdown notes',
    'scoreboard.kpi_vecs_label': 'Vector Embeddings',
    'scoreboard.kpi_vecs_sub': 'Dense vector index coverage',
    'scoreboard.kpi_cache_label': 'Cache Database Size',
    'scoreboard.kpi_cache_sub': 'SQLite cache.sqlite footprint',
    'scoreboard.kpi_logs_label': 'Offloaded Log Nodes',
    'scoreboard.kpi_logs_sub': 'Symbolic terminal traces',
    'scoreboard.token_title': 'Token Economics & Context Hygiene',
    'scoreboard.benchmark_badge': 'Phase 3.8 Baseline',
    'scoreboard.token_hero_label': 'Estimated Cumulative Tokens Saved',
    'scoreboard.token_hero_unit': 'tokens',
    'scoreboard.token_hero_desc': 'Saved by replacing bulky terminal streams with symbolic pointer nodes.',
    'scoreboard.trr_title': 'Token Reduction Ratio (TRR)',
    'scoreboard.trr_target': 'Phase 3.8: 99.44% Target',
    'scoreboard.callout_title': 'Benchmark Verified:',
    'scoreboard.callout_desc': 'Exceeds Phase 3.8 TRR baseline of 99.44% context reduction on 10,000-line compiler traces.',
    'scoreboard.sync_card_title': 'Cache Operations & Index Sync',
    'scoreboard.sync_ready': 'Ready',
    'scoreboard.sync_card_desc': 'Trigger an incremental scan to index newly added or modified Markdown files, update WikiLinks connections, and compute vector embeddings without daemon overhead.',
    'scoreboard.active_vault_label': 'Active Vault Path:',
    'scoreboard.last_sync_label': 'Last Sync Completed:',
    'scoreboard.engine_mode_label': 'Engine Mode:',
    'scoreboard.engine_mode_val': 'Zero-Daemon Embedded (FastMCP)',
    'scoreboard.btn_incremental_sync': 'Run Incremental Sync',
    'toast.vault_synced': 'Vault synced: +{added} added, ~{modified} updated, {vectors} vectors embedded',
    'toast.sync_failed': 'Sync failed: {message}',
    'toast.copied': 'Copied to clipboard!',
    'toast.copy_failed': 'Failed to copy to clipboard',
    'toast.copy_not_supported': 'Clipboard copy not supported',
    'toast.graph_failed': 'Graph load failed: {message}',
    'toast.lang_switched': 'Switched to English',
  },
  zh: {
    'brand.subtitle': 'Obsidian 与大模型 Wiki 零后台常驻记忆中枢',
    'status.vault': '知识库:',
    'status.vault_title': '当前活跃知识库路径',
    'status.docs': '文档:',
    'status.docs_title': '已索引文档总数',
    'status.vectors': '向量:',
    'status.vectors_title': '已嵌入向量总数',
    'status.synced': '同步:',
    'status.synced_title': '最近增量同步时间戳',
    'status.connecting': '连接中...',
    'status.disconnected': '已断开',
    'status.never': '从未',
    'status.unknown': '未知',
    'sync.button': '同步知识库',
    'sync.syncing': '同步中...',
    'sync.synced': '已同步',
    'sync.error': '同步失败',
    'sync.ready': '就绪',
    'sync.title': '触发增量扫描与向量重新索引',
    'sync.toggle_lang_title': '切换语言 (当前: 简体中文)',
    'tabs.search': '多路检索调试',
    'tabs.graph': '知识图谱图鉴',
    'tabs.logs': '日志与状态机透视',
    'tabs.scoreboard': 'Token 节省计分板',
    'search.title': '多路混合检索调试与可解释性瀑布流',
    'search.desc': '交互式检索试验场：支持 BM25 排名、向量余弦相似度、RRF 倒数排名融合与 WikiLinks 图谱加权。',
    'search.latency_title': '检索查询执行延迟',
    'search.placeholder': '输入检索查询词测试排序（如：memory architecture, 向量索引）...',
    'search.shortcut_hint': '[ / 或 ⌘K ]',
    'search.mode_label': '检索模式:',
    'search.mode_hybrid': '混合检索 (RRF)',
    'search.mode_hybrid_title': '倒数排名融合 (RRF) 搭配 WikiLinks 图谱加权',
    'search.mode_bm25': 'BM25 词法检索',
    'search.mode_bm25_title': '纯 BM25 全文关键词检索',
    'search.mode_vector': '语义向量检索',
    'search.mode_vector_title': '稠密向量语义余弦相似度检索',
    'search.limit_label': '返回数量:',
    'search.run_button': '运行',
    'search.run_title': '立即执行检索',
    'search.initial_text': '在上方输入检索词以查看可解释的多路排序瀑布流。',
    'search.initial_subtext': '按 / 或 ⌘K 聚焦检索框 • 尝试输入 "memory"、"storage" 或 "architecture"',
    'search.no_hits_title': '未找到匹配的笔记',
    'search.no_hits_desc': '在 <strong>{mode}</strong> 模式下未找到与 "<strong>{query}</strong>" 匹配的笔记。',
    'search.no_hits_sug1': '尝试更宽泛的关键词、词干，或检查拼写是否有误。',
    'search.no_hits_sug2': '切换为“语义向量检索”模式以查找语义相关笔记。',
    'search.no_hits_sug3': '点击顶部“同步知识库”确保最新笔记已加入索引。',
    'search.score_label': '得分:',
    'search.score_title': 'RRF 融合综合得分',
    'search.bm25_hit_title': 'BM25 候选池排名',
    'search.bm25_miss_title': '未命中 BM25 词法候选池',
    'search.vector_badge': '向量排名 #{rank}',
    'search.vector_miss': '向量 --',
    'search.vector_hit_title': '向量近邻检索排名',
    'search.vector_miss_title': '未命中向量稠密候选池',
    'search.graph_boost': '图谱加权',
    'search.graph_boost_title': '由 1-hop WikiLinks 图谱连通性提升',
    'search.failed': '检索执行失败：{error}',
    'search.failed_sub': '请检查后端状态或调整查询参数',
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
    'graph.counter': '{nodes} 节点 · {links} 连线',
    'graph.drawer_title_default': '选择笔记节点',
    'graph.drawer_close_title': '关闭抽屉 (Esc)',
    'graph.drawer_tags': '标签',
    'graph.drawer_no_tags': '无标签',
    'graph.drawer_outlinks': '出链引用',
    'graph.drawer_no_outlinks': '无出链引用',
    'graph.drawer_backlinks': '反向链接 Backlinks',
    'graph.drawer_no_backlinks': '无反向链接',
    'logs.title': '日志与状态机透视',
    'logs.desc': '可视化 Mermaid 状态机图表渲染器与已卸载节点的折叠调用栈查看器。',
    'logs.badge': '{count} 条日志',
    'logs.search_placeholder': '按 Node ID 或 Task ID 过滤...',
    'logs.empty_title': '.scratch/refs/ 中未找到已卸载日志',
    'logs.empty_subtext': '运行耗时命令或执行 k0maru offload 捕获日志',
    'logs.no_match': '未找到匹配 "{filter}" 的日志',
    'logs.no_match_sub': '尝试调整过滤关键词',
    'logs.lines': '{count} 行',
    'logs.select_node': '选择日志节点',
    'logs.copy_node_id': '复制 Node ID',
    'logs.copy_node_id_title': '复制 Node ID 到剪贴板',
    'logs.copy_raw_log': '复制原始日志',
    'logs.copy_raw_log_title': '复制原始日志切片到剪贴板',
    'logs.copied': '已复制!',
    'logs.diagram_title': '状态机管线图',
    'logs.diagram_placeholder': '选择日志节点以渲染管线状态图。',
    'logs.raw_slice_title': '原始日志切片',
    'logs.raw_slice_legend': '已高亮错误与警告',
    'logs.raw_slice_placeholder': '原始日志输出将显示在此处。',
    'logs.log_empty': '日志内容为空。',
    'logs.loading_diagram': '正在加载轨迹状态图...',
    'logs.loading_raw': '正在加载原始日志切片...',
    'logs.diagram_failed': '加载状态图失败：{error}',
    'logs.raw_failed': '加载日志内容失败：{error}',
    'scoreboard.title': '缓存健康与 Token 节省计分板',
    'scoreboard.desc': '实时监控索引文档、向量覆盖率、缓存体积以及累计上下文压缩率 (TRR)。',
    'scoreboard.status_healthy': '运行良好',
    'scoreboard.kpi_docs_label': '索引 Markdown 文档',
    'scoreboard.kpi_docs_sub': '已解析 Markdown 笔记',
    'scoreboard.kpi_vecs_label': '向量索引与覆盖率',
    'scoreboard.kpi_vecs_sub': '稠密向量索引覆盖率',
    'scoreboard.kpi_cache_label': 'SQLite 缓存体积',
    'scoreboard.kpi_cache_sub': 'cache.sqlite 占用空间',
    'scoreboard.kpi_logs_label': '已卸载日志节点',
    'scoreboard.kpi_logs_sub': '符号化终端调用轨迹',
    'scoreboard.token_title': 'Token 经济学与上下文整洁度',
    'scoreboard.benchmark_badge': 'Phase 3.8 基线',
    'scoreboard.token_hero_label': '累计估算节省 Token',
    'scoreboard.token_hero_unit': 'tokens',
    'scoreboard.token_hero_desc': '通过使用符号指针节点替代冗长的终端输出流实现节省。',
    'scoreboard.trr_title': '上下文压缩率 (TRR)',
    'scoreboard.trr_target': 'Phase 3.8: 99.44% 达标线',
    'scoreboard.callout_title': '基线实测验证:',
    'scoreboard.callout_desc': '在 10,000 行编译日志追踪测试中，超越 Phase 3.8 TRR 99.44% 上下文压缩基线。',
    'scoreboard.sync_card_title': '缓存运维与索引同步',
    'scoreboard.sync_ready': '就绪',
    'scoreboard.sync_card_desc': '触发增量扫描，零守护进程常驻开销下索引新增或修改的 Markdown 文档、更新 WikiLinks 关联并生成向量嵌入。',
    'scoreboard.active_vault_label': '当前活跃知识库路径:',
    'scoreboard.last_sync_label': '最近完成同步:',
    'scoreboard.engine_mode_label': '运行引擎模式:',
    'scoreboard.engine_mode_val': '零守护进程嵌入式 (FastMCP)',
    'scoreboard.btn_incremental_sync': '执行增量同步',
    'toast.vault_synced': '知识库已同步：新增 +{added} 篇，更新 ~{modified} 篇，嵌入 {vectors} 个向量',
    'toast.sync_failed': '同步失败：{message}',
    'toast.copied': '已复制到剪贴板！',
    'toast.copy_failed': '复制到剪贴板失败',
    'toast.copy_not_supported': '当前环境不支持剪贴板复制',
    'toast.graph_failed': '加载图谱数据失败：{message}',
    'toast.lang_switched': '已切换至简体中文',
  },
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

function getLanguage() {
  try {
    const stored = localStorage.getItem('k0maru_lang');
    if (stored === 'zh' || stored === 'en') {
      return stored;
    }
  } catch (_e) {}
  const navLang = (navigator.language || navigator.userLanguage || '').toLowerCase();
  if (navLang.startsWith('zh')) {
    return 'zh';
  }
  return 'en';
}

function setLanguage(lang) {
  currentLang = (lang === 'zh') ? 'zh' : 'en';
  try {
    localStorage.setItem('k0maru_lang', currentLang);
  } catch (_e) {}
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';

  const indicator = document.getElementById('lang-indicator');
  if (indicator) {
    indicator.textContent = currentLang === 'zh' ? '中文' : 'English';
  }
  const btnToggle = document.getElementById('btn-lang-toggle');
  if (btnToggle) {
    btnToggle.title = currentLang === 'zh' ? '切换语言 (当前: 简体中文)' : 'Switch Language (Current: English)';
  }

  // Translate static DOM elements
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    if (key && TRANSLATIONS[currentLang] && TRANSLATIONS[currentLang][key]) {
      el.textContent = TRANSLATIONS[currentLang][key];
    }
  });

  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    const key = el.getAttribute('data-i18n-placeholder');
    if (key && TRANSLATIONS[currentLang] && TRANSLATIONS[currentLang][key]) {
      el.setAttribute('placeholder', TRANSLATIONS[currentLang][key]);
    }
  });

  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    const key = el.getAttribute('data-i18n-title');
    if (key && TRANSLATIONS[currentLang] && TRANSLATIONS[currentLang][key]) {
      el.setAttribute('title', TRANSLATIONS[currentLang][key]);
    }
  });

  // Re-render dynamic elements
  const syncBtn = document.getElementById('btn-sync');
  if (syncBtn) {
    const syncText = syncBtn.querySelector('.btn-text');
    if (syncText) {
      if (syncBtn.classList.contains('syncing')) {
        syncText.textContent = t('sync.syncing');
      } else {
        syncText.textContent = t('sync.button');
      }
    }
  }

  const scoreboardSyncStatus = document.getElementById('scoreboard-sync-status');
  if (scoreboardSyncStatus) {
    if (scoreboardSyncStatus.textContent === 'Syncing...' || scoreboardSyncStatus.textContent === '同步中...') {
      scoreboardSyncStatus.textContent = t('sync.syncing');
    } else {
      scoreboardSyncStatus.textContent = t('sync.ready');
    }
  }

  if (typeof updateGraphCounter === 'function' && graphState && graphState.nodes) {
    updateGraphCounter();
  }

  if (typeof openDrawer === 'function' && graphState && graphState.selectedNode) {
    openDrawer(graphState.selectedNode);
  }

  if (cachedLogs) {
    const logsBadge = document.getElementById('logs-count-badge');
    if (logsBadge) {
      logsBadge.textContent = t('logs.badge', { count: cachedLogs.length });
    }
    const searchInput = document.getElementById('logs-search');
    const filter = searchInput ? searchInput.value.trim().toLowerCase() : '';
    if (typeof renderLogsList === 'function') {
      renderLogsList(filter);
    }
  }

  const searchInput = document.getElementById('search-input');
  const resultsContainer = document.getElementById('search-results');
  if (resultsContainer) {
    if (!searchInput || !searchInput.value.trim()) {
      if (typeof renderEmptyInitialState === 'function') {
        renderEmptyInitialState(resultsContainer);
      }
    } else if (window.latestSearchResults && typeof renderSearchResults === 'function') {
      renderSearchResults(window.latestSearchResults, searchInput.value.trim(), resultsContainer);
    }
  }

  if (window.latestStatus && typeof renderStatus === 'function') {
    renderStatus(window.latestStatus);
  }
}

function initLanguageSwitcher() {
  const btnToggle = document.getElementById('btn-lang-toggle');
  if (btnToggle) {
    btnToggle.addEventListener('click', () => {
      const nextLang = currentLang === 'zh' ? 'en' : 'zh';
      setLanguage(nextLang);
      showToast(t('toast.lang_switched'), 'info');
    });
  }
}

function getTierDisplayName(tierKey) {
  switch (tierKey) {
    case 'Project': return t('graph.tier_project');
    case 'L3 Evergreen': return t('graph.tier_l3');
    case 'L2 Log': return t('graph.tier_l2');
    case 'L1 Resource': return t('graph.tier_l1');
    case 'L0 Daily': return t('graph.tier_l0');
    default: return tierKey;
  }
}

// Application State for Search Console
let currentSearchMode = 'hybrid';
let currentSearchLimit = 5;
let searchDebounceTimer = null;
let currentAbortController = null;

document.addEventListener('DOMContentLoaded', () => {
  initTabs();
  initLanguageSwitcher();
  initSync();
  initSearchConsole();
  initGraphExplorer();
  initLogInspector();
  initScoreboard();
  initKeyboardShortcuts();
  setLanguage(getLanguage());
  fetchStatus();
  loadLogsList();
});

/**
 * Tab Navigation and Hash Routing
 */
function initTabs() {
  const tabs = document.querySelectorAll('.tab-btn');
  const panels = document.querySelectorAll('.tab-panel');

  function switchTab(tabId, updateHash = true) {
    // If graph tab is requested, fallback cleanly to search (graph is temporarily hidden)
    let target = tabId;
    if (target === 'graph') {
      target = 'search';
    }

    const validTabs = ['search', 'logs', 'scoreboard'];
    const activeTab = validTabs.includes(target) ? target : 'search';

    tabs.forEach(tab => {
      if (tab.dataset.tab === activeTab) {
        tab.classList.add('active');
        tab.setAttribute('aria-selected', 'true');
      } else {
        tab.classList.remove('active');
        tab.setAttribute('aria-selected', 'false');
      }
    });

    panels.forEach(panel => {
      if (panel.id === `panel-${activeTab}`) {
        panel.classList.add('active');
      } else {
        panel.classList.remove('active');
      }
    });

    if (activeTab === 'graph' && typeof window.onGraphTabActivated === 'function') {
      window.onGraphTabActivated();
    } else if (activeTab === 'logs') {
      loadLogsList();
    } else if (activeTab === 'scoreboard') {
      fetchStatus();
    }

    if (updateHash || tabId === 'graph') {
      history.replaceState(null, null, `#${activeTab}`);
    }
  }

  // Bind click handlers to tab buttons
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      switchTab(tab.dataset.tab, true);
    });
  });

  // Handle URL hash changes
  window.addEventListener('hashchange', () => {
    const hash = window.location.hash.replace('#', '');
    if (hash) {
      switchTab(hash, false);
    }
  });

  // Initial tab from hash or default
  const initialHash = window.location.hash.replace('#', '');
  if (initialHash) {
    switchTab(initialHash, false);
  } else {
    switchTab('search', false);
  }
}

/**
 * Global Keyboard Shortcut Listeners
 * - Pressing `/` or `Cmd+K` / `Ctrl+K` focuses the search bar
 * - Pressing `Esc` blurs the search input
 */
function initKeyboardShortcuts() {
  window.addEventListener('keydown', (e) => {
    // Cmd+K or Ctrl+K
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      focusSearchInput();
      return;
    }

    // '/' key when not already focused inside an input or textarea
    if (e.key === '/' && !isTypingInInput(e.target)) {
      e.preventDefault();
      focusSearchInput();
      return;
    }

    // Escape key closes graph inspector drawer or blurs search input
    if (e.key === 'Escape') {
      const drawer = document.getElementById('graph-inspector') || document.querySelector('.graph-drawer');
      if (drawer && drawer.classList.contains('open')) {
        deselectNode();
        return;
      }
      const searchInput = document.getElementById('search-input');
      if (searchInput && document.activeElement === searchInput) {
        searchInput.blur();
      }
    }
  });
}

function focusSearchInput() {
  const searchTabBtn = document.getElementById('tab-btn-search');
  if (searchTabBtn && !searchTabBtn.classList.contains('active')) {
    searchTabBtn.click();
  }
  const input = document.getElementById('search-input');
  if (input) {
    input.focus();
    input.select();
  }
}

function isTypingInInput(target) {
  if (!target) return false;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || target.isContentEditable;
}

/**
 * Search Debugger Console & Scoring Waterfall (Ticket 21)
 */
function initSearchConsole() {
  const searchInput = document.getElementById('search-input');
  const runBtn = document.getElementById('btn-run-search');
  const limitSlider = document.getElementById('search-limit');
  const limitVal = document.getElementById('search-limit-val');
  const modeButtons = document.querySelectorAll('.btn-mode');

  // Mode toggles
  modeButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      modeButtons.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      currentSearchMode = btn.dataset.mode || 'hybrid';
      triggerSearch(false);
    });
  });

  // Limit slider
  if (limitSlider && limitVal) {
    limitSlider.addEventListener('input', () => {
      currentSearchLimit = parseInt(limitSlider.value, 10) || 5;
      limitVal.textContent = currentSearchLimit;
      triggerSearch(true);
    });
  }

  // Search input typing (debounced 250ms)
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      triggerSearch(true);
    });

    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        triggerSearch(false);
      }
    });
  }

  // Execute button click
  if (runBtn) {
    runBtn.addEventListener('click', () => {
      triggerSearch(false);
    });
  }
}

function triggerSearch(debounced = true) {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer);
    searchDebounceTimer = null;
  }

  if (debounced) {
    searchDebounceTimer = setTimeout(() => {
      executeSearch();
    }, 250);
  } else {
    executeSearch();
  }
}

async function executeSearch() {
  const searchInput = document.getElementById('search-input');
  const resultsContainer = document.getElementById('search-results');
  const latencyEl = document.getElementById('search-latency');
  if (!searchInput || !resultsContainer) return;

  const query = searchInput.value.trim();

  // If query is empty, render initial helpful state
  if (!query) {
    window.latestSearchResults = null;
    renderEmptyInitialState(resultsContainer);
    if (latencyEl) latencyEl.textContent = '-- ms';
    return;
  }

  // Abort any in-flight request
  if (currentAbortController) {
    currentAbortController.abort();
  }
  currentAbortController = new AbortController();

  const startTime = performance.now();
  resultsContainer.classList.add('loading');

  try {
    const url = `/api/search?q=${encodeURIComponent(query)}&mode=${encodeURIComponent(currentSearchMode)}&limit=${encodeURIComponent(currentSearchLimit)}`;
    const res = await fetch(url, { signal: currentAbortController.signal });

    const elapsed = Math.round(performance.now() - startTime);
    if (latencyEl) {
      latencyEl.textContent = `${elapsed}ms`;
    }

    if (!res.ok) {
      const errData = await res.json().catch(() => ({}));
      throw new Error(errData.error || `HTTP ${res.status}`);
    }

    const results = await res.json();
    window.latestSearchResults = results;
    renderSearchResults(results, query, resultsContainer);
  } catch (err) {
    if (err.name === 'AbortError') {
      return;
    }
    console.error('Search query failed:', err);
    window.latestSearchResults = null;
    renderSearchError(err.message, resultsContainer);
    if (latencyEl) latencyEl.textContent = 'Err';
  } finally {
    resultsContainer.classList.remove('loading');
  }
}

function renderEmptyInitialState(container) {
  container.innerHTML = `
    <div class="placeholder-state" id="search-initial-state">
      <svg class="icon icon-xl icon-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"></circle>
        <path d="m21 21-4.3-4.3"></path>
      </svg>
      <p class="placeholder-text" data-i18n="search.initial_text">${t('search.initial_text')}</p>
      <span class="placeholder-subtext mono" data-i18n="search.initial_subtext">${t('search.initial_subtext')}</span>
    </div>
  `;
}

function renderSearchError(errorMessage, container) {
  container.innerHTML = `
    <div class="placeholder-state">
      <svg class="icon icon-xl" style="color: var(--danger);" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
      <p class="placeholder-text" style="color: var(--danger);">${t('search.failed', { error: escapeHtml(errorMessage) })}</p>
      <span class="placeholder-subtext mono" data-i18n="search.failed_sub">${t('search.failed_sub')}</span>
    </div>
  `;
}

function renderSearchResults(results, query, container) {
  if (!results || results.length === 0) {
    container.innerHTML = `
      <div class="zero-hits-state">
        <svg class="icon icon-xl icon-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"></circle>
          <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
        <h3 class="zero-hits-title" data-i18n="search.no_hits_title">${t('search.no_hits_title')}</h3>
        <p class="zero-hits-desc">${t('search.no_hits_desc', { query: escapeHtml(query), mode: escapeHtml(currentSearchMode) })}</p>
        <ul class="zero-hits-suggestions">
          <li data-i18n="search.no_hits_sug1">${t('search.no_hits_sug1')}</li>
          <li data-i18n="search.no_hits_sug2">${t('search.no_hits_sug2')}</li>
          <li data-i18n="search.no_hits_sug3">${t('search.no_hits_sug3')}</li>
        </ul>
      </div>
    `;
    return;
  }

  const cardsHtml = results.map((item, idx) => {
    const tier = getHierarchyTier(item.path);
    const tierName = getTierDisplayName(tier.name);
    const scoreFormatted = Number(item.score).toFixed(4);
    const highlightedSnippet = highlightQueryTerms(item.snippet, query);

    // BM25 badge
    const bm25Badge = item.bm25_rank != null
      ? `<span class="pill-bm25 hit" title="${t('search.bm25_hit_title')}">BM25 #${item.bm25_rank}</span>`
      : `<span class="pill-bm25" title="${t('search.bm25_miss_title')}">BM25 --</span>`;

    // Vector badge
    const vectorBadge = item.vector_rank != null
      ? `<span class="pill-vector hit" title="${t('search.vector_hit_title')}">${t('search.vector_badge', { rank: item.vector_rank })}</span>`
      : `<span class="pill-vector" title="${t('search.vector_miss_title')}">${t('search.vector_miss')}</span>`;

    // Graph boost badge
    const graphBoostBadge = (item.graph_boost && item.graph_boost > 0)
      ? `<span class="badge badge-emerald graph-boost" title="${t('search.graph_boost_title')}">
          <svg class="graph-boost-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 3-1.9 5.8a2 2 0 0 1-1.3 1.3L3 12l5.8 1.9a2 2 0 0 1 1.3 1.3L12 21l1.9-5.8a2 2 0 0 1 1.3-1.3L21 12l-5.8-1.9a2 2 0 0 1-1.3-1.3Z"></path>
          </svg>
          +${Number(item.graph_boost).toFixed(2)} ${t('search.graph_boost')}
        </span>`
      : '';

    return `
      <article class="search-result-card" data-path="${escapeHtml(item.path)}">
        <div class="card-header">
          <div class="card-title-group">
            <div class="card-title-row">
              <span class="badge-tier ${tier.className}">${tierName}</span>
              <h3 class="card-title">${escapeHtml(item.title || item.path)}</h3>
            </div>
            <span class="card-path mono">
              <svg class="icon icon-muted" style="width: 13px; height: 13px;" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                <polyline points="14 2 14 8 20 8"></polyline>
              </svg>
              ${escapeHtml(item.path)}
            </span>
          </div>

          <div class="waterfall-pill-group">
            <span class="pill-score" title="${t('search.score_title')}">
              <span class="pill-label" data-i18n="search.score_label">${t('search.score_label')}</span>
              <span class="score-val">${scoreFormatted}</span>
            </span>
            ${bm25Badge}
            ${vectorBadge}
            ${graphBoostBadge}
          </div>
        </div>

        <pre class="card-snippet"><code>${highlightedSnippet}</code></pre>
      </article>
    `;
  }).join('');

  container.innerHTML = cardsHtml;
}

/**
 * Maps document relative path to semantic hierarchy tier
 */
function getHierarchyTier(path) {
  const p = (path || '').toLowerCase();
  if (p.startsWith('00_daily') || p.includes('/daily/') || p.includes('daily')) {
    return { name: 'L0 Daily', className: 'tier-l0' };
  }
  if (p.startsWith('05_resources') || p.includes('/resources/') || p.includes('resource')) {
    return { name: 'L1 Resource', className: 'tier-l1' };
  }
  if (p.startsWith('30_logs') || p.includes('/refs/') || p.includes('/logs/') || p.includes('node_')) {
    return { name: 'L2 Log', className: 'tier-l2' };
  }
  if (p.startsWith('10_projects') || p.includes('/projects/') || p.includes('project')) {
    return { name: 'Project', className: 'tier-project' };
  }
  return { name: 'L3 Evergreen', className: 'tier-l3' };
}

/**
 * Escapes HTML characters to prevent XSS vulnerabilities
 */
function escapeHtml(str) {
  if (str == null) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

/**
 * Escapes regex special characters
 */
function escapeRegex(str) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Highlights matching query terms in snippet text
 */
function highlightQueryTerms(text, query) {
  if (!text) return '';
  const escaped = escapeHtml(text);
  if (!query) return escaped;

  const terms = query
    .split(/\s+/)
    .map(t => t.trim())
    .filter(t => t.length > 0)
    .sort((a, b) => b.length - a.length);

  if (terms.length === 0) return escaped;

  const pattern = terms.map(t => escapeRegex(escapeHtml(t))).join('|');
  try {
    const regex = new RegExp(`(${pattern})`, 'gi');
    return escaped.replace(regex, '<mark class="search-highlight">$1</mark>');
  } catch (_e) {
    return escaped;
  }
}

/**
 * Vault Status Polling and Rendering
 */
async function fetchStatus() {
  try {
    const res = await fetch('/api/status');
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: Failed to fetch status`);
    }

    const data = await res.json();
    renderStatus(data);
  } catch (err) {
    console.error('Failed to load vault status:', err);
    const vaultEl = document.getElementById('status-vault-path');
    if (vaultEl) {
      vaultEl.textContent = t('status.disconnected');
      vaultEl.title = err.message;
    }
  }
}

function renderStatus(status) {
  // Vault Path
  const vaultPathEl = document.getElementById('status-vault-path');
  if (vaultPathEl) {
    vaultPathEl.textContent = status.vault_path || t('status.unknown');
    vaultPathEl.title = status.vault_path || '';
  }

  // Document Count
  const docCountEl = document.getElementById('status-doc-count');
  if (docCountEl) {
    docCountEl.textContent = Number(status.total_documents).toLocaleString();
  }

  // Vector Count
  const vecCountEl = document.getElementById('status-vector-count');
  if (vecCountEl) {
    vecCountEl.textContent = Number(status.total_vectors).toLocaleString();
  }

  // Last Sync
  const lastSyncEl = document.getElementById('status-last-sync');
  if (lastSyncEl) {
    if (status.last_sync_time) {
      const d = new Date(status.last_sync_time * 1000);
      lastSyncEl.textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
      lastSyncEl.title = d.toISOString();
    } else {
      lastSyncEl.textContent = t('status.never');
    }
  }

  // Update Scoreboard Panel metrics
  updateScoreboard(status);
}

/**
 * Vault Synchronization Trigger (Header & Scoreboard Sync Buttons)
 */
function initSync() {
  const syncBtn = document.getElementById('btn-sync');
  const scoreboardSyncBtn = document.getElementById('btn-scoreboard-sync');

  async function performSync(triggerBtn) {
    const allBtns = [syncBtn, scoreboardSyncBtn].filter(Boolean);
    allBtns.forEach(b => {
      b.classList.add('syncing');
      b.disabled = true;
      const syncText = b.querySelector('.btn-text');
      if (syncText) syncText.textContent = t('sync.syncing');
    });

    const statusBadge = document.getElementById('scoreboard-sync-status');
    if (statusBadge) statusBadge.textContent = t('sync.syncing');

    try {
      const res = await fetch('/api/sync', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });

      if (!res.ok) {
        const errorData = await res.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${res.status}`);
      }

      const syncData = await res.json();
      const added = syncData.sync_stats?.added ?? 0;
      const modified = syncData.sync_stats?.modified ?? 0;
      const vectors = syncData.vector_stats?.embedded_count ?? 0;

      showToast(t('toast.vault_synced', { added, modified, vectors }), 'success');
      await fetchStatus();
      await loadLogsList();
    } catch (err) {
      console.error('Vault sync error:', err);
      showToast(t('toast.sync_failed', { message: err.message }), 'error');
    } finally {
      allBtns.forEach(b => {
        b.classList.remove('syncing');
        b.disabled = false;
        const syncText = b.querySelector('.btn-text');
        if (syncText) syncText.textContent = t('sync.button');
      });
      if (statusBadge) statusBadge.textContent = t('sync.ready');
    }
  }

  if (syncBtn) {
    syncBtn.addEventListener('click', () => performSync(syncBtn));
  }
  if (scoreboardSyncBtn) {
    scoreboardSyncBtn.addEventListener('click', () => performSync(scoreboardSyncBtn));
  }
}

/**
 * Toast Notifications
 */
function showToast(message, type = 'info') {
  const container = document.getElementById('toast-container');
  if (!container) return;

  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  toast.textContent = message;

  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateY(8px)';
    toast.style.transition = 'all 200ms ease';
    setTimeout(() => {
      if (toast.parentNode) {
        toast.parentNode.removeChild(toast);
      }
    }, 200);
  }, 3500);
}

/**
 * ==========================================================================
 * WikiLinks Knowledge Graph Explorer & Hierarchy Visualizer (Ticket 22)
 * Interactive 2D Force-Directed Canvas Network
 * ==========================================================================
 */

const HIERARCHY_CONFIG = {
  'Project': {
    name: 'Project',
    color: '#22C55E',
    dimColor: 'rgba(34, 197, 94, 0.18)',
    badgeClass: 'tier-project',
    baseRadius: 13,
  },
  'L3 Evergreen': {
    name: 'L3 Evergreen',
    color: '#F59E0B',
    dimColor: 'rgba(245, 158, 11, 0.18)',
    badgeClass: 'tier-l3',
    baseRadius: 10,
  },
  'L2 Log': {
    name: 'L2 Log',
    color: '#3B82F6',
    dimColor: 'rgba(59, 130, 246, 0.18)',
    badgeClass: 'tier-l2',
    baseRadius: 8,
  },
  'L1 Resource': {
    name: 'L1 Resource',
    color: '#A855F7',
    dimColor: 'rgba(168, 85, 247, 0.18)',
    badgeClass: 'tier-l1',
    baseRadius: 8,
  },
  'L0 Daily': {
    name: 'L0 Daily',
    color: '#64748B',
    dimColor: 'rgba(100, 116, 139, 0.18)',
    badgeClass: 'tier-l0',
    baseRadius: 7,
  },
};

const graphState = {
  initialized: false,
  loading: false,
  rawNodes: [],
  rawEdges: [],
  nodes: [],
  links: [],
  nodeMap: new Map(),
  activeTiers: new Set(['Project', 'L3 Evergreen', 'L2 Log', 'L1 Resource', 'L0 Daily']),
  highlightOrphans: false,
  searchQuery: '',
  selectedNode: null,
  hoveredNode: null,
  transform: { x: 0, y: 0, scale: 1 },
  isDraggingCanvas: false,
  draggedNode: null,
  dragStartMouse: { x: 0, y: 0 },
  dragStartTransform: { x: 0, y: 0 },
  mouseDownPos: { x: 0, y: 0 },
  simulation: {
    alpha: 1.0,
    alphaMin: 0.001,
    alphaDecay: 0.015,
  },
  animFrameId: null,
};

function classifyNodeTier(node) {
  const h = (node.hierarchy || '').toLowerCase();
  const p = (node.id || '').toLowerCase();

  if (p.startsWith('00_daily') || p.includes('/daily/') || h.includes('l0') || h.includes('ephemeral')) {
    return 'L0 Daily';
  }
  if (p.startsWith('05_resources') || p.includes('/resources/') || h.includes('l1') || h.includes('resource')) {
    return 'L1 Resource';
  }
  if (p.startsWith('01_ai_logs') || p.startsWith('30_logs') || p.includes('/logs/') || p.includes('/refs/') || h.includes('l2') || h.includes('log')) {
    return 'L2 Log';
  }
  if (p.startsWith('10_projects') || p.includes('/projects/') || h.includes('project')) {
    return 'Project';
  }
  return 'L3 Evergreen';
}

function isGraphPanelVisible() {
  const panel = document.getElementById('panel-graph');
  if (!panel) return false;
  if (!panel.classList.contains('active')) return false;
  if (typeof window !== 'undefined' && window.getComputedStyle) {
    const style = window.getComputedStyle(panel);
    if (style.display === 'none' || style.visibility === 'hidden') {
      return false;
    }
  }
  return true;
}

function initGraphExplorer() {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas) return;

  // Window resize observer
  const container = document.getElementById('graph-viewport-container');
  if (container && window.ResizeObserver) {
    const ro = new ResizeObserver(() => {
      if (!isGraphPanelVisible()) return;
      resizeGraphCanvas();
      reheatSimulation(0.3);
    });
    ro.observe(container);
  } else {
    window.addEventListener('resize', () => {
      if (!isGraphPanelVisible()) return;
      resizeGraphCanvas();
      reheatSimulation(0.3);
    });
  }

  // Hook tab activation
  window.onGraphTabActivated = () => {
    if (!isGraphPanelVisible()) return;
    resizeGraphCanvas();
    if (!graphState.initialized) {
      loadGraphData();
    } else {
      reheatSimulation(0.4);
      startAnimationLoop();
    }
  };

  initGraphControls();
  initCanvasInteractions(canvas);
}

async function loadGraphData() {
  if (graphState.loading) return;
  graphState.loading = true;

  try {
    const res = await fetch('/api/graph');
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: Failed to load graph data`);
    }

    const data = await res.json();
    graphState.rawNodes = data.nodes || [];
    graphState.rawEdges = data.edges || [];

    buildGraphModel();
    graphState.initialized = true;
    resizeGraphCanvas();
    resetGraphView();
    startAnimationLoop();
  } catch (err) {
    console.error('Failed to load graph data:', err);
    showToast(`Graph load failed: ${err.message}`, 'error');
  } finally {
    graphState.loading = false;
  }
}

function buildGraphModel() {
  const canvas = document.getElementById('graph-canvas');
  const width = canvas ? canvas.clientWidth : 800;
  const height = canvas ? canvas.clientHeight : 600;
  const cx = width / 2;
  const cy = height / 2;

  graphState.nodeMap.clear();

  // 1. Process Nodes
  graphState.nodes = graphState.rawNodes.map((raw, idx) => {
    const tierName = classifyNodeTier(raw);
    const tierCfg = HIERARCHY_CONFIG[tierName] || HIERARCHY_CONFIG['L3 Evergreen'];

    // Initial positioning in circular cluster
    const angle = (idx / Math.max(graphState.rawNodes.length, 1)) * Math.PI * 2;
    const radius = 100 + Math.random() * 150;
    const x = cx + Math.cos(angle) * radius + (Math.random() - 0.5) * 40;
    const y = cy + Math.sin(angle) * radius + (Math.random() - 0.5) * 40;

    const node = {
      id: raw.id,
      title: raw.title || raw.id,
      hierarchy: raw.hierarchy,
      tags: raw.tags || [],
      tierName,
      tierConfig: tierCfg,
      x,
      y,
      vx: 0,
      vy: 0,
      radius: tierCfg.baseRadius,
      degree: 0,
      outLinks: [],
      inLinks: [],
      unresolvedOut: [],
      isOrphan: true,
      visible: graphState.activeTiers.has(tierName),
      pinned: false,
    };

    // Index by multiple keys for link matching
    graphState.nodeMap.set(node.id, node);
    graphState.nodeMap.set(node.id.toLowerCase(), node);
    graphState.nodeMap.set(node.title.toLowerCase(), node);

    // Index by stem without directory and .md
    const filename = node.id.split('/').pop() || node.id;
    const stem = filename.replace(/\.md$/i, '');
    graphState.nodeMap.set(stem.toLowerCase(), node);

    return node;
  });

  // 2. Resolve Edges
  const linkSet = new Set();
  graphState.links = [];

  graphState.rawEdges.forEach(edge => {
    const sourceNode = findNodeByKey(edge.source);
    const targetNode = findNodeByKey(edge.target);

    if (sourceNode && targetNode && sourceNode !== targetNode) {
      const linkKey = `${sourceNode.id}->${targetNode.id}`;
      if (!linkSet.has(linkKey)) {
        linkSet.add(linkKey);
        graphState.links.push({
          source: sourceNode,
          target: targetNode,
        });
        sourceNode.outLinks.push(targetNode);
        targetNode.inLinks.push(sourceNode);
        sourceNode.degree++;
        targetNode.degree++;
      }
    } else if (sourceNode && edge.target) {
      sourceNode.unresolvedOut.push(edge.target);
    }
  });

  // 3. Mark orphan status and compute radius
  graphState.nodes.forEach(node => {
    node.isOrphan = (node.outLinks.length === 0 && node.inLinks.length === 0);
    // Slight radius boost based on degree
    node.radius = node.tierConfig.baseRadius + Math.min(node.degree, 10) * 0.7;
  });

  updateGraphCounter();
}

function findNodeByKey(key) {
  if (!key) return null;
  const k = String(key).trim();
  const direct = graphState.nodeMap.get(k) || graphState.nodeMap.get(k.toLowerCase());
  if (direct) return direct;

  // Try stripping .md or path
  const stem = k.split('/').pop().replace(/\.md$/i, '').toLowerCase();
  return graphState.nodeMap.get(stem) || null;
}

function updateGraphCounter() {
  const counterEl = document.getElementById('graph-counter');
  if (!counterEl) return;

  const visibleNodes = graphState.nodes.filter(n => n.visible);
  const visibleLinks = graphState.links.filter(l => l.source.visible && l.target.visible);
  counterEl.textContent = t('graph.counter', { nodes: visibleNodes.length, links: visibleLinks.length });
}

function resizeGraphCanvas() {
  const canvas = document.getElementById('graph-canvas');
  const container = document.getElementById('graph-viewport-container');
  if (!canvas || !container) return;

  const dpr = window.devicePixelRatio || 1;
  const width = container.clientWidth || 800;
  const height = container.clientHeight || 600;

  if (canvas.width !== width * dpr || canvas.height !== height * dpr) {
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
  }
}

function resetGraphView() {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas) return;

  const width = canvas.clientWidth || 800;
  const height = canvas.clientHeight || 600;
  const visibleNodes = graphState.nodes.filter(n => n.visible);

  if (visibleNodes.length === 0) {
    graphState.transform = { x: 0, y: 0, scale: 1 };
    return;
  }

  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  visibleNodes.forEach(n => {
    minX = Math.min(minX, n.x - n.radius);
    minY = Math.min(minY, n.y - n.radius);
    maxX = Math.max(maxX, n.x + n.radius);
    maxY = Math.max(maxY, n.y + n.radius);
  });

  const spanX = Math.max(maxX - minX, 100);
  const spanY = Math.max(maxY - minY, 100);
  const padding = 80;

  const scaleX = (width - padding * 2) / spanX;
  const scaleY = (height - padding * 2) / spanY;
  const scale = Math.min(Math.max(Math.min(scaleX, scaleY), 0.35), 1.6);

  const cx = (minX + maxX) / 2;
  const cy = (minY + maxY) / 2;

  graphState.transform = {
    x: width / 2 - cx * scale,
    y: height / 2 - cy * scale,
    scale,
  };
}

function reheatSimulation(alpha = 0.6) {
  if (!isGraphPanelVisible()) return;
  graphState.simulation.alpha = Math.max(graphState.simulation.alpha, alpha);
  startAnimationLoop();
}

function startAnimationLoop() {
  if (!isGraphPanelVisible()) return;
  if (graphState.animFrameId) return;

  function loop() {
    if (!isGraphPanelVisible()) {
      graphState.animFrameId = null;
      return;
    }
    tickPhysics();
    renderGraph();

    // Keep loop active while simulation has energy or user is interacting
    const isSimulating = graphState.simulation.alpha > graphState.simulation.alphaMin;
    const isInteracting = graphState.isDraggingCanvas || graphState.draggedNode != null;
    const isOrphanActive = graphState.highlightOrphans;

    if (isSimulating || isInteracting || isOrphanActive) {
      graphState.animFrameId = requestAnimationFrame(loop);
    } else {
      graphState.animFrameId = null;
      renderGraph(); // Final crisp frame
    }
  }

  graphState.animFrameId = requestAnimationFrame(loop);
}

function tickPhysics() {
  if (!isGraphPanelVisible()) return;
  const visibleNodes = graphState.nodes.filter(n => n.visible);
  const visibleLinks = graphState.links.filter(l => l.source.visible && l.target.visible);
  const count = visibleNodes.length;
  if (count === 0) return;

  const alpha = graphState.simulation.alpha;
  const canvas = document.getElementById('graph-canvas');
  const width = canvas ? canvas.clientWidth : 800;
  const height = canvas ? canvas.clientHeight : 600;
  const cx = width / 2;
  const cy = height / 2;

  // 1. Repulsion between all visible node pairs
  const kRepulse = 750;
  for (let i = 0; i < count; i++) {
    const ni = visibleNodes[i];
    for (let j = i + 1; j < count; j++) {
      const nj = visibleNodes[j];
      let dx = nj.x - ni.x;
      let dy = nj.y - ni.y;
      let dist2 = dx * dx + dy * dy;
      if (dist2 < 1) {
        dx = (Math.random() - 0.5) * 2;
        dy = (Math.random() - 0.5) * 2;
        dist2 = 1;
      }
      const dist = Math.sqrt(dist2);
      const minDistance = ni.radius + nj.radius + 20;

      const force = (kRepulse / (dist2 + 100)) * alpha;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;

      if (!ni.pinned) { ni.vx -= fx; ni.vy -= fy; }
      if (!nj.pinned) { nj.vx += fx; nj.vy += fy; }

      // Stronger push if overlapping
      if (dist < minDistance) {
        const overlap = (minDistance - dist) * 0.45 * alpha;
        const ox = (dx / dist) * overlap;
        const oy = (dy / dist) * overlap;
        if (!ni.pinned) { ni.vx -= ox; ni.vy -= oy; }
        if (!nj.pinned) { nj.vx += ox; nj.vy += oy; }
      }
    }
  }

  // 2. Link Spring Attraction
  const targetLinkDist = 95;
  const kSpring = 0.045;
  for (let i = 0; i < visibleLinks.length; i++) {
    const link = visibleLinks[i];
    const s = link.source;
    const t = link.target;
    let dx = t.x - s.x;
    let dy = t.y - s.y;
    let dist = Math.sqrt(dx * dx + dy * dy);
    if (dist < 0.001) dist = 0.001;
    const displacement = dist - targetLinkDist;
    const force = displacement * kSpring * alpha;
    const fx = (dx / dist) * force;
    const fy = (dy / dist) * force;

    if (!s.pinned) { s.vx += fx; s.vy += fy; }
    if (!t.pinned) { t.vx += fx; t.vy += fy; }
  }

  // 3. Weak Centering Gravity
  const kCenter = 0.015;
  for (let i = 0; i < count; i++) {
    const node = visibleNodes[i];
    if (!node.pinned) {
      node.vx += (cx - node.x) * kCenter * alpha;
      node.vy += (cy - node.y) * kCenter * alpha;
    }
  }

  // 4. Velocity damping & position update
  const damping = 0.65;
  for (let i = 0; i < count; i++) {
    const node = visibleNodes[i];
    if (node.pinned) {
      node.vx = 0;
      node.vy = 0;
      continue;
    }
    node.vx *= damping;
    node.vy *= damping;
    node.x += node.vx;
    node.y += node.vy;
  }

  // 5. Alpha decay
  graphState.simulation.alpha = Math.max(
    graphState.simulation.alphaMin,
    graphState.simulation.alpha * (1 - graphState.simulation.alphaDecay)
  );
}

function renderGraph() {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas) return;

  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;

  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, width, height);

  // Apply Camera Transform
  ctx.translate(graphState.transform.x, graphState.transform.y);
  ctx.scale(graphState.transform.scale, graphState.transform.scale);

  const visibleNodes = graphState.nodes.filter(n => n.visible);
  const visibleLinks = graphState.links.filter(l => l.source.visible && l.target.visible);

  const focusNode = graphState.hoveredNode || graphState.selectedNode;
  const connectedIds = new Set();
  if (focusNode) {
    connectedIds.add(focusNode.id);
    focusNode.outLinks.forEach(n => connectedIds.add(n.id));
    focusNode.inLinks.forEach(n => connectedIds.add(n.id));
  }

  // 1. Draw Links
  for (let i = 0; i < visibleLinks.length; i++) {
    const link = visibleLinks[i];
    const isConnected = focusNode &&
      (link.source === focusNode || link.target === focusNode);

    ctx.beginPath();
    ctx.moveTo(link.source.x, link.source.y);
    ctx.lineTo(link.target.x, link.target.y);

    if (focusNode) {
      if (isConnected) {
        ctx.strokeStyle = '#22C55E';
        ctx.lineWidth = 2.2;
      } else {
        ctx.strokeStyle = 'rgba(148, 163, 184, 0.05)';
        ctx.lineWidth = 1.0;
      }
    } else {
      ctx.strokeStyle = 'rgba(148, 163, 184, 0.22)';
      ctx.lineWidth = 1.2;
    }
    ctx.stroke();
  }

  // 2. Draw Nodes
  const now = Date.now();
  for (let i = 0; i < visibleNodes.length; i++) {
    const node = visibleNodes[i];
    const isFocus = node === focusNode;
    const isConnected = focusNode ? connectedIds.has(node.id) : true;

    ctx.save();
    if (focusNode && !isConnected) {
      ctx.globalAlpha = 0.22;
    }

    // Orphan Pulse Ring
    if (graphState.highlightOrphans && node.isOrphan) {
      const pulse = Math.sin(now / 220);
      const ringRadius = node.radius + 6 + pulse * 3;
      ctx.beginPath();
      ctx.arc(node.x, node.y, Math.max(ringRadius, node.radius + 2), 0, Math.PI * 2);
      ctx.strokeStyle = 'rgba(239, 68, 68, 0.85)';
      ctx.lineWidth = 2.0;
      ctx.stroke();
    }

    // Selected / Hovered Halo
    if (isFocus) {
      ctx.beginPath();
      ctx.arc(node.x, node.y, node.radius + 5, 0, Math.PI * 2);
      ctx.strokeStyle = '#FFFFFF';
      ctx.lineWidth = 2.0;
      ctx.stroke();
    }

    // Node Body
    ctx.beginPath();
    ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
    ctx.fillStyle = node.tierConfig.color;
    ctx.fill();
    ctx.strokeStyle = isFocus ? '#FFFFFF' : 'rgba(255, 255, 255, 0.25)';
    ctx.lineWidth = isFocus ? 2 : 1;
    ctx.stroke();

    // Node Label
    const showLabel = isFocus || isConnected || node.radius >= 12 || graphState.transform.scale > 1.1;
    if (showLabel) {
      ctx.font = '11px "JetBrains Mono", ui-monospace, monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';

      const labelText = node.title || node.id;
      const truncated = labelText.length > 22 ? labelText.slice(0, 20) + '..' : labelText;

      // Subtle shadow for legibility
      ctx.fillStyle = 'rgba(11, 17, 32, 0.85)';
      ctx.fillText(truncated, node.x + 1, node.y + node.radius + 6);
      ctx.fillStyle = isFocus ? '#FFFFFF' : (isConnected ? '#E2E8F0' : '#94A3B8');
      ctx.fillText(truncated, node.x, node.y + node.radius + 5);
    }

    ctx.restore();
  }

  ctx.restore();
}

function initCanvasInteractions(canvas) {
  function screenToWorld(clientX, clientY) {
    const rect = canvas.getBoundingClientRect();
    const sx = clientX - rect.left;
    const sy = clientY - rect.top;
    const wx = (sx - graphState.transform.x) / graphState.transform.scale;
    const wy = (sy - graphState.transform.y) / graphState.transform.scale;
    return { wx, wy, sx, sy };
  }

  function findNodeAt(worldX, worldY) {
    const visibleNodes = graphState.nodes.filter(n => n.visible);
    for (let i = visibleNodes.length - 1; i >= 0; i--) {
      const node = visibleNodes[i];
      const dx = worldX - node.x;
      const dy = worldY - node.y;
      const hitRadius = Math.max(node.radius + 6, 14);
      if (dx * dx + dy * dy <= hitRadius * hitRadius) {
        return node;
      }
    }
    return null;
  }

  // Pointer Down
  canvas.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return; // Only left click
    const { wx, wy } = screenToWorld(e.clientX, e.clientY);
    graphState.mouseDownPos = { x: e.clientX, y: e.clientY };

    const hit = findNodeAt(wx, wy);
    if (hit) {
      graphState.draggedNode = hit;
      hit.pinned = true;
      reheatSimulation(0.4);
    } else {
      graphState.isDraggingCanvas = true;
      canvas.classList.add('dragging');
      graphState.dragStartMouse = { x: e.clientX, y: e.clientY };
      graphState.dragStartTransform = { ...graphState.transform };
    }
  });

  // Pointer Move
  window.addEventListener('mousemove', (e) => {
    if (graphState.draggedNode) {
      const { wx, wy } = screenToWorld(e.clientX, e.clientY);
      graphState.draggedNode.x = wx;
      graphState.draggedNode.y = wy;
      graphState.draggedNode.vx = 0;
      graphState.draggedNode.vy = 0;
      reheatSimulation(0.25);
      return;
    }

    if (graphState.isDraggingCanvas) {
      const dx = e.clientX - graphState.dragStartMouse.x;
      const dy = e.clientY - graphState.dragStartMouse.y;
      graphState.transform.x = graphState.dragStartTransform.x + dx;
      graphState.transform.y = graphState.dragStartTransform.y + dy;
      renderGraph();
      return;
    }

    // Hover detection when inside canvas
    const rect = canvas.getBoundingClientRect();
    if (
      e.clientX >= rect.left &&
      e.clientX <= rect.right &&
      e.clientY >= rect.top &&
      e.clientY <= rect.bottom
    ) {
      const { wx, wy } = screenToWorld(e.clientX, e.clientY);
      const hit = findNodeAt(wx, wy);
      if (hit !== graphState.hoveredNode) {
        graphState.hoveredNode = hit;
        canvas.style.cursor = hit ? 'pointer' : 'grab';
        renderGraph();
      }
    } else if (graphState.hoveredNode) {
      graphState.hoveredNode = null;
      renderGraph();
    }
  });

  // Pointer Up
  window.addEventListener('mouseup', (e) => {
    const distMoved = Math.hypot(
      e.clientX - graphState.mouseDownPos.x,
      e.clientY - graphState.mouseDownPos.y
    );

    if (graphState.draggedNode) {
      graphState.draggedNode.pinned = false;
      // If it was just a click without much movement, select the node
      if (distMoved < 5) {
        selectNode(graphState.draggedNode);
      }
      graphState.draggedNode = null;
      reheatSimulation(0.15);
    } else if (graphState.isDraggingCanvas) {
      graphState.isDraggingCanvas = false;
      canvas.classList.remove('dragging');
      if (distMoved < 5) {
        // Click on background closes inspector
        deselectNode();
      }
    }
  });

  // Wheel Zoom
  canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    const zoomFactor = e.deltaY < 0 ? 1.12 : 0.89;
    const currentScale = graphState.transform.scale;
    const newScale = Math.min(Math.max(currentScale * zoomFactor, 0.15), 4.0);

    // Zoom centered on cursor position
    graphState.transform.x = mouseX - (mouseX - graphState.transform.x) * (newScale / currentScale);
    graphState.transform.y = mouseY - (mouseY - graphState.transform.y) * (newScale / currentScale);
    graphState.transform.scale = newScale;

    renderGraph();
  }, { passive: false });
}

function initGraphControls() {
  // Zoom Controls
  const btnZoomIn = document.getElementById('btn-zoom-in');
  const btnZoomOut = document.getElementById('btn-zoom-out');
  const btnZoomFit = document.getElementById('btn-zoom-fit');

  if (btnZoomIn) {
    btnZoomIn.addEventListener('click', () => {
      zoomBy(1.25);
    });
  }
  if (btnZoomOut) {
    btnZoomOut.addEventListener('click', () => {
      zoomBy(0.8);
    });
  }
  if (btnZoomFit) {
    btnZoomFit.addEventListener('click', () => {
      resetGraphView();
      renderGraph();
    });
  }

  // Hierarchy Filter Buttons
  const filterPills = document.querySelectorAll('.btn-filter-pill');
  filterPills.forEach(pill => {
    pill.addEventListener('click', () => {
      const tier = pill.dataset.tier;
      if (!tier) return;

      if (graphState.activeTiers.has(tier)) {
        // If clicking last active tier, prevent emptying completely
        if (graphState.activeTiers.size > 1) {
          graphState.activeTiers.delete(tier);
          pill.classList.remove('active');
        }
      } else {
        graphState.activeTiers.add(tier);
        pill.classList.add('active');
      }

      // Update node visibility
      graphState.nodes.forEach(node => {
        node.visible = graphState.activeTiers.has(node.tierName);
      });

      updateGraphCounter();
      reheatSimulation(0.5);
    });
  });

  // Highlight Orphans Toggle
  const toggleOrphans = document.getElementById('toggle-orphans');
  if (toggleOrphans) {
    toggleOrphans.addEventListener('change', () => {
      graphState.highlightOrphans = toggleOrphans.checked;
      reheatSimulation(0.2);
    });
  }

  // Node Search Filter Input
  const searchInput = document.getElementById('graph-search');
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const query = searchInput.value.trim().toLowerCase();
      graphState.searchQuery = query;

      if (!query) {
        renderGraph();
        return;
      }

      // Find best matching node
      const match = graphState.nodes.find(n =>
        n.visible && (
          n.title.toLowerCase().includes(query) ||
          n.id.toLowerCase().includes(query) ||
          n.tags.some(t => t.toLowerCase().includes(query))
        )
      );

      if (match) {
        focusOnNode(match);
      } else {
        renderGraph();
      }
    });

    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const query = searchInput.value.trim().toLowerCase();
        if (!query) return;
        const match = graphState.nodes.find(n =>
          n.visible && (
            n.title.toLowerCase().includes(query) ||
            n.id.toLowerCase().includes(query)
          )
        );
        if (match) {
          selectNode(match);
          focusOnNode(match);
        }
      }
    });
  }

  // Drawer Close Button
  const drawerCloseBtn = document.getElementById('graph-drawer-close');
  if (drawerCloseBtn) {
    drawerCloseBtn.addEventListener('click', () => {
      deselectNode();
    });
  }
}

function zoomBy(factor) {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas) return;

  const width = canvas.clientWidth || 800;
  const height = canvas.clientHeight || 600;
  const cx = width / 2;
  const cy = height / 2;

  const currentScale = graphState.transform.scale;
  const newScale = Math.min(Math.max(currentScale * factor, 0.15), 4.0);

  graphState.transform.x = cx - (cx - graphState.transform.x) * (newScale / currentScale);
  graphState.transform.y = cy - (cy - graphState.transform.y) * (newScale / currentScale);
  graphState.transform.scale = newScale;

  renderGraph();
}

function focusOnNode(node) {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas || !node) return;

  const width = canvas.clientWidth || 800;
  const height = canvas.clientHeight || 600;
  const targetScale = Math.max(graphState.transform.scale, 1.1);

  graphState.transform.scale = targetScale;
  graphState.transform.x = width / 2 - node.x * targetScale;
  graphState.transform.y = height / 2 - node.y * targetScale;

  renderGraph();
}

function selectNode(node) {
  graphState.selectedNode = node;
  openDrawer(node);
  renderGraph();
}

function deselectNode() {
  graphState.selectedNode = null;
  closeDrawer();
  renderGraph();
}

function openDrawer(node) {
  const drawer = document.getElementById('graph-inspector') || document.querySelector('.graph-drawer');
  if (!drawer || !node) return;

  drawer.classList.add('open');
  drawer.setAttribute('aria-hidden', 'false');

  // Title & Path
  const titleEl = document.getElementById('drawer-title');
  if (titleEl) titleEl.textContent = node.title || node.id;

  const pathEl = document.getElementById('drawer-path');
  if (pathEl) pathEl.textContent = node.id;

  // Hierarchy Badge
  const hierarchyEl = document.getElementById('drawer-hierarchy');
  if (hierarchyEl) {
    hierarchyEl.textContent = getTierDisplayName(node.tierName);
    hierarchyEl.className = `badge-tier ${node.tierConfig.badgeClass}`;
  }

  // Tags
  const tagsEl = document.getElementById('drawer-tags');
  if (tagsEl) {
    if (node.tags && node.tags.length > 0) {
      tagsEl.innerHTML = node.tags.map(t => `<span class="drawer-tag-pill">#${escapeHtml(t)}</span>`).join('');
    } else {
      tagsEl.innerHTML = `<span class="drawer-empty-hint" data-i18n="graph.drawer_no_tags">${t('graph.drawer_no_tags')}</span>`;
    }
  }

  // Out-Links
  const outLinksEl = document.getElementById('drawer-outlinks');
  const outCountEl = document.getElementById('drawer-outlinks-count');
  const allOut = [...node.outLinks, ...node.unresolvedOut.map(target => ({ id: target, title: target, unresolved: true }))];

  if (outCountEl) outCountEl.textContent = `(${allOut.length})`;
  if (outLinksEl) {
    if (allOut.length > 0) {
      outLinksEl.innerHTML = allOut.map(target => {
        const title = target.title || target.id;
        const isClickable = !target.unresolved;
        return `
          <li class="drawer-link-item ${isClickable ? 'clickable' : 'unresolved'}" data-id="${escapeHtml(target.id)}" title="${escapeHtml(target.id)}">
            <span class="drawer-link-title mono">${escapeHtml(title)}</span>
            <svg class="drawer-link-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
          </li>
        `;
      }).join('');

      // Add click navigation
      outLinksEl.querySelectorAll('.drawer-link-item.clickable').forEach(item => {
        item.addEventListener('click', () => {
          const targetId = item.dataset.id;
          const targetNode = findNodeByKey(targetId);
          if (targetNode) {
            if (!targetNode.visible) {
              graphState.activeTiers.add(targetNode.tierName);
              targetNode.visible = true;
              const pill = document.querySelector(`.btn-filter-pill[data-tier="${targetNode.tierName}"]`);
              if (pill) pill.classList.add('active');
              updateGraphCounter();
            }
            selectNode(targetNode);
            focusOnNode(targetNode);
          }
        });
      });
    } else {
      outLinksEl.innerHTML = `<li class="drawer-empty-hint" data-i18n="graph.drawer_no_outlinks">${t('graph.drawer_no_outlinks')}</li>`;
    }
  }

  // Inbound Backlinks
  const backlinksEl = document.getElementById('drawer-backlinks');
  const backCountEl = document.getElementById('drawer-backlinks-count');

  if (backCountEl) backCountEl.textContent = `(${node.inLinks.length})`;
  if (backlinksEl) {
    if (node.inLinks.length > 0) {
      backlinksEl.innerHTML = node.inLinks.map(sourceNode => `
        <li class="drawer-link-item clickable" data-id="${escapeHtml(sourceNode.id)}" title="${escapeHtml(sourceNode.id)}">
          <span class="drawer-link-title mono">${escapeHtml(sourceNode.title || sourceNode.id)}</span>
          <svg class="drawer-link-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
        </li>
      `).join('');

      backlinksEl.querySelectorAll('.drawer-link-item.clickable').forEach(item => {
        item.addEventListener('click', () => {
          const sourceId = item.dataset.id;
          const sourceNode = findNodeByKey(sourceId);
          if (sourceNode) {
            if (!sourceNode.visible) {
              graphState.activeTiers.add(sourceNode.tierName);
              sourceNode.visible = true;
              const pill = document.querySelector(`.btn-filter-pill[data-tier="${sourceNode.tierName}"]`);
              if (pill) pill.classList.add('active');
              updateGraphCounter();
            }
            selectNode(sourceNode);
            focusOnNode(sourceNode);
          }
        });
      });
    } else {
      backlinksEl.innerHTML = `<li class="drawer-empty-hint" data-i18n="graph.drawer_no_backlinks">${t('graph.drawer_no_backlinks')}</li>`;
    }
  }
}

function closeDrawer() {
  const drawer = document.getElementById('graph-inspector') || document.querySelector('.graph-drawer');
  if (!drawer) return;
  drawer.classList.remove('open');
  drawer.setAttribute('aria-hidden', 'true');
}

/**
 * ==========================================================================
 * Panel 3: Log & Trace Inspector & Offline Mermaid SVG Renderer (Ticket 23)
 * ==========================================================================
 */

let cachedLogs = [];
let selectedLogId = null;
let currentLogData = null;

function initLogInspector() {
  const searchInput = document.getElementById('logs-search');
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      renderLogsList(searchInput.value.trim().toLowerCase());
    });
  }

  // Copy buttons
  const copyIdBtn = document.getElementById('btn-copy-node-id');
  if (copyIdBtn) {
    copyIdBtn.addEventListener('click', () => {
      if (selectedLogId) {
        copyToClipboard(selectedLogId, copyIdBtn, 'logs.copy_node_id');
      }
    });
  }

  const copyLogBtn = document.getElementById('btn-copy-raw-log');
  if (copyLogBtn) {
    copyLogBtn.addEventListener('click', () => {
      if (currentLogData && currentLogData.content) {
        copyToClipboard(currentLogData.content, copyLogBtn, 'logs.copy_raw_log');
      }
    });
  }

  // Raw log collapsible toggle
  const rawToggleBtn = document.getElementById('raw-log-toggle-btn');
  const rawCard = document.querySelector('.raw-log-card');
  if (rawToggleBtn && rawCard) {
    rawToggleBtn.addEventListener('click', () => {
      rawCard.classList.toggle('collapsed');
      const isExpanded = !rawCard.classList.contains('collapsed');
      rawToggleBtn.setAttribute('aria-expanded', isExpanded ? 'true' : 'false');
    });
    rawToggleBtn.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        rawToggleBtn.click();
      }
    });
  }
}

async function loadLogsList() {
  const container = document.getElementById('logs-list');
  const badgeEl = document.getElementById('logs-count-badge');
  if (!container) return;

  try {
    const res = await fetch('/api/logs');
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const logs = await res.json();
    cachedLogs = Array.isArray(logs) ? logs : [];

    if (badgeEl) {
      badgeEl.textContent = t('logs.badge', { count: cachedLogs.length });
    }

    const searchInput = document.getElementById('logs-search');
    const filter = searchInput ? searchInput.value.trim().toLowerCase() : '';
    renderLogsList(filter);

    // If there are logs and none selected, auto-select the first log
    if (cachedLogs.length > 0 && !selectedLogId) {
      selectLogNode(cachedLogs[0].id);
    }

    // Refresh token metrics using the updated cachedLogs
    if (window.latestStatus) {
      updateTokenMetrics(window.latestStatus);
    }
  } catch (err) {
    console.error('Failed to load logs:', err);
    container.innerHTML = `
      <div class="placeholder-state logs-empty-state">
        <p class="placeholder-text" style="color: var(--danger);">${t('logs.raw_failed', { error: escapeHtml(err.message) })}</p>
      </div>
    `;
  }
}

function renderLogsList(filter = '') {
  const container = document.getElementById('logs-list');
  if (!container) return;

  const filtered = cachedLogs.filter(log => {
    if (!filter) return true;
    const matchId = log.id && log.id.toLowerCase().includes(filter);
    const matchTask = log.task_id && log.task_id.toLowerCase().includes(filter);
    return matchId || matchTask;
  });

  if (filtered.length === 0) {
    if (cachedLogs.length === 0) {
      container.innerHTML = `
        <div class="placeholder-state logs-empty-state" id="logs-empty-hint">
          <svg class="icon icon-xl icon-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14 2 14 8 20 8"></polyline>
          </svg>
          <p class="placeholder-text" data-i18n="logs.empty_title">${t('logs.empty_title')}</p>
          <span class="placeholder-subtext mono" data-i18n="logs.empty_subtext">${t('logs.empty_subtext')}</span>
        </div>
      `;
    } else {
      container.innerHTML = `
        <div class="placeholder-state logs-empty-state">
          <p class="placeholder-text">${t('logs.no_match', { filter: escapeHtml(filter) })}</p>
          <span class="placeholder-subtext mono" data-i18n="logs.no_match_sub">${t('logs.no_match_sub')}</span>
        </div>
      `;
    }
    return;
  }

  container.innerHTML = filtered.map(log => {
    const isSelected = log.id === selectedLogId;
    const taskBadge = log.task_id
      ? `<span class="log-item-task mono" title="Task ID: ${escapeHtml(log.task_id)}">${escapeHtml(log.task_id)}</span>`
      : '';
    const dateFormatted = formatLogDate(log.created_at);

    return `
      <div class="log-item ${isSelected ? 'active' : ''}" data-id="${escapeHtml(log.id)}" role="button" tabindex="0">
        <div class="log-item-header">
          <span class="log-item-id mono">${escapeHtml(log.id)}</span>
          <span class="badge badge-muted mono log-item-lines">${t('logs.lines', { count: log.line_count })}</span>
        </div>
        <div class="log-item-meta">
          ${taskBadge}
          <span class="mono">${dateFormatted}</span>
        </div>
      </div>
    `;
  }).join('');

  // Bind click & keyboard handlers
  container.querySelectorAll('.log-item').forEach(item => {
    item.addEventListener('click', () => {
      const id = item.dataset.id;
      selectLogNode(id);
    });
    item.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        item.click();
      }
    });
  });
}

function formatLogDate(isoString) {
  if (!isoString) return '--';
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return isoString;
    return d.toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch (_) {
    return isoString;
  }
}

async function selectLogNode(nodeId) {
  if (!nodeId) return;
  selectedLogId = nodeId;

  // Highlight active element in list
  document.querySelectorAll('#logs-list .log-item').forEach(el => {
    if (el.dataset.id === nodeId) {
      el.classList.add('active');
    } else {
      el.classList.remove('active');
    }
  });

  const detailIdEl = document.getElementById('log-detail-id');
  const detailTaskEl = document.getElementById('log-detail-task');
  const detailLinesEl = document.getElementById('log-detail-lines');
  const detailTimeEl = document.getElementById('log-detail-time');
  const copyIdBtn = document.getElementById('btn-copy-node-id');
  const copyLogBtn = document.getElementById('btn-copy-raw-log');
  const mermaidViewer = document.getElementById('log-mermaid-viewer');
  const rawViewer = document.getElementById('log-raw-viewer');
  const rawLineBadge = document.getElementById('raw-log-line-badge');

  if (detailIdEl) detailIdEl.textContent = nodeId;
  if (copyIdBtn) copyIdBtn.disabled = false;
  if (copyLogBtn) copyLogBtn.disabled = true;

  // Populate from cachedLogs metadata
  const meta = cachedLogs.find(l => l.id === nodeId);
  if (meta) {
    if (detailTaskEl) {
      if (meta.task_id) {
        detailTaskEl.textContent = meta.task_id;
        detailTaskEl.style.display = 'inline-flex';
      } else {
        detailTaskEl.style.display = 'none';
      }
    }
    if (detailLinesEl) {
      detailLinesEl.textContent = t('logs.lines', { count: meta.line_count });
      detailLinesEl.style.display = 'inline-flex';
    }
    if (detailTimeEl) {
      detailTimeEl.textContent = meta.created_at;
    }
    if (rawLineBadge) {
      rawLineBadge.textContent = t('logs.lines', { count: meta.line_count });
    }
  }

  // Placeholder while loading
  if (mermaidViewer) {
    mermaidViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" data-i18n="logs.loading_diagram">${t('logs.loading_diagram')}</p></div>`;
  }
  if (rawViewer) {
    rawViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" data-i18n="logs.loading_raw">${t('logs.loading_raw')}</p></div>`;
  }

  try {
    const res = await fetch(`/api/logs/${encodeURIComponent(nodeId)}`);
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.error || `HTTP ${res.status}`);
    }
    const data = await res.json();
    currentLogData = data;

    if (copyLogBtn) copyLogBtn.disabled = false;

    // Render Mermaid State Diagram natively
    if (mermaidViewer) {
      renderMermaidToSvg(data.mermaid, mermaidViewer);
    }

    // Render Syntax-Highlighted Raw Log Slice
    if (rawViewer) {
      renderRawLogSlice(data.content, rawViewer);
    }
  } catch (err) {
    console.error('Failed to inspect log node:', err);
    if (mermaidViewer) {
      mermaidViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" style="color: var(--danger);">${t('logs.diagram_failed', { error: escapeHtml(err.message) })}</p></div>`;
    }
    if (rawViewer) {
      rawViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" style="color: var(--danger);">${t('logs.raw_failed', { error: escapeHtml(err.message) })}</p></div>`;
    }
  }
}

function renderRawLogSlice(rawContent, container) {
  if (!rawContent || !container) {
    container.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" data-i18n="logs.log_empty">${t('logs.log_empty')}</p></div>`;
    return;
  }

  const lines = rawContent.split(/\r?\n/);
  const totalLines = lines.length;

  const rawLineBadge = document.getElementById('raw-log-line-badge');
  if (rawLineBadge) {
    rawLineBadge.textContent = t('logs.lines', { count: totalLines });
  }

  const rowsHtml = lines.map((line, idx) => {
    const lineNum = idx + 1;
    const lower = line.toLowerCase();

    let highlightClass = '';
    if (
      lower.includes('error') ||
      lower.includes('failed') ||
      lower.includes('panicked') ||
      lower.includes('exception')
    ) {
      highlightClass = 'line-error';
    } else if (lower.includes('warning')) {
      highlightClass = 'line-warning';
    }

    return `
      <div class="log-row ${highlightClass}">
        <span class="log-num mono">${lineNum}</span>
        <span class="log-line mono">${escapeHtml(line)}</span>
      </div>
    `;
  }).join('');

  container.innerHTML = rowsHtml;
}

function copyToClipboard(text, button, originalKey = 'logs.copy_node_id') {
  if (!text) return;

  function setCopied() {
    button.classList.add('copied');
    const textSpan = button.querySelector('.btn-copy-text');
    if (textSpan) textSpan.textContent = t('logs.copied');

    const icon = button.querySelector('.copy-btn-icon');
    let prevIconHtml = '';
    if (icon) {
      prevIconHtml = icon.outerHTML;
      icon.outerHTML = `
        <svg class="icon copy-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
      `;
    }

    showToast(t('toast.copied'), 'success');

    setTimeout(() => {
      button.classList.remove('copied');
      if (textSpan) textSpan.textContent = t(originalKey);
      const currentIcon = button.querySelector('.copy-btn-icon');
      if (currentIcon && prevIconHtml) {
        currentIcon.outerHTML = prevIconHtml;
      }
    }, 2000);
  }

  if (navigator.clipboard && window.isSecureContext) {
    navigator.clipboard.writeText(text).then(setCopied).catch(fallbackCopy);
  } else {
    fallbackCopy();
  }

  function fallbackCopy() {
    try {
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.left = '-9999px';
      textarea.style.top = '0';
      document.body.appendChild(textarea);
      textarea.focus();
      textarea.select();
      const successful = document.execCommand('copy');
      document.body.removeChild(textarea);
      if (successful) {
        setCopied();
      } else {
        showToast(t('toast.copy_failed'), 'error');
      }
    } catch (e) {
      console.error('Fallback copy error:', e);
      showToast(t('toast.copy_not_supported'), 'error');
    }
  }
}

/**
 * Pure Offline SVG State Diagram & Flowchart Renderer
 * High-contrast Developer Dark Cockpit styling, zero external CDN dependencies.
 */
function renderMermaidToSvg(mermaidCode, container) {
  if (!container) return;
  if (!mermaidCode || !mermaidCode.trim()) {
    container.innerHTML = '<div class="placeholder-state"><p class="placeholder-text">No diagram definition found.</p></div>';
    return;
  }

  try {
    const raw = mermaidCode.trim();

    // Check for nested composite state: state Name { ... }
    const compositeMatch = raw.match(/state\s+([A-Za-z0-9_-]+)\s*\{([\s\S]*?)\}/);
    let compName = null;
    let compBody = null;
    let strippedCode = raw;

    if (compositeMatch) {
      compName = compositeMatch[1];
      compBody = compositeMatch[2];
      strippedCode = raw.replace(compositeMatch[0], '');
    }

    // Parse transitions
    const topTransitions = [];
    const lines = strippedCode.split('\n');
    for (let line of lines) {
      line = line.trim();
      if (!line || line.startsWith('stateDiagram') || line.startsWith('direction') || line.startsWith('flowchart') || line.startsWith('graph')) {
        continue;
      }
      const transMatch = line.match(/([*A-Za-z0-9_-]+)\s*-->\s*([*A-Za-z0-9_-]+)(?:\s*:\s*(.*))?/);
      if (transMatch) {
        topTransitions.push({
          from: transMatch[1],
          to: transMatch[2],
          label: transMatch[3] ? transMatch[3].trim() : null,
        });
      }
    }

    // If composite state exists (e.g. Offloaded)
    if (compName && compBody) {
      const startTrans = topTransitions.find(t => t.from === '[*]');
      const step1Name = startTrans ? startTrans.to : 'Running';

      const toCompTrans = topTransitions.find(t => t.to === compName);
      const toCompLabel = toCompTrans && toCompTrans.label ? toCompTrans.label : '';

      let innerStateName = 'Captured';
      const innerDescs = [];

      const innerLines = compBody.split('\n');
      for (let l of innerLines) {
        l = l.trim();
        if (!l) continue;
        const innerTrans = l.match(/([*A-Za-z0-9_-]+)\s*-->\s*([*A-Za-z0-9_-]+)/);
        if (innerTrans && innerTrans[1] === '[*]') {
          innerStateName = innerTrans[2];
        }
        const descMatch = l.match(/([A-Za-z0-9_-]+)\s*:\s*(.*)/);
        if (descMatch) {
          innerDescs.push({
            state: descMatch[1],
            text: descMatch[2].trim(),
          });
        }
      }

      const svg = generateCompositeStateSvg({
        step1Name,
        compName,
        toCompLabel,
        innerStateName,
        innerDescs,
      });
      container.innerHTML = svg;
      return;
    }

    // Generic flow SVG
    const genericSvg = generateGenericFlowSvg(topTransitions, raw);
    container.innerHTML = genericSvg;
  } catch (err) {
    console.error('Error rendering SVG diagram:', err);
    container.innerHTML = `
      <div class="placeholder-state">
        <p class="placeholder-text" style="color: var(--danger);">SVG rendering error: ${escapeHtml(err.message)}</p>
        <pre class="mono text-xs" style="text-align: left; padding: 10px; background: rgba(0,0,0,0.3); border-radius: 4px;">${escapeHtml(mermaidCode)}</pre>
      </div>
    `;
  }
}

function generateCompositeStateSvg({ step1Name, compName, toCompLabel, innerStateName, innerDescs }) {
  const width = 560;
  const cx = width / 2;

  const descCount = Math.max(innerDescs.length, 1);
  const innerCardH = 40 + descCount * 22;
  const compH = 100 + innerCardH;
  const totalH = 440 + (descCount - 3) * 22;

  const startY = 32;
  const line1StartY = startY + 10;
  const step1Y = 66;
  const step1H = 42;
  const step1W = Math.min(340, Math.max(180, (step1Name.length * 10) + 40));
  const step1X = cx - (step1W / 2);

  const line2StartY = step1Y + step1H;
  const compY = line2StartY + 56;
  const pillY = line2StartY + 16;

  const line3StartY = compY + compH;
  const endY = line3StartY + 42;

  const descItemsSvg = innerDescs.map((d, i) => {
    const textY = compY + 80 + 38 + (i * 22);
    let color = '#94A3B8';
    if (d.text.toLowerCase().includes('nodeid')) color = '#22C55E';
    else if (d.text.toLowerCase().includes('inspect')) color = '#38BDF8';
    else if (d.text.toLowerCase().includes('error') || d.text.toLowerCase().includes('failed')) color = '#EF4444';

    return `
      <text x="${cx - 190}" y="${textY}" fill="${color}" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="11" font-weight="500">
        ${escapeXml(d.text)}
      </text>
    `;
  }).join('');

  const pillText = toCompLabel || 'Log Captured';
  const pillW = Math.min(360, Math.max(160, pillText.length * 8 + 30));
  const pillX = cx - (pillW / 2);

  return `
    <svg xmlns="http://www.w3.org/2000/svg" class="mermaid-svg" viewBox="0 0 ${width} ${totalH}" width="100%" height="${totalH}">
      <defs>
        <marker id="mermaid-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M 0 1 L 8 5 L 0 9 z" fill="#64748B"/>
        </marker>
        <marker id="mermaid-arrow-emerald" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M 0 1 L 8 5 L 0 9 z" fill="#22C55E"/>
        </marker>
        <linearGradient id="comp-grad" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stop-color="#141C2E" stop-opacity="0.9"/>
          <stop offset="100%" stop-color="#0F172A" stop-opacity="0.95"/>
        </linearGradient>
      </defs>

      <!-- Step 0: Start Node -->
      <circle cx="${cx}" cy="${startY}" r="9" fill="#22C55E" />

      <!-- Arrow: Start -> Step 1 -->
      <line x1="${cx}" y1="${line1StartY}" x2="${cx}" y2="${step1Y}" stroke="#64748B" stroke-width="1.5" marker-end="url(#mermaid-arrow)" />

      <!-- Step 1: Running / Task Node -->
      <rect x="${step1X}" y="${step1Y}" width="${step1W}" height="${step1H}" rx="8" fill="#1B2336" stroke="#334155" stroke-width="1.5" />
      <text x="${cx}" y="${step1Y + 26}" fill="#F8FAFC" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="13" font-weight="600" text-anchor="middle">
        ${escapeXml(step1Name)}
      </text>

      <!-- Arrow: Step 1 -> Composite State with Label Pill -->
      <line x1="${cx}" y1="${line2StartY}" x2="${cx}" y2="${compY}" stroke="#64748B" stroke-width="1.5" marker-end="url(#mermaid-arrow)" />
      <rect x="${pillX}" y="${pillY}" width="${pillW}" height="24" rx="12" fill="#0F172A" stroke="#334155" stroke-width="1" />
      <text x="${cx}" y="${pillY + 16}" fill="#38BDF8" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="11" font-weight="500" text-anchor="middle">
        ${escapeXml(pillText)}
      </text>

      <!-- Step 2: Composite State Container (Offloaded) -->
      <rect x="50" y="${compY}" width="460" height="${compH}" rx="10" fill="url(#comp-grad)" stroke="#22C55E" stroke-width="1.5" stroke-dasharray="6,4" />
      <text x="70" y="${compY + 26}" fill="#22C55E" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="12" font-weight="700">
        state ${escapeXml(compName)}
      </text>
      <rect x="400" y="${compY + 12}" width="90" height="20" rx="4" fill="rgba(34, 197, 94, 0.15)" stroke="rgba(34, 197, 94, 0.4)" stroke-width="1" />
      <text x="445" y="${compY + 26}" fill="#22C55E" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="10" font-weight="700" text-anchor="middle">
        OFFLOADED
      </text>

      <!-- Inner Start Circle -->
      <circle cx="${cx}" cy="${compY + 54}" r="7" fill="#22C55E" />
      <line x1="${cx}" y1="${compY + 61}" x2="${cx}" y2="${compY + 76}" stroke="#64748B" stroke-width="1.5" marker-end="url(#mermaid-arrow)" />

      <!-- Inner Captured State Card -->
      <rect x="75" y="${compY + 76}" width="410" height="${innerCardH}" rx="8" fill="#1B2336" stroke="#334155" stroke-width="1.5" />
      <text x="95" y="${compY + 98}" fill="#F8FAFC" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="13" font-weight="700">
        ${escapeXml(innerStateName)}
      </text>
      <line x1="75" y1="${compY + 108}" x2="485" y2="${compY + 108}" stroke="#334155" stroke-width="1" />
      ${descItemsSvg}

      <!-- Arrow: Composite -> End -->
      <line x1="${cx}" y1="${line3StartY}" x2="${cx}" y2="${endY - 14}" stroke="#64748B" stroke-width="1.5" marker-end="url(#mermaid-arrow)" />

      <!-- Step 3: End State Circle -->
      <circle cx="${cx}" cy="${endY}" r="11" fill="none" stroke="#94A3B8" stroke-width="2" />
      <circle cx="${cx}" cy="${endY}" r="6" fill="#94A3B8" />
    </svg>
  `;
}

function generateGenericFlowSvg(transitions, raw) {
  const width = 560;
  const cx = width / 2;

  const nodeSet = [];
  function addNode(id) {
    if (!nodeSet.includes(id)) nodeSet.push(id);
  }

  transitions.forEach(t => {
    addNode(t.from);
    addNode(t.to);
  });

  if (nodeSet.length === 0) {
    nodeSet.push('[*]', 'Completed', '[*]');
  }

  const nodeHeight = 40;
  const gap = 46;
  const totalH = Math.max(260, nodeSet.length * (nodeHeight + gap) + 40);

  let nodesSvg = '';
  let linesSvg = '';

  const nodePositions = {};
  nodeSet.forEach((nodeId, idx) => {
    const y = 30 + idx * (nodeHeight + gap);
    nodePositions[nodeId] = { y, isStartEnd: nodeId === '[*]' };

    if (nodeId === '[*]') {
      if (idx === 0) {
        nodesSvg += `<circle cx="${cx}" cy="${y + 20}" r="9" fill="#22C55E" />`;
      } else {
        nodesSvg += `
          <circle cx="${cx}" cy="${y + 20}" r="11" fill="none" stroke="#94A3B8" stroke-width="2" />
          <circle cx="${cx}" cy="${y + 20}" r="6" fill="#94A3B8" />
        `;
      }
    } else {
      const boxW = Math.min(320, Math.max(160, nodeId.length * 10 + 40));
      const boxX = cx - (boxW / 2);
      nodesSvg += `
        <rect x="${boxX}" y="${y}" width="${boxW}" height="${nodeHeight}" rx="8" fill="#1B2336" stroke="#334155" stroke-width="1.5" />
        <text x="${cx}" y="${y + 25}" fill="#F8FAFC" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="12" font-weight="600" text-anchor="middle">
          ${escapeXml(nodeId)}
        </text>
      `;
    }
  });

  transitions.forEach(t => {
    const fromPos = nodePositions[t.from];
    const toPos = nodePositions[t.to];
    if (fromPos && toPos && fromPos.y < toPos.y) {
      const y1 = fromPos.isStartEnd ? fromPos.y + 29 : fromPos.y + nodeHeight;
      const y2 = toPos.isStartEnd ? toPos.y + 9 : toPos.y;
      linesSvg += `
        <line x1="${cx}" y1="${y1}" x2="${cx}" y2="${y2}" stroke="#64748B" stroke-width="1.5" marker-end="url(#mermaid-arrow)" />
      `;
      if (t.label) {
        const midY = (y1 + y2) / 2;
        const pillW = Math.min(260, Math.max(100, t.label.length * 7 + 20));
        const pillX = cx - (pillW / 2);
        linesSvg += `
          <rect x="${pillX}" y="${midY - 11}" width="${pillW}" height="22" rx="11" fill="#0F172A" stroke="#334155" stroke-width="1" />
          <text x="${cx}" y="${midY + 4}" fill="#38BDF8" font-family="'JetBrains Mono', ui-monospace, monospace" font-size="10" text-anchor="middle">${escapeXml(t.label)}</text>
        `;
      }
    }
  });

  return `
    <svg xmlns="http://www.w3.org/2000/svg" class="mermaid-svg" viewBox="0 0 ${width} ${totalH}" width="100%" height="${totalH}">
      <defs>
        <marker id="mermaid-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M 0 1 L 8 5 L 0 9 z" fill="#64748B"/>
        </marker>
      </defs>
      ${linesSvg}
      ${nodesSvg}
    </svg>
  `;
}

function escapeXml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

/**
 * ==========================================================================
 * Panel 4: Cache Health & Token Scoreboard (Ticket 23)
 * ==========================================================================
 */

function initScoreboard() {
  // Scoreboard is primarily driven by updateScoreboard(status) and updateTokenMetrics
}

function updateScoreboard(status) {
  if (!status) return;
  window.latestStatus = status;

  // Vault Documents
  const scoreboardDocEl = document.getElementById('scoreboard-doc-count');
  if (scoreboardDocEl) {
    scoreboardDocEl.textContent = Number(status.total_documents || 0).toLocaleString();
  }

  // Vector Embeddings
  const scoreboardVecEl = document.getElementById('scoreboard-vec-count');
  if (scoreboardVecEl) {
    scoreboardVecEl.textContent = Number(status.total_vectors || 0).toLocaleString();
  }

  // Vector Index Coverage %
  const docs = status.total_documents || 0;
  const vecs = status.total_vectors || 0;
  const coverage = docs > 0 ? Math.min(100, Math.round((vecs / docs) * 100)) : (vecs > 0 ? 100 : 0);
  const coverageEl = document.getElementById('scoreboard-vec-coverage');
  if (coverageEl) {
    coverageEl.textContent = `${coverage}%`;
  }

  // Cache Size
  const cacheSizeEl = document.getElementById('scoreboard-cache-size');
  if (cacheSizeEl) {
    const bytes = status.cache_size_bytes || 0;
    if (bytes >= 1048576) {
      cacheSizeEl.textContent = `${(bytes / 1048576).toFixed(2)} MB`;
    } else {
      cacheSizeEl.textContent = `${(bytes / 1024).toFixed(1)} KB`;
    }
  }

  // Total Logs
  const logCountEl = document.getElementById('scoreboard-log-count');
  if (logCountEl) {
    logCountEl.textContent = Number(status.total_logs || 0).toLocaleString();
  }

  // Vault Path & Sync Status in Card
  const cardVaultPath = document.getElementById('scoreboard-vault-path');
  if (cardVaultPath) {
    cardVaultPath.textContent = status.vault_path || t('status.unknown');
    cardVaultPath.title = status.vault_path || '';
  }

  const cardLastSync = document.getElementById('scoreboard-last-sync');
  if (cardLastSync) {
    if (status.last_sync_time) {
      const d = new Date(status.last_sync_time * 1000);
      cardLastSync.textContent = d.toLocaleString();
    } else {
      cardLastSync.textContent = t('status.never');
    }
  }

  // Token Economics
  updateTokenMetrics(status);
}

function updateTokenMetrics(status) {
  const totalLogs = (status && status.total_logs != null) ? status.total_logs : cachedLogs.length;

  let totalLines = 0;
  if (cachedLogs && cachedLogs.length > 0) {
    totalLines = cachedLogs.reduce((sum, item) => sum + (item.line_count || 0), 0);
  } else if (totalLogs > 0) {
    totalLines = totalLogs * 500;
  }

  const rawTokens = totalLines * 18;
  const mermaidOverhead = totalLogs * 60;
  const tokensSaved = Math.max(0, rawTokens - mermaidOverhead);

  const savedValEl = document.getElementById('token-saved-val');
  if (savedValEl) {
    savedValEl.textContent = tokensSaved.toLocaleString();
  }

  let trr = 99.44;
  if (rawTokens > 0) {
    trr = parseFloat(((tokensSaved / rawTokens) * 100).toFixed(2));
  } else if (totalLogs === 0) {
    trr = 99.44;
  }

  const trrValEl = document.getElementById('token-trr-val');
  if (trrValEl) {
    trrValEl.textContent = `${trr.toFixed(2)}%`;
  }

  const trrBarEl = document.getElementById('token-trr-bar');
  if (trrBarEl) {
    trrBarEl.style.width = `${Math.min(100, Math.max(0, trr))}%`;
  }
}
