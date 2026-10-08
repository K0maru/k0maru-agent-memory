//! Zero-daemon FastMCP stdio server implementing JSON-RPC 2.0.
//!
//! Exposes five lean agent memory tools:
//! 1. `get_project_loadout`: Generates sub-300-token context pack
//! 2. `recall_memory`: Fast FTS5 BM25 search over notes and memories
//! 3. `offload_context`: Truncates long command logs into Mermaid state diagram and disk reference
//! 4. `inspect_log_node`: Retrieves full raw log by node ID
//! 5. `flush_session`: Crystallizes memory notes into the target vault according to conventions

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::adapters::{GenericWikiAdapter, ObsidianAdapter};
use crate::core::traits::CacheStorage;
use crate::loadout::LoadoutBuilder;
use crate::offload::{inspect_node, OffloadEngine};
use crate::scanner::IncrementalScanner;
use crate::storage::hybrid::{HybridSearchEngine, SearchMode};
use crate::storage::SqliteStorage;
use crate::vector::{EmbeddingEngine, MockEmbeddingEngine};

/// FastMCP stdio server running on top of JSON-RPC 2.0.
pub struct McpServer {
    vault_path: PathBuf,
    refs_dir: PathBuf,
    storage: Option<SqliteStorage>,
    embedder: Option<Arc<dyn EmbeddingEngine>>,
}

impl McpServer {
    /// Creates a new `McpServer` targeting the given vault path.
    pub fn new(vault_path: impl Into<PathBuf>) -> Self {
        let vault_path = vault_path.into();
        let refs_dir = if vault_path.join(".k0maru").join("refs").exists() {
            vault_path.join(".k0maru").join("refs")
        } else {
            vault_path.join(".scratch").join("refs")
        };

        Self {
            vault_path,
            refs_dir,
            storage: None,
            embedder: Some(Arc::new(MockEmbeddingEngine::new(384))),
        }
    }

    /// Sets custom refs directory for offloaded logs.
    pub fn with_refs_dir(mut self, refs_dir: impl Into<PathBuf>) -> Self {
        self.refs_dir = refs_dir.into();
        self
    }

