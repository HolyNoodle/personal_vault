mod domain;
mod application;
mod infrastructure;
mod adapters;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Secure Sandbox Server starting...");
    
    // TODO: Bootstrap application
    
    Ok(())
}
