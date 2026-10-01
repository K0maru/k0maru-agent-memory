/**
 * K0maru Agent Memory Hub — Developer Dark Cockpit Frontend
 * Vanilla ES6 Application Logic
 */

// Application State for Search Console
let currentSearchMode = 'hybrid';
let currentSearchLimit = 5;
let searchDebounceTimer = null;
let currentAbortController = null;

document.addEventListener('DOMContentLoaded', () => {
  initTabs();
  initSync();
  initSearchConsole();
  initGraphExplorer();
  initLogInspector();
  initScoreboard();
  initKeyboardShortcuts();
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
    const validTabs = ['search', 'graph', 'logs', 'scoreboard'];
    const activeTab = validTabs.includes(tabId) ? tabId : 'search';

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

    if (updateHash) {
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
    renderSearchResults(results, query, resultsContainer);
  } catch (err) {
    if (err.name === 'AbortError') {
      return;
    }
    console.error('Search query failed:', err);
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
      <p class="placeholder-text">Enter a query above to view explainable search ranking waterfall.</p>
      <span class="placeholder-subtext mono">Press / or ⌘K to focus search &bull; Try "memory", "storage", or "architecture"</span>
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
      <p class="placeholder-text" style="color: var(--danger);">Search execution failed: ${escapeHtml(errorMessage)}</p>
      <span class="placeholder-subtext mono">Check backend status or adjust query parameters</span>
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
        <h3 class="zero-hits-title">No matching notes found</h3>
        <p class="zero-hits-desc">Zero notes matched "<strong>${escapeHtml(query)}</strong>" in <strong>${escapeHtml(currentSearchMode)}</strong> mode.</p>
        <ul class="zero-hits-suggestions">
          <li>Try broader keywords, partial stems, or check for typos.</li>
          <li>Switch mode to <strong>Vector Dense</strong> to find semantically related notes.</li>
          <li>Click <strong>Sync Vault</strong> in the top header to ensure recent files are indexed.</li>
        </ul>
      </div>
    `;
    return;
  }

  const cardsHtml = results.map((item, idx) => {
    const tier = getHierarchyTier(item.path);
    const scoreFormatted = Number(item.score).toFixed(4);
    const highlightedSnippet = highlightQueryTerms(item.snippet, query);

    // BM25 badge
    const bm25Badge = item.bm25_rank != null
      ? `<span class="pill-bm25 hit" title="BM25 rank in candidate pool">BM25 #${item.bm25_rank}</span>`
      : `<span class="pill-bm25" title="Not ranked in BM25 lexical pool">BM25 --</span>`;

    // Vector badge
    const vectorBadge = item.vector_rank != null
      ? `<span class="pill-vector hit" title="Vector nearest neighbor rank">Vector #${item.vector_rank}</span>`
      : `<span class="pill-vector" title="Not ranked in Vector dense pool">Vector --</span>`;

    // Graph boost badge
    const graphBoostBadge = (item.graph_boost && item.graph_boost > 0)
      ? `<span class="badge badge-emerald graph-boost" title="Elevated by 1-hop WikiLinks graph connectivity">
          <svg class="graph-boost-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 3-1.9 5.8a2 2 0 0 1-1.3 1.3L3 12l5.8 1.9a2 2 0 0 1 1.3 1.3L12 21l1.9-5.8a2 2 0 0 1 1.3-1.3L21 12l-5.8-1.9a2 2 0 0 1-1.3-1.3Z"></path>
          </svg>
          +${Number(item.graph_boost).toFixed(2)} Graph Boost
        </span>`
      : '';

    return `
      <article class="search-result-card" data-path="${escapeHtml(item.path)}">
        <div class="card-header">
          <div class="card-title-group">
            <div class="card-title-row">
              <span class="badge-tier ${tier.className}">${tier.name}</span>
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
            <span class="pill-score" title="Fused RRF Score">
              <span class="pill-label">Score:</span>
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
      vaultEl.textContent = 'Disconnected';
      vaultEl.title = err.message;
    }
  }
}

function renderStatus(status) {
  // Vault Path
  const vaultPathEl = document.getElementById('status-vault-path');
  if (vaultPathEl) {
    vaultPathEl.textContent = status.vault_path || 'Unknown';
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
      lastSyncEl.textContent = 'Never';
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
    });

    const statusBadge = document.getElementById('scoreboard-sync-status');
    if (statusBadge) statusBadge.textContent = 'Syncing...';

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

      showToast(`Vault synced: +${added} added, ~${modified} updated, ${vectors} vectors embedded`, 'success');
      await fetchStatus();
      await loadLogsList();
    } catch (err) {
      console.error('Vault sync error:', err);
      showToast(`Sync failed: ${err.message}`, 'error');
    } finally {
      allBtns.forEach(b => {
        b.classList.remove('syncing');
        b.disabled = false;
      });
      if (statusBadge) statusBadge.textContent = 'Ready';
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

function initGraphExplorer() {
  const canvas = document.getElementById('graph-canvas');
  if (!canvas) return;

  // Window resize observer
  const container = document.getElementById('graph-viewport-container');
  if (container && window.ResizeObserver) {
    const ro = new ResizeObserver(() => {
      resizeGraphCanvas();
      reheatSimulation(0.3);
    });
    ro.observe(container);
  } else {
    window.addEventListener('resize', () => {
      resizeGraphCanvas();
      reheatSimulation(0.3);
    });
  }

  // Hook tab activation
  window.onGraphTabActivated = () => {
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
  counterEl.textContent = `${visibleNodes.length} nodes \u00B7 ${visibleLinks.length} links`;
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
  graphState.simulation.alpha = Math.max(graphState.simulation.alpha, alpha);
  startAnimationLoop();
}

function startAnimationLoop() {
  if (graphState.animFrameId) return;

  function loop() {
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
    hierarchyEl.textContent = node.tierConfig.name;
    hierarchyEl.className = `badge-tier ${node.tierConfig.badgeClass}`;
  }

  // Tags
  const tagsEl = document.getElementById('drawer-tags');
  if (tagsEl) {
    if (node.tags && node.tags.length > 0) {
      tagsEl.innerHTML = node.tags.map(t => `<span class="drawer-tag-pill">#${escapeHtml(t)}</span>`).join('');
    } else {
      tagsEl.innerHTML = '<span class="drawer-empty-hint">No tags</span>';
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
      outLinksEl.innerHTML = '<li class="drawer-empty-hint">No outgoing WikiLinks</li>';
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
      backlinksEl.innerHTML = '<li class="drawer-empty-hint">No inbound backlinks</li>';
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
        copyToClipboard(selectedLogId, copyIdBtn, 'Copy Node ID');
      }
    });
  }

  const copyLogBtn = document.getElementById('btn-copy-raw-log');
  if (copyLogBtn) {
    copyLogBtn.addEventListener('click', () => {
      if (currentLogData && currentLogData.content) {
        copyToClipboard(currentLogData.content, copyLogBtn, 'Copy Raw Log');
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
      badgeEl.textContent = `${cachedLogs.length} logs`;
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
        <p class="placeholder-text" style="color: var(--danger);">Failed to load logs: ${escapeHtml(err.message)}</p>
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
          <p class="placeholder-text">No offloaded logs found in .scratch/refs/</p>
          <span class="placeholder-subtext mono">Run long commands or k0maru offload to capture logs</span>
        </div>
      `;
    } else {
      container.innerHTML = `
        <div class="placeholder-state logs-empty-state">
          <p class="placeholder-text">No logs matching "${escapeHtml(filter)}"</p>
          <span class="placeholder-subtext mono">Try adjusting search query</span>
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
          <span class="badge badge-muted mono log-item-lines">${log.line_count} lines</span>
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
      detailLinesEl.textContent = `${meta.line_count} lines`;
      detailLinesEl.style.display = 'inline-flex';
    }
    if (detailTimeEl) {
      detailTimeEl.textContent = meta.created_at;
    }
    if (rawLineBadge) {
      rawLineBadge.textContent = `${meta.line_count} lines`;
    }
  }

  // Placeholder while loading
  if (mermaidViewer) {
    mermaidViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text">Loading trace diagram...</p></div>`;
  }
  if (rawViewer) {
    rawViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text">Loading raw log slice...</p></div>`;
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
      mermaidViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" style="color: var(--danger);">Failed to load diagram: ${escapeHtml(err.message)}</p></div>`;
    }
    if (rawViewer) {
      rawViewer.innerHTML = `<div class="placeholder-state"><p class="placeholder-text" style="color: var(--danger);">Failed to load log content: ${escapeHtml(err.message)}</p></div>`;
    }
  }
}

