use std::sync::Arc;

use anyhow::*;
use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};
use gateway::{Mutation, Query};
use inventory::client::InventoryClient;

fn playground(endpoint: impl AsRef<str> + std::fmt::Display) -> String {
    format!(
        r#"
<div style="width: 100%; height: 100%;" id='embedded-sandbox'></div>
<script src="https://embeddable-sandbox.cdn.apollographql.com/v2/embeddable-sandbox.umd.production.min.js"></script> 
<script>
  new window.EmbeddedSandbox({{
    target: '#embedded-sandbox',
    initialEndpoint: '{endpoint}',
  }});
</script>
"#
    )
}

fn schema_from_inventory_env_var() -> Schema<Query, Mutation, EmptySubscription> {
    let inventory_base_url = std::env::var("INVENTORY_BASE_URL")
        .expect("INVENTORY_BASE_URL environment variable must be set");
    schema_from_inventory_base_url(&inventory_base_url)
}

fn schema_from_inventory_base_url(
    inventory_base_url: &str,
) -> Schema<Query, Mutation, EmptySubscription> {
    let inventory = Arc::new(InventoryClient::new(inventory_base_url));
    Schema::build(
        Query::new(inventory.clone()),
        Mutation::new(inventory),
        EmptySubscription,
    )
    .finish()
}

async fn graphiql_handler() -> impl IntoResponse {
    Html(playground("/"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let schema = schema_from_inventory_env_var();
    let app = Router::new().route(
        "/",
        get(graphiql_handler).post_service(GraphQL::new(schema)),
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::schema_from_inventory_base_url;
    use async_graphql::Request;
    use axum::{
        Router,
        body::Bytes,
        http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
        response::IntoResponse,
        routing::post,
    };
    use inventory::{
        dto::gateway_dto::{AddFoodItem, FoodItem},
        protocol::{
            INVENTORY_V1_BINCODE_MEDIA_TYPE, decode_add_food_item, encode_food_item,
            encode_food_items,
        },
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn configured_gateway_schema_uses_the_inventory_service_for_add_food() {
        let upstream_item = FoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000031").unwrap(),
            name: "Milk".to_string(),
            quantity: 4,
        };
        let base_url = spawn_test_server(success_app(upstream_item.clone())).await;
        let schema = schema_from_inventory_base_url(&base_url);

        let response = schema
            .execute(Request::new(
                r#"
                mutation {
                    addFood(name: "Milk", qty: 4) {
                        id
                        name
                        qty
                    }
                }
            "#,
            ))
            .await;

        assert!(
            response.errors.is_empty(),
            "gateway mutations should succeed when the upstream inventory service accepts the request"
        );
        assert_eq!(
            response.data.to_string(),
            "{addFood: {id: \"00000000-0000-0000-0000-000000000031\", name: \"Milk\", qty: 4}}",
            "the running gateway configuration should surface the real inventory service response"
        );
    }

    #[tokio::test]
    async fn configured_gateway_schema_surfaces_upstream_inventory_failures() {
        let base_url = spawn_test_server(error_app(StatusCode::SERVICE_UNAVAILABLE)).await;
        let schema = schema_from_inventory_base_url(&base_url);

        let response = schema
            .execute(Request::new(
                r#"
                mutation {
                    addFood(name: "Milk", qty: 2) {
                        id
                        name
                        qty
                    }
                }
            "#,
            ))
            .await;

        assert!(
            !response.errors.is_empty(),
            "gateway mutations should not pretend success when the upstream inventory service fails"
        );
    }

    #[tokio::test]
    async fn configured_gateway_schema_uses_the_inventory_service_for_list_food() {
        let upstream_items = vec![
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000041").unwrap(),
                name: "Milk".to_string(),
                quantity: 2,
            },
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000042").unwrap(),
                name: "Eggs".to_string(),
                quantity: 12,
            },
        ];
        let base_url = spawn_test_server(success_app_for_list(upstream_items.clone())).await;
        let schema = schema_from_inventory_base_url(&base_url);

        let response = schema
            .execute(Request::new(
                r#"
                query {
                    listFood {
                        id
                        name
                        qty
                    }
                }
            "#,
            ))
            .await;

        assert!(
            response.errors.is_empty(),
            "gateway reads should succeed when the upstream inventory service accepts the request"
        );
        assert_eq!(
            response.data.to_string(),
            "{listFood: [{id: \"00000000-0000-0000-0000-000000000041\", name: \"Milk\", qty: 2}, {id: \"00000000-0000-0000-0000-000000000042\", name: \"Eggs\", qty: 12}]}",
            "the running gateway configuration should surface the real inventory service read response"
        );
    }

    #[tokio::test]
    async fn configured_gateway_schema_uses_the_inventory_service_for_delete_food() {
        let deleted_id = Uuid::parse_str("00000000-0000-0000-0000-000000000043").unwrap();
        let deleted_item = FoodItem {
            id: deleted_id,
            name: "Milk".to_string(),
            quantity: 2,
        };
        let base_url = spawn_test_server(success_app_for_delete(deleted_item.clone())).await;
        let schema = schema_from_inventory_base_url(&base_url);

        let response = schema
            .execute(Request::new(format!(
                r#"
                mutation {{
                    deleteFood(id: "{deleted_id}") {{
                        id
                        name
                        qty
                    }}
                }}
            "#
            )))
            .await;

        assert!(
            response.errors.is_empty(),
            "gateway deletes should succeed when the upstream inventory service accepts the request"
        );
        assert_eq!(
            response.data.to_string(),
            format!("{{deleteFood: {{id: \"{deleted_id}\", name: \"Milk\", qty: 2}}}}"),
            "the running gateway configuration should surface the real inventory service delete response"
        );
    }

    fn success_app(response: FoodItem) -> Router {
        Router::new().route(
            "/food",
            post(move |headers: HeaderMap, body: Bytes| {
                let response = response.clone();
                async move { success_handler(headers, body, response).await }
            }),
        )
    }

    fn error_app(status: StatusCode) -> Router {
        Router::new().route("/food", post(move || async move { status }))
    }

    fn success_app_for_list(response: Vec<FoodItem>) -> Router {
        Router::new().route(
            "/food",
            axum::routing::get(move || {
                let response = response.clone();
                async move {
                    (
                        StatusCode::OK,
                        [(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)],
                        encode_food_items(&response).expect("test response should encode"),
                    )
                }
            }),
        )
    }

    fn success_app_for_delete(response: FoodItem) -> Router {
        Router::new().route(
            "/food/{id}",
            axum::routing::delete(move || {
                let response = response.clone();
                async move {
                    (
                        StatusCode::OK,
                        [(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)],
                        encode_food_item(&response).expect("test response should encode"),
                    )
                }
            }),
        )
    }

    async fn success_handler(
        headers: HeaderMap,
        body: Bytes,
        response: FoodItem,
    ) -> impl IntoResponse {
        let media_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("gateway client should send a content type");
        assert_eq!(
            media_type, INVENTORY_V1_BINCODE_MEDIA_TYPE,
            "gateway should call inventory with the shared binary media type"
        );

        let command = decode_add_food_item(&body).expect("gateway request should decode");
        assert_eq!(
            command,
            AddFoodItem {
                name: "Milk".to_string(),
                quantity: 4,
            },
            "gateway should forward GraphQL writes as the shared AddFoodItem command"
        );

        (
            StatusCode::OK,
            [(CONTENT_TYPE, INVENTORY_V1_BINCODE_MEDIA_TYPE)],
            encode_food_item(&response).expect("test response should encode"),
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