    /// Pre-injects a configured [`SqliteStorage`].
    pub fn with_storage(mut self, storage: SqliteStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Sets custom embedding engine for hybrid recall.
    pub fn with_embedder(mut self, embedder: Arc<dyn EmbeddingEngine>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Returns the target vault root directory.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Returns the active refs directory.
    pub fn refs_dir(&self) -> &Path {
        &self.refs_dir
    }

    fn is_obsidian(&self) -> bool {
        self.vault_path.join("10_Projects").is_dir()
            || self.vault_path.join(".obsidian").exists()
            || self.vault_path.join("20_Cards").is_dir()
    }

    fn get_or_init_storage(&mut self) -> Result<&mut SqliteStorage, Box<dyn std::error::Error>> {
        if self.storage.is_none() {
            let cache_dir = self.vault_path.join(".k0maru");
            let cache_path = cache_dir.join("cache.sqlite");
            let storage = match SqliteStorage::open(&cache_path) {
                Ok(s) => s,
                Err(_) => SqliteStorage::in_memory()?,
            };
            self.storage = Some(storage);
        }
        Ok(self.storage.as_mut().unwrap())
    }

    fn sync_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let is_obsidian = self.is_obsidian();
        let vault_path = self.vault_path.clone();
        let embedder = self.embedder.clone();
        let storage = self.get_or_init_storage()?;

        let has_vectors = storage.has_vectors().unwrap_or(false);

        if is_obsidian {
            let adapter = ObsidianAdapter::new(&vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, storage);
            let _ = scanner.sync(false)?;
            if has_vectors {
                if let Some(ref emb) = embedder {
                    let _ = crate::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &**emb);
                }
            }
        } else {
            let adapter = GenericWikiAdapter::new(&vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, storage);
            let _ = scanner.sync(false)?;
            if has_vectors {
                if let Some(ref emb) = embedder {
                    let _ = crate::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &**emb);
                }
            }
        }
        Ok(())
    }

    fn call_get_project_loadout(&mut self, arguments: &Value) -> Result<String, String> {
        let project_name = arguments
            .get("project_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'project_name'".to_string())?;

        if let Err(e) = self.sync_cache() {
            return Err(format!("Cache sync failed: {}", e));
        }

        let is_obsidian = self.is_obsidian();
        let vault_path = self.vault_path.clone();
        let storage = self
            .get_or_init_storage()
            .map_err(|e| format!("Storage initialization failed: {}", e))?;

        let loadout_opt = if is_obsidian {
            let adapter = ObsidianAdapter::new(&vault_path);
            let builder = LoadoutBuilder::new(&adapter, storage);
            builder.build(project_name).map_err(|e| e.to_string())?
        } else {
            let adapter = GenericWikiAdapter::new(&vault_path);
            let builder = LoadoutBuilder::new(&adapter, storage);
            builder.build(project_name).map_err(|e| e.to_string())?
        };

        match loadout_opt {
            Some(loadout) => Ok(loadout.markdown),
            None => Err(format!(
                "Project matching '{}' not found in vault.",
                project_name
            )),
        }
    }

    fn call_recall_memory(&mut self, arguments: &Value) -> Result<String, String> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'query'".to_string())?;

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);

        if let Err(e) = self.sync_cache() {
            return Err(format!("Cache sync failed: {}", e));
        }

        let embedder = self.embedder.clone();
        let storage = self
            .get_or_init_storage()
            .map_err(|e| format!("Storage initialization failed: {}", e))?;

        let has_vectors = storage.has_vectors().unwrap_or(false);

        if has_vectors {
            let engine = HybridSearchEngine::new(storage, embedder);
            let results = engine
                .search(query, SearchMode::Hybrid, limit)
                .map_err(|e| format!("Hybrid search failed: {}", e))?;

            if results.is_empty() {
                return Ok(format!("No memories found matching query: '{}'", query));
            }

            let mut formatted = Vec::new();
            for res in &results {
                let mut item = format!("## {} ({})\n", res.title, res.path);
                item.push_str(&format!("Score: {:.4}\n", res.score));

                let backlinks = retrieve_backlinks(storage, &res.title, &res.path);
                if !backlinks.is_empty() {
                    let bl_strs: Vec<String> =
                        backlinks.iter().map(|s| format!("[[{}]]", s)).collect();
                    item.push_str(&format!("Backlinks: {}\n", bl_strs.join(", ")));
                }

                if let Ok(Some(doc)) = storage.get_document(Path::new(&res.path)) {
                    if !doc.tags.is_empty() {
                        item.push_str(&format!("Tags: #{}\n", doc.tags.join(" #")));
                    }
                    item.push_str(&format!("Hierarchy: {}\n", doc.hierarchy));
                }

                item.push_str(&format!("\n{}", res.snippet));
                formatted.push(item);
            }

            Ok(formatted.join("\n\n---\n\n"))
        } else {
            let docs = storage
                .search_fts(query, limit)
                .map_err(|e| format!("FTS search failed: {}", e))?;

            if docs.is_empty() {
                return Ok(format!("No memories found matching query: '{}'", query));
            }

            let mut formatted = Vec::new();
            for (i, doc) in docs.iter().enumerate() {
                let rank = i + 1;
                let score = 1.0 / (60.0 + rank as f32);
                let path_str = doc.path.to_string_lossy();
                let mut item = format!("## {} ({})\n", doc.title, doc.path.display());
                item.push_str(&format!("Score: {:.4}\n", score));

                let backlinks = retrieve_backlinks(storage, &doc.title, &path_str);
                if !backlinks.is_empty() {
                    let bl_strs: Vec<String> =
                        backlinks.iter().map(|s| format!("[[{}]]", s)).collect();
                    item.push_str(&format!("Backlinks: {}\n", bl_strs.join(", ")));
                }

                if !doc.tags.is_empty() {
                    item.push_str(&format!("Tags: #{}\n", doc.tags.join(" #")));
                }
                item.push_str(&format!("Hierarchy: {}\n\n", doc.hierarchy));
                item.push_str(&doc.body);
                formatted.push(item);
            }

            Ok(formatted.join("\n\n---\n\n"))
        }
    }

    fn call_offload_context(&self, arguments: &Value) -> Result<String, String> {
        let raw_text = arguments
            .get("raw_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'raw_text'".to_string())?;

        let task_name = arguments
            .get("task_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let mut engine = OffloadEngine::new(&self.refs_dir).with_threshold(50);
        if let Some(task) = task_name {
            engine = engine.with_task_id(task);
        }

        let result = engine
            .offload_text(raw_text)
            .map_err(|e| format!("Offload failed: {}", e))?;

        if result.truncated {
            Ok(format!(
                "{}\n\n{}",
                result.mermaid_graph, result.summary_text
            ))
        } else {
            Ok(result.summary_text)
        }
    }

    fn call_inspect_log_node(&self, arguments: &Value) -> Result<String, String> {
        let node_id = arguments
            .get("node_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'node_id'".to_string())?;

        inspect_node(&self.refs_dir, node_id)
            .map_err(|e| format!("Inspection failed for node '{}': {}", node_id, e))
    }

    fn call_flush_session(&mut self, arguments: &Value) -> Result<String, String> {
        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'title'".to_string())?;

        let content = arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'content'".to_string())?;

        let summary = arguments
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let category = arguments
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("log")
            .to_string();

        let tags: Vec<String> = arguments
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let related_notes: Vec<String> = arguments
            .get("related_notes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let engine = crate::convention::FlushEngine::new(&self.vault_path)
            .map_err(|e| format!("FlushEngine initialization failed: {}", e))?;

        let request = crate::convention::FlushRequest {
            title: title.to_string(),
            summary,
            content: content.to_string(),
            category,
            tags,
            related_notes: related_notes.clone(),
            dry_run: false,
        };

        let result = engine
            .flush(request)
            .map_err(|e| format!("Flush failed: {}", e))?;

        let _ = self.sync_cache();

        let links_str = if related_notes.is_empty() {
            "None".to_string()
        } else {
            related_notes
                .iter()
                .map(|r| {
                    let t = r.trim();
                    if t.starts_with("[[") && t.ends_with("]]") {
                        t.to_string()
                    } else {
                        format!("[[{}]]", t)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        if !result.created {
            Ok(format!(
                "Note already up to date with identical content.\nPath: {}\nCategory: {}",
                result.relative_path.display(),
                result.category
            ))
        } else {
            Ok(format!(
                "✓ Crystallized note into vault.\nRelative Path: {}\nCategory: {}\nWikiLinks: {}",
                result.relative_path.display(),
                result.category,
                links_str
            ))
        }
    }

    fn call_distill_session_skill(&mut self, arguments: &Value) -> Result<String, String> {
        let raw_trace = arguments.get("raw_trace").and_then(|v| v.as_str());
        let node_id = arguments.get("node_id").and_then(|v| v.as_str());

        if raw_trace.is_none() && node_id.is_none() {
            return Err("Either 'raw_trace' or 'node_id' must be provided".to_string());
        }

        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let context_hint = arguments
            .get("context_hint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let category = arguments
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("skill")
            .to_string();

        let dry_run = arguments
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let tags: Vec<String> = arguments
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let related_notes: Vec<String> = arguments
            .get("related_notes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let engine = crate::distill::DistillEngine::new(&self.vault_path, &self.refs_dir);
        let opts = crate::distill::DistillOptions {
            title,
            context_hint,
            category: Some(category),
            tags,
            related_notes: related_notes.clone(),
            dry_run,
        };

        let result = if let Some(trace) = raw_trace {
            engine
                .distill_text(trace, opts)
                .map_err(|e| format!("Distill failed: {}", e))?
        } else if let Some(nid) = node_id {
            engine
                .distill_node(nid, opts)
                .map_err(|e| format!("Distill failed: {}", e))?
        } else {
            unreachable!();
        };

        if !dry_run {
            let _ = self.sync_cache();
        }

        let links_str = if related_notes.is_empty() {
            "None".to_string()
        } else {
            related_notes
                .iter()
                .map(|r| {
                    let t = r.trim();
                    if t.starts_with("[[") && t.ends_with("]]") {
                        t.to_string()
                    } else {
                        format!("[[{}]]", t)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        if dry_run {
            Ok(format!(
                "[Dry Run] Generated skill preview:\n\nTitle: {}\nCategory: {}\nRelative Path: {}\nWikiLinks: {}\n\n---\n{}",
                result.skill.title,
                result.flush_result.category,
                result.flush_result.relative_path.display(),
                links_str,
                result.preview_markdown
            ))
        } else if !result.flush_result.created {
            Ok(format!(
                "Skill note already up to date with identical content.\nPath: {}\nCategory: {}",
                result.flush_result.relative_path.display(),
                result.flush_result.category
            ))
        } else {
            Ok(format!(
                "✓ Crystallized skill note into vault.\nTitle: {}\nRelative Path: {}\nCategory: {}\nWikiLinks: {}",
                result.skill.title,
                result.flush_result.relative_path.display(),
                result.flush_result.category,
                links_str
            ))
        }
    }

    fn handle_initialize(&self, id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "k0maru",
                    "version": crate::VERSION
                }
            }
        })
    }

    fn handle_tools_list(&self, id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "get_project_loadout",
                        "description": "Generate a compact sub-300-token project context loadout (vision, L3 principles, recent L2 logs)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_name": {
                                    "type": "string",
                                    "description": "Target project name or keyword"
                                }
                            },
                            "required": ["project_name"]
                        }
                    },
                    {
                        "name": "recall_memory",
                        "description": "Search knowledge hub memories and notes using hybrid RRF retrieval (BM25 full-text + vector embeddings + graph links)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query keywords"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of results to return (default: 5)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "offload_context",
                        "description": "Truncate and externalize long text/logs (>50 lines) to disk references, returning a Mermaid diagram and node_id",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "raw_text": {
                                    "type": "string",
                                    "description": "Raw log or output text to be offloaded"
                                },
                                "task_name": {
                                    "type": "string",
                                    "description": "Optional task identifier prefix"
                                }
                            },
                            "required": ["raw_text"]
                        }
                    },
                    {
                        "name": "inspect_log_node",
                        "description": "Inspect and retrieve the full offloaded raw log content by its node ID",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "node_id": {
                                    "type": "string",
                                    "description": "Offloaded log node identifier (e.g. node_1a2b3c4d)"
                                }
                            },
                            "required": ["node_id"]
                        }
                    },
                    {
                        "name": "flush_session",
                        "description": "Crystallize learnings, architectural decisions, or task summaries back into the Markdown knowledge base according to the vault's conventions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "Title of the decision or log note"
                                },
                                "content": {
                                    "type": "string",
                                    "description": "Detailed Markdown content"
                                },
                                "summary": {
                                    "type": "string",
                                    "description": "Brief one-line summary or executive conclusion"
                                },
                                "category": {
                                    "type": "string",
                                    "description": "Category: 'decision', 'log', 'concept', or custom (default: 'log')"
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Associated tags"
                                },
                                "related_notes": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Related existing notes to link via WikiLinks"
                                }
                            },
                            "required": ["title", "content"]
                        }
                    },
                    {
                        "name": "distill_session_skill",
                        "description": "Distill an execution trace, error log, or troubleshooting session into a reusable skill note and crystallize it into the vault.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "raw_trace": {
                                    "type": "string",
                                    "description": "Raw execution log, error stack trace, or debugging terminal output"
                                },
                                "node_id": {
                                    "type": "string",
                                    "description": "Optional offloaded node ID from offload_context (e.g. node_1a2b3c4d)"
                                },
                                "title": {
                                    "type": "string",
                                    "description": "Optional explicit skill title (auto-inferred if omitted)"
                                },
                                "context_hint": {
                                    "type": "string",
                                    "description": "Optional context hint or explanation describing what was happening"
                                },
                                "category": {
                                    "type": "string",
                                    "description": "Target category (defaults to 'skill')"
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional tags array (e.g. ['topic/rust', 'topic/build'])"
                                },
                                "related_notes": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional related notes or WikiLinks array"
                                },
                                "dry_run": {
                                    "type": "boolean",
                                    "description": "If true, simulates distillation and returns preview without writing to vault"
                                }
                            }
                        }
                    }
                ]
            }
        })
    }

    /// Dispatches and executes an MCP tool call given request ID and tool parameters.
    pub fn handle_tool_call(&mut self, id: Value, params: &Value) -> Value {
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let empty_args = Value::Object(serde_json::Map::new());
        let arguments = params.get("arguments").unwrap_or(&empty_args);

        let (content_text, is_error) = match tool_name {
            "get_project_loadout" => match self.call_get_project_loadout(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "recall_memory" => match self.call_recall_memory(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "offload_context" => match self.call_offload_context(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "inspect_log_node" => match self.call_inspect_log_node(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "flush_session" => match self.call_flush_session(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "distill_session_skill" => match self.call_distill_session_skill(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            unknown => (format!("Unknown tool: '{}'", unknown), true),
        };

        let mut result_obj = serde_json::Map::new();
        result_obj.insert(
            "content".to_string(),
            json!([
                {
                    "type": "text",
                    "text": content_text
                }
            ]),
        );
        if is_error {
            result_obj.insert("isError".to_string(), Value::Bool(true));
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": Value::Object(result_obj)
        })
    }

    /// Alias for [`Self::handle_tool_call`].
    pub fn handle_call_tool(&mut self, id: Value, params: &Value) -> Value {
        self.handle_tool_call(id, params)
    }

    /// Handles a single incoming JSON-RPC 2.0 request or notification string.
    ///
    /// Returns `Some(response_json)` for requests, or `None` for notifications (silent acknowledgment).
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let obj = match parsed.as_object() {
            Some(o) => o,
            None => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let id = obj.get("id").cloned();
        let is_notification = !obj.contains_key("id");

        let method = match obj.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                if is_notification {
                    return None;
                }
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap_or(Value::Null),
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let params = obj.get("params").unwrap_or(&Value::Null);

        match method {
            "initialize" => {
                let resp = self.handle_initialize(id.unwrap_or(Value::Null));
                Some(resp.to_string())
            }
            "notifications/initialized" | "initialized" => {
                // Silent acknowledgment according to MCP specification
                None
            }
            "tools/list" => {
                let resp = self.handle_tools_list(id.unwrap_or(Value::Null));
                Some(resp.to_string())
            }
            "tools/call" => {
                let resp = self.handle_tool_call(id.unwrap_or(Value::Null), params);
                Some(resp.to_string())
            }
            "ping" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap_or(Value::Null),
                    "result": {}
                });
                Some(resp.to_string())
            }
            _ => {
                if is_notification {
                    None
                } else {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id.unwrap_or(Value::Null),
                        "error": {
                            "code": -32601,
                            "message": "Method not found"
                        }
                    });
                    Some(err_resp.to_string())
                }
            }
        }
    }

    /// Runs the stdio JSON-RPC loop reading lines from `reader` and writing responses to `writer`.
    pub fn run_stdio<R: BufRead, W: Write>(
        &mut self,
        mut reader: R,
        mut writer: W,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Some(resp) = self.handle_line(trimmed) {
                            if let Err(e) = writer.write_all(resp.as_bytes()) {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                            if let Err(e) = writer.write_all(b"\n") {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                            if let Err(e) = writer.flush() {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        break;
                    }
                    return Err(Box::new(e));
                }
            }
        }
        Ok(())
    }
}

