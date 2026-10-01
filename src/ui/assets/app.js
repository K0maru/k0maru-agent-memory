/**
 * K0maru Agent Memory Hub — Developer Dark Cockpit Frontend
 * Vanilla ES6 Application Logic
 */

document.addEventListener('DOMContentLoaded', () => {
  initTabs();
  initSync();
  initSearchModeToggles();
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
 * Search Mode Toggles Stub
 */
function initSearchModeToggles() {
  const modeButtons = document.querySelectorAll('.btn-mode');
  modeButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      modeButtons.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
    });
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
