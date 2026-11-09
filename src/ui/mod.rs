//! Developer dashboard embedded web server and REST API.

pub mod routes;
pub mod server;

pub use routes::{
    create_router, is_forbidden_system_vault_path, static_handler, AppState, BootstrapApiResponse,
    BootstrapRequest, ClientsResponse, ConfigResponse, DashboardAssets, GraphEdge, GraphNode,
    GraphResponse, GuardCheckPayload, GuardCheckRequest, GuardCheckResponse, GuardRulesResponse,
    InstallClientsRequest, LogDetailResponse, LogItemResponse, NoteParams, NoteQueryRequest,
    NoteResponse, SearchParams, StatusResponse, SyncResponse, TimelineParams, TimelineQueryRequest,
    TimelineResponse, UpdateConfigRequest, VaultRuntime,
};
pub use server::run_server;