/// Helper function to retrieve all backlinks matching document title, file stem, or whitespace-normalized variants.
fn retrieve_backlinks(storage: &SqliteStorage, title: &str, path_str: &str) -> Vec<String> {
    let mut all = Vec::new();
    if let Ok(bls) = storage.get_backlinks(title) {
        for b in bls {
            let s = b.to_string_lossy().to_string();
            if !all.contains(&s) {
                all.push(s);
            }
        }
    }
    if let Some(stem) = Path::new(path_str).file_stem().and_then(|s| s.to_str()) {
        if stem != title {
            if let Ok(bls) = storage.get_backlinks(stem) {
                for b in bls {
                    let s = b.to_string_lossy().to_string();
                    if !all.contains(&s) {
                        all.push(s);
                    }
                }
            }
        }
    }
    let under = title.replace(' ', "_");
    if under != title {
        if let Ok(bls) = storage.get_backlinks(&under) {
            for b in bls {
                let s = b.to_string_lossy().to_string();
                if !all.contains(&s) {
                    all.push(s);
                }
            }
        }
    }
    let space = title.replace('_', " ");
    if space != title {
        if let Ok(bls) = storage.get_backlinks(&space) {
            for b in bls {
                let s = b.to_string_lossy().to_string();
                if !all.contains(&s) {
                    all.push(s);
                }
            }
        }
    }
    all.sort();
    all
}
