use async_graphql::*;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

// Basic AR scene for now
#[derive(Clone, Debug, Serialize, Deserialize, SimpleObject)]
pub struct ARScene {
    pub timestamp: u64,
    pub frame_id: u32,
}

// GraphQL context, holds broadcast channel for AR streaming
pub struct Context {
    pub ar_stream_tx: broadcast::Sender<ARScene>,
}

// Query root + Health Check
pub struct Query;
#[Object]
impl Query {
    async fn health(&self) -> String {
        "MERLIN GraphQL Gateway OK".to_string()
    }
}

// Subscription root - AR Streaming
pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn ar_stream<'ctx>(
        &self,
        ctx: &async_graphql::Context<'ctx>,
    ) -> impl futures_util::Stream<Item = ARScene> { // AR Streaming subscription
        let context = ctx.data::<Context>().unwrap(); // Get context
        let mut rx = context.ar_stream_tx.subscribe(); // Subscribe to AR stream

        async_stream::stream! { 
            while let Ok(scene) = rx.recv().await { // Loop and return scene
                yield scene;
            }
        }
    }
}

// GraphQL Schema
pub type MerlinSchema = Schema<Query, EmptyMutation, Subscription>;

// Build schema with AR stream channel:
pub fn build_schema(ar_stream_tx: broadcast::Sender<ARScene>) -> MerlinSchema {
    Schema::build(Query, EmptyMutation, Subscription)
        .data(Context { ar_stream_tx })
        .finish()
}

