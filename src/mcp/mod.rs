//! Model Context Protocol (MCP) server integration.
//!
//! Provides a zero-daemon, stdio-based JSON-RPC 2.0 FastMCP standard server
//! enabling AI agents (Claude Code, Cursor, Windsurf) to natively invoke K0maru tools.

pub mod server;

pub use server::McpServer;
