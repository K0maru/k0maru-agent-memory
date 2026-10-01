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
  initKeyboardShortcuts();
  fetchStatus();
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

    // Escape key blurs search input
    if (e.key === 'Escape') {
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
  const scoreboardDocEl = document.getElementById('scoreboard-doc-count');
  if (scoreboardDocEl) {
    scoreboardDocEl.textContent = Number(status.total_documents).toLocaleString();
  }

  // Vector Count
  const vecCountEl = document.getElementById('status-vector-count');
  if (vecCountEl) {
    vecCountEl.textContent = Number(status.total_vectors).toLocaleString();
  }
  const scoreboardVecEl = document.getElementById('scoreboard-vec-count');
  if (scoreboardVecEl) {
    scoreboardVecEl.textContent = Number(status.total_vectors).toLocaleString();
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

  // Cache Size
  const cacheSizeEl = document.getElementById('scoreboard-cache-size');
  if (cacheSizeEl) {
    const kb = (status.cache_size_bytes / 1024).toFixed(1);
    cacheSizeEl.textContent = `${kb} KB`;
  }

  // Total Logs
  const logCountEl = document.getElementById('scoreboard-log-count');
  if (logCountEl) {
    logCountEl.textContent = Number(status.total_logs).toLocaleString();
  }
}

/**
 * Vault Synchronization Trigger
 */
function initSync() {
  const syncBtn = document.getElementById('btn-sync');
  if (!syncBtn) return;

  syncBtn.addEventListener('click', async () => {
    syncBtn.classList.add('syncing');
    syncBtn.disabled = true;

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
    } catch (err) {
      console.error('Vault sync error:', err);
      showToast(`Sync failed: ${err.message}`, 'error');
    } finally {
      syncBtn.classList.remove('syncing');
      syncBtn.disabled = false;
    }
  });
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
