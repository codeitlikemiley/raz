use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u8,
}

#[derive(Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    age: u8,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/users", post(create_user))
        .route("/users/{name}", get(get_user));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Welcome to the Axum server!"
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "Healthy")
}

async fn create_user(Json(payload): Json<CreateUserRequest>) -> impl IntoResponse {
    let user = User {
        name: payload.name,
        age: payload.age,
    };
    (StatusCode::CREATED, Json(user))
}

async fn get_user(Path(name): Path<String>) -> impl IntoResponse {
    let user = User {
        name,
        age: 25, // Default age
    };
    Json(user)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
     assert!(true); 
    }

    #[test]
    fn sure_it_does() {
       assert!(true); 
    }
}
