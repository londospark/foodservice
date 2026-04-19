use axum::{Router, debug_handler, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(root).post(add_item));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    println!("Server is running on http://localhost:3001");
}

#[debug_handler]
async fn add_item() -> &'static str {
    "Item added to inventory!"
}

#[debug_handler]
async fn root() -> &'static str {
    "Welcome to the Inventory API!"
}