function renderRawLogSlice(rawContent, container) {
  if (!rawContent || !container) {
    container.innerHTML = '<div class="placeholder-state"><p class="placeholder-text">Log is empty.</p></div>';
    return;
  }

  const lines = rawContent.split(/\r?\n/);
  const totalLines = lines.length;

  const rawLineBadge = document.getElementById('raw-log-line-badge');
  if (rawLineBadge) {
    rawLineBadge.textContent = `${totalLines} lines`;
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

function copyToClipboard(text, button, originalLabel = 'Copy') {
  if (!text) return;

  function setCopied() {
    button.classList.add('copied');
    const textSpan = button.querySelector('.btn-copy-text');
    if (textSpan) textSpan.textContent = 'Copied!';

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

    showToast('Copied to clipboard!', 'success');

    setTimeout(() => {
      button.classList.remove('copied');
      if (textSpan) textSpan.textContent = originalLabel;
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
        showToast('Failed to copy to clipboard', 'error');
      }
    } catch (e) {
      console.error('Fallback copy error:', e);
      showToast('Clipboard copy not supported', 'error');
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
    cardVaultPath.textContent = status.vault_path || 'Unknown';
    cardVaultPath.title = status.vault_path || '';
  }

  const cardLastSync = document.getElementById('scoreboard-last-sync');
  if (cardLastSync) {
    if (status.last_sync_time) {
      const d = new Date(status.last_sync_time * 1000);
      cardLastSync.textContent = d.toLocaleString();
    } else {
      cardLastSync.textContent = 'Never';
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
