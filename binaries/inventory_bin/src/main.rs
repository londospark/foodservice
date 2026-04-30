use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
    routing::{delete, get, post},
};
use inventory::{
    dto::gateway_dto::{AddFoodItem, FoodItem},
    protocol::{
        INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_add_food_item, encode_food_item, encode_food_items,
    },
    traits::{GatewayInventoryService, ServiceInventoryService}, // Add the missing import
};
use inventory_svc::PostgresInventoryService;
use std::sync::Arc;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../crates/inventory_svc/migrations");

#[derive(Clone)]
struct AppState {
    inventory_service: Arc<dyn GatewayInventoryService>,
}

fn app(inventory_service: Arc<dyn GatewayInventoryService>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/food", post(add_item).get(list_food))
        .route("/food/{id}", delete(delete_food))
        .with_state(AppState { inventory_service })
}

#[tokio::main]
async fn main() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database");
    MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run database migrations");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(
        listener,
        app(Arc::new(RuntimePostgresInventoryService::new(pool))),
    )
    .await
    .unwrap();
}

async fn delete_food(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let inventory = &state.inventory_service;
    let response = inventory
        .delete_food_item(&id) // Fix the method call
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

async fn list_food(State(state): State<AppState>) -> impl IntoResponse {
    let inventory = &state.inventory_service;
    let response = inventory
        .list_food_items()
        .await
        .expect("inventory service should not fail in this example");

    let response = encode_food_items(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = response.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        INVENTORY_V1_BINCODE_MEDIA_TYPE.parse().unwrap(),
    );
    response
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

struct RuntimePostgresInventoryService {
    pool: sqlx::PgPool,
}

impl RuntimePostgresInventoryService {
    fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl GatewayInventoryService for RuntimePostgresInventoryService {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem> {
        let service_item = inventory::dto::InventoryAddFoodItem::from(item);
        let result = PostgresInventoryService::new(&self.pool)
            .add_food_item(&service_item)
            .await?;
        Ok(FoodItem::from(&result))
    }

    async fn list_food_items(&self) -> anyhow::Result<Vec<FoodItem>> {
        let results = PostgresInventoryService::new(&self.pool)
            .list_food_items()
            .await?;
        Ok(results.into_iter().map(FoodItem::from).collect())
    }

    async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<FoodItem> {
        let result = PostgresInventoryService::new(&self.pool)
            .delete_food_item(&id) // Fix the method call
            .await?;
        Ok(FoodItem::from(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimePostgresInventoryService, app};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use inventory::{
        dto::gateway_dto::{AddFoodItem, FoodItem},
        protocol::{
            INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_food_item, decode_food_items,
            encode_add_food_item,
        },
        traits::GatewayInventoryService,
    };
    use inventory_svc::PostgresInventoryService;
    use sqlx::{PgPool, Row};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    use uuid::Uuid;

    static MIGRATOR: sqlx::migrate::Migrator =
        sqlx::migrate!("../../crates/inventory_svc/migrations");

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

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn post_food_with_postgres_inventory_service_persists_the_written_food(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let request_body = encode_add_food_item(&AddFoodItem {
            name: "Milk".to_string(),
            quantity: 2,
        })?;

        let response = app(Arc::new(PostgresInventoryHarness::new(pool.clone())))
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
            "The inventory HTTP adapter should succeed when backed by the real Postgres service"
        );

        let row = sqlx::query("SELECT name, quantity FROM food_items WHERE name = $1")
            .bind("Milk")
            .fetch_one(&pool)
            .await?;

        let name: String = row.try_get("name")?;
        let quantity: i32 = row.try_get("quantity")?;
        assert_eq!(name, "Milk");
        assert_eq!(
            quantity, 2,
            "Successful POST /food requests should persist through the Postgres-backed inventory service"
        );

        Ok(())
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn post_food_with_postgres_inventory_service_returns_the_merged_quantity(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let first = encode_add_food_item(&AddFoodItem {
            name: "Pizza".to_string(),
            quantity: 4,
        })?;
        let second = encode_add_food_item(&AddFoodItem {
            name: "Pizza".to_string(),
            quantity: 6,
        })?;

        app(Arc::new(PostgresInventoryHarness::new(pool.clone())))
            .oneshot(
                Request::post("/food")
                    .header(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)
                    .body(Body::from(first))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        let response = app(Arc::new(PostgresInventoryHarness::new(pool.clone())))
            .oneshot(
                Request::post("/food")
                    .header(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)
                    .body(Body::from(second))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        let body = to_bytes(response.into_body(), usize::MAX).await?;
        let returned = decode_food_item(&body)?;

        assert_eq!(
            returned.quantity, 10,
            "When duplicate food is merged, the HTTP response should report the stored merged quantity"
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_food_uses_the_inventory_service_and_returns_binary_inventory_items() {
        let service = Arc::new(RecordingInventoryService::with_list_response(vec![
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000071").unwrap(),
                name: "Milk".to_string(),
                quantity: 2,
            },
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000072").unwrap(),
                name: "Eggs".to_string(),
                quantity: 12,
            },
        ]));

        let response = app(service.clone())
            .oneshot(Request::get("/food").body(Body::empty()).unwrap())
            .await
            .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "inventory_bin should expose a real GET /food endpoint rather than leaving reads unimplemented"
        );
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            INVENTORY_V1_BINCODE_MEDIA_TYPE,
            "inventory_bin should return list responses using the shared binary media type"
        );

        let response_body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let returned = decode_food_items(&response_body).expect("service responses should decode");

        assert_eq!(
            service.recorded_list_calls(),
            1,
            "the HTTP adapter should delegate inventory reads to the inventory service"
        );
        assert_eq!(
            returned,
            vec![
                FoodItem {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000071").unwrap(),
                    name: "Milk".to_string(),
                    quantity: 2,
                },
                FoodItem {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000072").unwrap(),
                    name: "Eggs".to_string(),
                    quantity: 12,
                },
            ],
            "the HTTP adapter should return the inventory service list result, encoded as bincode"
        );
    }

    #[tokio::test]
    async fn delete_food_uses_the_inventory_service_and_returns_the_deleted_food() {
        let deleted_id = Uuid::parse_str("00000000-0000-0000-0000-000000000073").unwrap();
        let service = Arc::new(RecordingInventoryService::with_delete_response(FoodItem {
            id: deleted_id,
            name: "Milk".to_string(),
            quantity: 2,
        }));

        let response = app(service.clone())
            .oneshot(
                Request::delete(format!("/food/{deleted_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "inventory_bin should expose a real DELETE /food/{{id}} endpoint rather than leaving deletes unimplemented"
        );
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            INVENTORY_V1_BINCODE_MEDIA_TYPE,
            "inventory_bin should return delete responses using the shared binary media type"
        );

        let response_body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let returned =
            decode_food_item(&response_body).expect("service responses should decode via bincode");

        assert_eq!(
            service.recorded_delete_calls(),
            vec![deleted_id],
            "the HTTP adapter should delegate inventory deletes to the inventory service"
        );
        assert_eq!(
            returned,
            FoodItem {
                id: deleted_id,
                name: "Milk".to_string(),
                quantity: 2,
            },
            "the HTTP adapter should return the deleted inventory item, encoded as bincode"
        );
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn app_from_pool_uses_the_postgres_inventory_service_for_runtime_writes(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let request_body = encode_add_food_item(&AddFoodItem {
            name: "Milk".to_string(),
            quantity: 2,
        })?;

        let response = app_from_pool(pool.clone())
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
            "the runtime app builder should wire inventory_bin to the Postgres-backed inventory service"
        );

        let row = sqlx::query("SELECT quantity FROM food_items WHERE name = $1")
            .bind("Milk")
            .fetch_one(&pool)
            .await?;

        let quantity: i32 = row.try_get("quantity")?;
        assert_eq!(
            quantity, 2,
            "the runtime app builder should persist writes through the Postgres-backed inventory service"
        );

        Ok(())
    }

    struct RecordingInventoryService {
        response: FoodItem,
        list_response: Vec<FoodItem>,
        delete_response: FoodItem,
        recorded_calls: Mutex<Vec<AddFoodItem>>,
        recorded_list_calls: Mutex<usize>,
        recorded_delete_calls: Mutex<Vec<Uuid>>,
    }

    struct PostgresInventoryHarness {
        pool: PgPool,
    }

    impl RecordingInventoryService {
        fn successful(item: FoodItem) -> Self {
            Self {
                response: item.clone(),
                list_response: Vec::new(),
                delete_response: item,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_calls: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn with_list_response(items: Vec<FoodItem>) -> Self {
            let fallback = items.first().cloned().unwrap_or(FoodItem {
                id: Uuid::now_v7(),
                name: "Fallback".to_string(),
                quantity: 1,
            });
            Self {
                response: fallback.clone(),
                list_response: items,
                delete_response: fallback,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_calls: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn with_delete_response(item: FoodItem) -> Self {
            Self {
                response: item.clone(),
                list_response: Vec::new(),
                delete_response: item,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_calls: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn recorded_calls(&self) -> Vec<AddFoodItem> {
            self.recorded_calls.lock().unwrap().clone()
        }

        fn recorded_list_calls(&self) -> usize {
            *self.recorded_list_calls.lock().unwrap()
        }

        fn recorded_delete_calls(&self) -> Vec<Uuid> {
            self.recorded_delete_calls.lock().unwrap().clone()
        }
    }

    impl PostgresInventoryHarness {
        fn new(pool: PgPool) -> Self {
            Self { pool }
        }
    }

    fn app_from_pool(pool: PgPool) -> axum::Router {
        app(Arc::new(RuntimePostgresInventoryService::new(pool)))
    }

    #[async_trait::async_trait]
    impl GatewayInventoryService for RecordingInventoryService {
        async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem> {
            self.recorded_calls.lock().unwrap().push(item.clone());
            Ok(self.response.clone())
        }

        async fn list_food_items(&self) -> anyhow::Result<Vec<FoodItem>> {
            *self.recorded_list_calls.lock().unwrap() += 1;
            Ok(self.list_response.clone())
        }

        async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<FoodItem> {
            self.recorded_delete_calls.lock().unwrap().push(id);
            Ok(self.delete_response.clone())
        }
    }

    #[async_trait::async_trait]
    impl GatewayInventoryService for PostgresInventoryHarness {
        async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem> {
            let service_item = inventory::dto::InventoryAddFoodItem::from(item);
            let result = PostgresInventoryService::new(&self.pool)
                .add_food_item(&service_item)
                .await?;
            Ok(FoodItem::from(&result))
        }

        async fn list_food_items(&self) -> anyhow::Result<Vec<FoodItem>> {
            let results = PostgresInventoryService::new(&self.pool)
                .list_food_items()
                .await?;
            Ok(results.into_iter().map(FoodItem::from).collect())
        }

        async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<FoodItem> {
            let result = PostgresInventoryService::new(&self.pool)
                .delete_food_item(id)
                .await?;
            Ok(FoodItem::from(&result))
        }
    }
}
