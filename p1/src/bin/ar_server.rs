use p1::ar::ARBridgeServer;

#[tokio::main]
async fn main() -> Result<(), Box <dyn std::error::Error>> {
    println!("Merlin AR Brifge Server");
    println!();

    let server = ARBridgeServer::new("0.0.0.0:8765");

    // Run server
    server.run().await?;
    Ok(())
}