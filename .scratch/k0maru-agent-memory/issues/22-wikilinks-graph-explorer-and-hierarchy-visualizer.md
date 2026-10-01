# 22 — WikiLinks Graph Explorer and Hierarchy Visualizer

**What to build:**
Implement Cockpit Panel 2: Interactive 2D Force-Directed WikiLinks Knowledge Graph.
Visualizes the entire note network, semantic hierarchy tiers (L0–L3), bidirectional links, and orphan notes using an interactive canvas.

**Blocked by:** 20 (Dashboard Frontend Asset Pipeline and Static Embedding)

**Status:** ready-for-agent

- [ ] Render interactive 2D force-directed canvas graph using D3-force (or lightweight Cytoscape/HTML5 Canvas).
- [ ] Color-code nodes by semantic hierarchy:
  - 🟢 `Project` (#22C55E Emerald)
  - 🟡 `L3 Evergreen Card` (#F59E0B Amber)
  - 🔵 `L2 Log` (#3B82F6 Blue)
  - 🟣 `L1 Resource` (#A855F7 Purple)
  - ⚪ `L0 Daily` (#64748B Slate)
- [ ] Node interaction:
  - Hover: highlight connected 1-hop neighbor edges and dim unrelated nodes.
  - Click: open side-drawer with note details (title, path, tags, out-links, backlinks).
  - Search input: instant focus/pan to matching note node.
- [ ] Filter controls:
  - Toggle visibility by hierarchy level (e.g. hide L0 Daily, focus on L3 Cards & Projects).
  - "Highlight Orphans" toggle to identify disconnected notes with 0 links and 0 backlinks.
- [ ] Responsive canvas resizing and zoom/pan controls.
