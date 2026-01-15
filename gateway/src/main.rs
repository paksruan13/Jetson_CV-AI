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
use schema::{build_schema, MerlinSchema, ARScene};

// Appstate shared across routes
#[derive(Clone)]
struct AppState {
    schema: MerlinSchema,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init Logging
    tracing_subscriber::fmt() // formatter (INFO & below Type)
        .with_max_level(Level::INFO)
        .init();
    info!("Starting MERLIN GraphQL Gateway...");
    

    // Create broadcast channel for AR streaming (100 frame buffer)
    let (ar_stream_tx, _rx) = broadcast::channel::<ARScene>(100);
    // Build Schema
    let schema = build_schema(ar_stream_tx.clone());
    // Create app state
    let state = AppState { schema };

    // Spawn background task to simulate TEMP AR Frames
    let ar_tx_clone = ar_stream_tx.clone();
    tokio::spawn(async move {
        simulate_ar_frames(ar_tx_clone).await;
    });

    // Build Axum Router
    let app = Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/ws", get(graphql_subscription))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive()) // no permission for prod code
        .with_state(state);

    let addr = "0.0.0.0:4000"; // reverse prox for prod
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

// Simulate AR frames for testing (will replace with Jetson connection in Phase 3)
async fn simulate_ar_frames(ar_tx: broadcast::Sender<ARScene>) {
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::time::{interval, Duration};

    info!("Starting AR frame simulator (30 FPS)");

    let mut frame_id = 0u32;
    let mut timer = interval(Duration::from_millis(33)); // ~30 FPS

    loop {
        timer.tick().await;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let scene = ARScene {
            timestamp,
            frame_id,
        };

        // Broadcast to all subscribers
        let _ = ar_tx.send(scene);

        frame_id = frame_id.wrapping_add(1);

        // Log every 30 frames (1 second)
        if frame_id % 30 == 0 {
            info!("Simulated {} AR frames", frame_id);
        }
    }
}