use crate::{
    dto::{AddFoodItem, FoodItem},
    protocol::decode_food_item,
    traits::InventoryService,
};

pub struct InventoryClient {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl InventoryClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait::async_trait]
impl InventoryService for InventoryClient {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem> {
        let result = self
            .client
            .post(format!("{}/food", self.base_url))
            .header(
                reqwest::header::CONTENT_TYPE,
                crate::protocol::INVENTORY_V1_BINCODE_MEDIA_TYPE,
            )
            .body(crate::protocol::encode_add_food_item(item)?)
            .send()
            .await?
            .error_for_status()?;

        result
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .filter(|media_type| media_type == &crate::protocol::INVENTORY_V1_BINCODE_MEDIA_TYPE)
            .ok_or_else(|| anyhow::anyhow!("unexpected media type in response"))?;

        let response = result.text().await?;

        decode_food_item(response.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::InventoryClient;
    use crate::{
        dto::{AddFoodItem, FoodItem},
        protocol::{INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_add_food_item, encode_food_item},
        traits::InventoryService,
    };
    use axum::{
        Router,
        body::Bytes,
        extract::State,
        http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
        response::IntoResponse,
        routing::post,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[tokio::test]
    async fn add_food_item_posts_bincode_to_the_inventory_service_and_decodes_the_response() {
        let state = Arc::new(ObservedRequest::default());
        let returned_item = FoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            name: "Milk".to_string(),
            quantity: 2,
        };
        let base_url = spawn_test_server(success_app(state.clone(), returned_item.clone())).await;
        let client = InventoryClient::new(base_url);

        let result = client
            .add_food_item(&AddFoodItem {
                name: "Milk".to_string(),
                quantity: 2,
            })
            .await
            .expect("client should succeed when the service returns a valid binary response");

        assert_eq!(
            result, returned_item,
            "client responses should come from the inventory service rather than being invented locally"
        );
        assert_eq!(
            state.content_type().await,
            Some(INVENTORY_V1_BINCODE_MEDIA_TYPE.to_string()),
            "client requests should declare the inventory binary media type"
        );
        assert_eq!(
            state.command().await,
            Some(AddFoodItem {
                name: "Milk".to_string(),
                quantity: 2,
            }),
            "client requests should send the AddFoodItem command encoded as the shared binary protocol"
        );
    }

    #[tokio::test]
    async fn add_food_item_returns_an_error_for_non_success_responses() {
        let base_url = spawn_test_server(error_app(StatusCode::SERVICE_UNAVAILABLE)).await;
        let client = InventoryClient::new(base_url);

        let result = client
            .add_food_item(&AddFoodItem {
                name: "Milk".to_string(),
                quantity: 2,
            })
            .await;

        assert!(
            result.is_err(),
            "client calls should fail when the inventory service returns a non-success status"
        );
    }

    #[tokio::test]
    async fn add_food_item_rejects_success_responses_with_the_wrong_media_type() {
        let returned_item = FoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            name: "Milk".to_string(),
            quantity: 2,
        };
        let base_url = spawn_test_server(wrong_media_type_app(returned_item)).await;
        let client = InventoryClient::new(base_url);

        let result = client
            .add_food_item(&AddFoodItem {
                name: "Milk".to_string(),
                quantity: 2,
            })
            .await;

        assert!(
            result.is_err(),
            "client calls should keep the binary contract strict rather than silently accepting another media type"
        );
    }

    #[derive(Default)]
    struct ObservedRequest {
        command: Mutex<Option<AddFoodItem>>,
        content_type: Mutex<Option<String>>,
    }

    impl ObservedRequest {
        async fn record(&self, content_type: Option<String>, command: AddFoodItem) {
            *self.content_type.lock().await = content_type;
            *self.command.lock().await = Some(command);
        }

        async fn command(&self) -> Option<AddFoodItem> {
            self.command.lock().await.clone()
        }

        async fn content_type(&self) -> Option<String> {
            self.content_type.lock().await.clone()
        }
    }

    fn success_app(state: Arc<ObservedRequest>, response: FoodItem) -> Router {
        Router::new()
            .route("/food", post(success_handler))
            .with_state(SuccessState {
                observed: state,
                response,
            })
    }

    fn error_app(status: StatusCode) -> Router {
        Router::new()
            .route("/food", post(error_handler))
            .with_state(ErrorState { status })
    }

    fn wrong_media_type_app(response: FoodItem) -> Router {
        Router::new()
            .route("/food", post(wrong_media_type_handler))
            .with_state(WrongMediaTypeState { response })
    }

    #[derive(Clone)]
    struct SuccessState {
        observed: Arc<ObservedRequest>,
        response: FoodItem,
    }

    #[derive(Clone)]
    struct ErrorState {
        status: StatusCode,
    }

    #[derive(Clone)]
    struct WrongMediaTypeState {
        response: FoodItem,
    }

    async fn success_handler(
        State(state): State<SuccessState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let command = decode_add_food_item(&body).expect("test request should decode");
        state.observed.record(content_type, command).await;

        (
            StatusCode::OK,
            [(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)],
            encode_food_item(&state.response).expect("test response should encode"),
        )
    }

    async fn error_handler(State(state): State<ErrorState>) -> impl IntoResponse {
        state.status
    }

    async fn wrong_media_type_handler(
        State(state): State<WrongMediaTypeState>,
    ) -> impl IntoResponse {
        (
            StatusCode::OK,
            [(CONTENT_TYPE, "application/json")],
            encode_food_item(&state.response).expect("test response should encode"),
        )
    }

    async fn spawn_test_server(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server should bind to an ephemeral port");
        let address = listener
            .local_addr()
            .expect("test server should report its local address");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should run until the test exits");
        });

        format!("http://{address}")
    }
}
