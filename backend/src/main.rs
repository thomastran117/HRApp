use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/users", post(create_user));

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("🚀 Server running at http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Rust API is running"
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn create_user(
    Json(payload): Json<CreateUserRequest>,
) -> Json<UserResponse> {
    Json(UserResponse {
        id: 1,
        email: payload.email,
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    email: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: i32,
    email: String,
}
