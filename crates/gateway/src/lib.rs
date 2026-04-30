use anyhow::Result;
use async_graphql::{Object, SimpleObject};
use inventory::{
    dto::gateway_dto::{AddFoodItem, FoodItem as InventoryFoodItem},
    traits::GatewayInventoryService,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct Query {
    inventory_service: Arc<dyn GatewayInventoryService>,
}
pub struct Mutation {
    inventory_service: Arc<dyn GatewayInventoryService>,
}

#[derive(SimpleObject)]
pub struct FoodItem {
    id: Uuid,
    name: String,
    qty: usize,
}

impl Query {
    pub fn new(inventory_service: Arc<dyn GatewayInventoryService>) -> Self {
        Self { inventory_service }
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new(Arc::new(PlaceholderInventoryService))
    }
}

#[Object]
impl Query {
    async fn health(&self) -> Result<String> {
        Ok(format!("ok"))
    }

    async fn list_food(&self) -> Result<Vec<FoodItem>> {
        let inventory = &self.inventory_service;

        inventory
            .list_food_items()
            .await
            .map(|items| items.into_iter().map(FoodItem::from).collect())
    }
}

impl Mutation {
    pub fn new(inventory_service: Arc<dyn GatewayInventoryService>) -> Self {
        Self { inventory_service }
    }
}

impl Default for Mutation {
    fn default() -> Self {
        Self::new(Arc::new(PlaceholderInventoryService))
    }
}

struct PlaceholderInventoryService;

#[async_trait::async_trait]
impl GatewayInventoryService for PlaceholderInventoryService {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<InventoryFoodItem> {
        Ok(InventoryFoodItem {
            id: Uuid::now_v7(),
            name: item.name.clone(),
            quantity: item.quantity,
        })
    }

    async fn list_food_items(&self) -> anyhow::Result<Vec<InventoryFoodItem>> {
        Ok(vec![
            InventoryFoodItem {
                id: Uuid::now_v7(),
                name: "Pizza".to_string(),
                quantity: 10,
            },
            InventoryFoodItem {
                id: Uuid::now_v7(),
                name: "Burger".to_string(),
                quantity: 5,
            },
        ])
    }

    async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<InventoryFoodItem> {
        Ok(InventoryFoodItem {
            id,
            name: "Pizza".to_string(),
            quantity: 10,
        })
    }
}

impl From<InventoryFoodItem> for FoodItem {
    fn from(item: InventoryFoodItem) -> Self {
        Self {
            id: item.id,
            name: item.name,
            qty: item.quantity as usize,
        }
    }
}

#[Object]
impl Mutation {
    async fn add_food(&self, name: String, qty: i32) -> Result<FoodItem> {
        if qty <= 0 {
            Err(anyhow::anyhow!("Quantity must be a positive integer"))
        } else if name.trim().is_empty() {
            Err(anyhow::anyhow!("Food name cannot be blank"))
        } else {
            let inventory = &self.inventory_service;
            inventory
                .add_food_item(&AddFoodItem {
                    name: name.clone(),
                    quantity: qty as u32,
                })
                .await
                .map(FoodItem::from)
        }
    }

    async fn delete_food(&self, id: Uuid) -> Result<FoodItem> {
        let inventory = &self.inventory_service;
        inventory.delete_food_item(id).await.map(FoodItem::from)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mutation, Query};
    use async_graphql::{EmptyMutation, EmptySubscription, Schema};
    use inventory::{
        dto::gateway_dto::{AddFoodItem, FoodItem as InventoryFoodItem},
        traits::GatewayInventoryService,
    };
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn schema() -> Schema<Query, Mutation, EmptySubscription> {
        Schema::new(Query::default(), Mutation::default(), EmptySubscription)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let schema = Schema::new(Query::default(), EmptyMutation, EmptySubscription);
        let res = schema.execute("{ health }").await;
        assert_eq!(res.data.to_string(), "{health: \"ok\"}");
    }

    #[tokio::test]
    async fn list_food_returns_a_list_of_food() {
        let schema = Schema::new(Query::default(), EmptyMutation, EmptySubscription);
        let res = schema.execute("{ listFood { name, qty } }").await;
        assert_eq!(
            res.data.to_string(),
            "{listFood: [{name: \"Pizza\", qty: 10}, {name: \"Burger\", qty: 5}]}"
        );
    }

    #[tokio::test]
    async fn add_valid_food_returns_ok() {
        let schema = Schema::new(Query::default(), Mutation::default(), EmptySubscription);
        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Sushi", qty: 20) {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;
        assert_eq!(
            res.data.to_string(),
            "{addFood: {name: \"Sushi\", qty: 20}}"
        );
    }

    #[tokio::test]
    async fn add_food_with_negative_qty_returns_error() {
        let schema = Schema::new(Query::default(), Mutation::default(), EmptySubscription);
        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Sushi", qty: -5) {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;
        assert!(res.errors.len() > 0);
    }

    #[tokio::test]
    async fn add_food_with_zero_qty_returns_error() {
        let schema = Schema::new(Query::default(), Mutation::default(), EmptySubscription);
        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Sushi", qty: 0) {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;
        assert!(res.errors.len() > 0);
        assert_eq!(res.errors[0].message, "Quantity must be a positive integer");
    }

    #[tokio::test]
    async fn deleting_food_item_returns_ok() {
        let schema = Schema::new(Query::default(), Mutation::default(), EmptySubscription);
        let res = schema
            .execute(
                r#"
                mutation {
                    deleteFood(id: "00000000-0000-0000-0000-000000000001") {
                        id
                        name
                        qty
                    }
                }
            "#,
            )
            .await;
        assert_eq!(
            res.data.to_string(),
            "{deleteFood: {id: \"00000000-0000-0000-0000-000000000001\", name: \"Pizza\", qty: 10}}"
        );
    }

    #[tokio::test]
    async fn add_food_with_blank_name_returns_error() {
        let schema = schema();

        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "", qty: 2) {
                        id
                        name
                        qty
                    }
                }
            "#,
            )
            .await;
        assert!(
            !res.errors.is_empty(),
            "GraphQL clients should not be able to create anonymous food items"
        );
    }

    #[tokio::test]
    async fn add_food_delegates_to_the_inventory_service() {
        let service = Arc::new(RecordingInventoryService::new(InventoryFoodItem {
            id: Uuid::now_v7(),
            name: "Sushi".to_string(),
            quantity: 20,
        }));
        let schema = Schema::new(
            Query::new(service.clone()),
            Mutation::new(service.clone()),
            EmptySubscription,
        );

        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Sushi", qty: 20) {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;

        assert!(
            res.errors.is_empty(),
            "Gateway validation should allow this mutation before delegation is checked"
        );
        assert_eq!(
            service.recorded_calls(),
            vec![AddFoodItem {
                name: "Sushi".to_string(),
                quantity: 20,
            }],
            "Gateway mutations should delegate valid inventory writes to the inventory service"
        );
    }

    #[tokio::test]
    async fn add_food_returns_the_inventory_service_response() {
        let service = Arc::new(RecordingInventoryService::new(InventoryFoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000042").unwrap(),
            name: "Stored Sushi".to_string(),
            quantity: 21,
        }));
        let schema = Schema::new(
            Query::new(service.clone()),
            Mutation::new(service),
            EmptySubscription,
        );

        let res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Sushi", qty: 20) {
                        id
                        name
                        qty
                    }
                }
            "#,
            )
            .await;

        assert_eq!(
            res.data.to_string(),
            "{addFood: {id: \"00000000-0000-0000-0000-000000000042\", name: \"Stored Sushi\", qty: 21}}",
            "Gateway mutations should surface the inventory service result instead of inventing their own response"
        );
    }

    #[tokio::test]
    async fn list_food_delegates_to_the_inventory_service() {
        let service = Arc::new(RecordingInventoryService::with_list_response(vec![
            InventoryFoodItem {
                id: Uuid::now_v7(),
                name: "Milk".to_string(),
                quantity: 2,
            },
        ]));
        let schema = Schema::new(
            Query::new(service.clone()),
            EmptyMutation,
            EmptySubscription,
        );

        let res = schema
            .execute(
                r#"
                query {
                    listFood {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;

        assert!(res.errors.is_empty());
        assert_eq!(
            service.recorded_list_calls(),
            1,
            "Gateway reads should delegate inventory queries to the inventory service"
        );
    }

    #[tokio::test]
    async fn list_food_returns_the_inventory_service_response() {
        let service = Arc::new(RecordingInventoryService::with_list_response(vec![
            InventoryFoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000051").unwrap(),
                name: "Milk".to_string(),
                quantity: 12,
            },
            InventoryFoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000052").unwrap(),
                name: "Eggs".to_string(),
                quantity: 12,
            },
        ]));
        let schema = Schema::new(Query::new(service), EmptyMutation, EmptySubscription);

        let res = schema
            .execute(
                r#"
                query {
                    listFood {
                        name
                        qty
                    }
                }
            "#,
            )
            .await;

        assert_eq!(
            res.data.to_string(),
            "{listFood: [{name: \"Milk\", qty: 12}, {name: \"Eggs\", qty: 12}]}",
            "Gateway reads should surface the inventory service results instead of a hardcoded list"
        );
    }

    #[tokio::test]
    async fn delete_food_delegates_to_the_inventory_service() {
        let deleted_id = Uuid::parse_str("00000000-0000-0000-0000-000000000061").unwrap();
        let service = Arc::new(RecordingInventoryService::with_delete_response(
            InventoryFoodItem {
                id: deleted_id,
                name: "Milk".to_string(),
                quantity: 2,
            },
        ));
        let schema = Schema::new(
            Query::new(service.clone()),
            Mutation::new(service.clone()),
            EmptySubscription,
        );
        let res = schema
            .execute(format!(
                r#"
                mutation {{
                    deleteFood(id: "{deleted_id}") {{
                        id
                        name
                        qty
                    }}
                }}
            "#
            ))
            .await;
        assert!(res.errors.is_empty());
        assert_eq!(
            service.recorded_delete_calls(),
            vec![deleted_id],
            "Gateway delete mutations should delegate inventory deletes to the inventory service"
        );
    }

    #[tokio::test]
    async fn delete_food_returns_the_inventory_service_response() {
        let deleted_id = Uuid::parse_str("00000000-0000-0000-0000-000000000062").unwrap();
        let service = Arc::new(RecordingInventoryService::with_delete_response(
            InventoryFoodItem {
                id: deleted_id,
                name: "Deleted Milk".to_string(),
                quantity: 1,
            },
        ));
        let schema = Schema::new(
            Query::new(service.clone()),
            Mutation::new(service),
            EmptySubscription,
        );
        let res = schema
            .execute(format!(
                r#"
                mutation {{
                    deleteFood(id: "{deleted_id}") {{
                        id
                        name
                        qty
                    }}
                }}
            "#
            ))
            .await;
        assert_eq!(
            res.data.to_string(),
            format!("{{deleteFood: {{id: \"{deleted_id}\", name: \"Deleted Milk\", qty: 1}}}}"),
            "Gateway delete mutations should surface the inventory service response instead of a hardcoded item"
        );
    }

    struct RecordingInventoryService {
        add_response: InventoryFoodItem,
        list_response: Vec<InventoryFoodItem>,
        delete_response: InventoryFoodItem,
        recorded_calls: Mutex<Vec<AddFoodItem>>,
        recorded_list_counts: Mutex<usize>,
        recorded_delete_calls: Mutex<Vec<Uuid>>,
    }

    impl RecordingInventoryService {
        fn new(response: InventoryFoodItem) -> Self {
            Self {
                add_response: response.clone(),
                list_response: Vec::new(),
                delete_response: response,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_counts: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn with_list_response(response: Vec<InventoryFoodItem>) -> Self {
            let fallback = response.first().cloned().unwrap_or(InventoryFoodItem {
                id: Uuid::now_v7(),
                name: "Fallback".to_string(),
                quantity: 1,
            });
            Self {
                add_response: fallback.clone(),
                list_response: response,
                delete_response: fallback,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_counts: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn with_delete_response(response: InventoryFoodItem) -> Self {
            Self {
                add_response: response.clone(),
                list_response: Vec::new(),
                delete_response: response,
                recorded_calls: Mutex::new(Vec::new()),
                recorded_list_counts: Mutex::new(0),
                recorded_delete_calls: Mutex::new(Vec::new()),
            }
        }

        fn recorded_calls(&self) -> Vec<AddFoodItem> {
            self.recorded_calls.lock().unwrap().clone()
        }

        fn recorded_list_calls(&self) -> usize {
            *self.recorded_list_counts.lock().unwrap()
        }

        fn recorded_delete_calls(&self) -> Vec<Uuid> {
            self.recorded_delete_calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl GatewayInventoryService for RecordingInventoryService {
        async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<InventoryFoodItem> {
            self.recorded_calls.lock().unwrap().push(item.clone());
            Ok(self.add_response.clone())
        }

        async fn list_food_items(&self) -> anyhow::Result<Vec<InventoryFoodItem>> {
            *self.recorded_list_counts.lock().unwrap() += 1;
            Ok(self.list_response.clone())
        }

        async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<InventoryFoodItem> {
            self.recorded_delete_calls.lock().unwrap().push(id);
            Ok(self.delete_response.clone())
        }
    }
}
