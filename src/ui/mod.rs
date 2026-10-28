//! Developer dashboard embedded web server and REST API.

pub mod routes;
pub mod server;

pub use routes::{
    create_router, static_handler, AppState, DashboardAssets, GraphEdge, GraphNode, GraphResponse,
    LogDetailResponse, LogItemResponse, SearchParams, StatusResponse, SyncResponse,
};
pub use server::run_server;
