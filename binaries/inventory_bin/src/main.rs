use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
    routing::{get, post},
};
use inventory::{
    dto::{AddFoodItem, FoodItem},
    protocol::{INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_add_food_item, encode_food_item},
    traits::InventoryService,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    inventory_service: Arc<dyn InventoryService>,
}

fn app(inventory_service: Arc<dyn InventoryService>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/food", post(add_item))
        .with_state(AppState { inventory_service })
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app(Arc::new(UnimplementedInventoryService)))
        .await
        .unwrap();
}

async fn add_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    match headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        Some(INVENTORY_V1_BINCODE_MEDIA_TYPE) => {}
        _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response(),
    }

    let command: AddFoodItem = match decode_add_food_item(&body) {
        Ok(command) => command,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let inventory = &state.inventory_service;
    let response = inventory
        .add_food_item(&command)
        .await
        .expect("inventory service should not fail in this example");

    let response = encode_food_item(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = response.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        INVENTORY_V1_BINCODE_MEDIA_TYPE.parse().unwrap(),
    );
    response
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn root() -> &'static str {
    "Welcome to the Inventory API!"
}

struct UnimplementedInventoryService;

#[async_trait::async_trait]
impl InventoryService for UnimplementedInventoryService {
    async fn add_food_item(&self, _item: &AddFoodItem) -> anyhow::Result<FoodItem> {
        anyhow::bail!("inventory service is not wired yet")
    }
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use inventory::{
        dto::{AddFoodItem, FoodItem},
        protocol::{INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_food_item, encode_add_food_item},
        traits::InventoryService,
    };
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn post_food_uses_the_binary_protocol_and_delegates_to_inventory_service() {
        let service = Arc::new(RecordingInventoryService::successful(FoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            name: "Milk".to_string(),
            quantity: 2,
        }));
        let request_body = encode_add_food_item(&AddFoodItem {
            name: "Milk".to_string(),
            quantity: 2,
        })
        .expect("test command should encode");

        let response = app(service.clone())
            .oneshot(
                Request::post("/food")
                    .header(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "inventory_bin should expose a real POST /food endpoint rather than a stub"
        );
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            INVENTORY_V1_BINCODE_MEDIA_TYPE,
            "inventory_bin should return the same explicit binary media type it accepts"
        );

        let response_body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let returned =
            decode_food_item(&response_body).expect("service responses should decode via bincode");

        assert_eq!(
            service.recorded_calls(),
            vec![AddFoodItem {
                name: "Milk".to_string(),
                quantity: 2,
            }],
            "the HTTP adapter should hand the decoded command to the inventory service"
        );
        assert_eq!(
            returned,
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                name: "Milk".to_string(),
                quantity: 2,
            },
            "the HTTP adapter should return the inventory service result, encoded as bincode"
        );
    }

    #[tokio::test]
    async fn post_food_rejects_non_binary_payloads() {
        let response = app(Arc::new(RecordingInventoryService::successful(FoodItem {
            id: Uuid::now_v7(),
            name: "Milk".to_string(),
            quantity: 2,
        })))
        .oneshot(
            Request::post("/food")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"Milk","quantity":2}"#))
                .unwrap(),
        )
        .await
        .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "the HTTP edge should keep the binary contract explicit instead of silently accepting JSON"
        );
    }

    #[tokio::test]
    async fn post_food_returns_bad_request_for_invalid_bincode() {
        let response = app(Arc::new(RecordingInventoryService::successful(FoodItem {
            id: Uuid::now_v7(),
            name: "Milk".to_string(),
            quantity: 2,
        })))
        .oneshot(
            Request::post("/food")
                .header(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)
                .body(Body::from(vec![0xde, 0xad, 0xbe, 0xef]))
                .unwrap(),
        )
        .await
        .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "invalid binary payloads should be rejected before reaching the service layer"
        );
    }

    struct RecordingInventoryService {
        response: FoodItem,
        recorded_calls: Mutex<Vec<AddFoodItem>>,
    }

    impl RecordingInventoryService {
        fn successful(item: FoodItem) -> Self {
            Self {
                response: item,
                recorded_calls: Mutex::new(Vec::new()),
            }
        }

        fn recorded_calls(&self) -> Vec<AddFoodItem> {
            self.recorded_calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl InventoryService for RecordingInventoryService {
        async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem> {
            self.recorded_calls.lock().unwrap().push(item.clone());
            Ok(self.response.clone())
        }
    }
}
