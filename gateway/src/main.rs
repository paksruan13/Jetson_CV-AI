use async_graphql::http::{ALL_WEBSOCKET_PROTOCOLS, GraphQLPlaygroundConfig, playground_source};
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{info, Level};

mod schema;
mod ar;

use schema::{build_schema, MerlinSchema, ARScene};
use ar::JetsonARClient;

// Appstate shared across routes
#[derive(Clone)]
struct AppState {
    schema: MerlinSchema,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    // Load Config from env
    let jetson_url = std::env::var("JETSON_AR_BRIDGE_URL").expect("Jetson AR Bridge URL not set in .env");
    let gateway_host = std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let gateway_port = std::env::var("GATEWAY_PORT").unwrap_or_else(|_| "4000".to_string());
    let buffer_size = std::env::var("AR_FRAME_BUFFER_SIZE")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<usize>()
        .unwrap_or(100);
    info!("Configs loaded:");
    info!("  Jetson URL: {}", jetson_url);
    info!("  Gateway: {}:{}", gateway_host, gateway_port);
    info!("  Frame buffer: {} frames", buffer_size);

    // Init Logging
    tracing_subscriber::fmt() // formatter (INFO & below Type)
        .with_max_level(Level::INFO)
        .init();
    info!("Starting MERLIN GraphQL Gateway...");
    

    // Create broadcast channel for AR streaming (100 frame buffer)
    let (ar_stream_tx, _rx) = broadcast::channel::<ARScene>(buffer_size);
    // Build Schema
    let schema = build_schema(ar_stream_tx.clone());
    // Create app state
    let state = AppState { schema };

    // Spawn background task to simulate TEMP AR Frames
    let ar_client = JetsonARClient::new(jetson_url, ar_stream_tx.clone());
    tokio::spawn(async move {
        ar_client.start().await;
    });

    // Build Axum Router
    let app = Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/ws", get(graphql_subscription))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive()) // no permission for prod code
        .with_state(state);

    let addr = format!("{}:{}", gateway_host, gateway_port); // reverse prox for prod
    info!("GraphQL Gateway listening on {}", addr);
    info!("GraphQL Playground: http://{}/", addr);
    info!("WebSocket Subscription: ws://{}/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
/// **END OF MAIN** 

// GraphQL Playground UI
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

// GraphQL query/mutation handler
async fn graphql_handler(
    State(state): State<AppState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

// GraphQL subscription (Websocket) handler
async fn graphql_subscription(
    State(state): State<AppState>,
    protocol: GraphQLProtocol,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl IntoResponse {
    ws.protocols(ALL_WEBSOCKET_PROTOCOLS).on_upgrade(move |stream| {
        GraphQLWebSocket::new(stream, state.schema, protocol)
            .serve()
    })
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    "MERLIN GraphQL Gateway Ok"
}
