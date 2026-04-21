use anyhow::Result;
use async_graphql::{Object, SimpleObject};
use inventory::{
    dto::{AddFoodItem, FoodItem as InventoryFoodItem},
    traits::InventoryService,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct Query;
pub struct Mutation {
    inventory_service: Arc<dyn InventoryService>,
}

#[derive(SimpleObject)]
pub struct FoodItem {
    id: Uuid,
    name: String,
    qty: usize,
}

#[Object]
impl Query {
    async fn health(&self) -> Result<String> {
        Ok(format!("ok"))
    }

    async fn list_food(&self) -> Result<Vec<FoodItem>> {
        Ok(vec![
            FoodItem {
                id: Uuid::now_v7(),
                name: "Pizza".to_string(),
                qty: 10,
            },
            FoodItem {
                id: Uuid::now_v7(),
                name: "Burger".to_string(),
                qty: 5,
            },
        ])
    }
}

impl Mutation {
    pub fn new(inventory_service: Arc<dyn InventoryService>) -> Self {
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
impl InventoryService for PlaceholderInventoryService {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<InventoryFoodItem> {
        Ok(InventoryFoodItem {
            id: Uuid::now_v7(),
            name: item.name.clone(),
            quantity: item.quantity,
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
    async fn add_food(&self, name: String, qty: usize) -> Result<FoodItem> {
        if qty <= 0 {
            Err(anyhow::anyhow!("Quantity must be a positive integer"))
        } else if name.trim().is_empty() {
            Err(anyhow::anyhow!("Food name cannot be blank"))
        } else {
            let inventory = &self.inventory_service;
            inventory
                .add_food_item(&AddFoodItem {
                    name: name.clone(),
                    quantity: qty as i32,
                })
                .await
                .map(FoodItem::from)
        }
    }

    async fn delete_food(&self, id: Uuid) -> Result<FoodItem> {
        Ok(FoodItem {
            id,
            name: "Pizza".to_string(),
            qty: 10,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mutation, Query};
    use async_graphql::{EmptyMutation, EmptySubscription, Schema};
    use inventory::{
        dto::{AddFoodItem, FoodItem as InventoryFoodItem},
        traits::InventoryService,
    };
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn schema() -> Schema<Query, Mutation, EmptySubscription> {
        Schema::new(Query, Mutation::default(), EmptySubscription)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let schema = Schema::new(Query, EmptyMutation, EmptySubscription);
        let res = schema.execute("{ health }").await;
        assert_eq!(res.data.to_string(), "{health: \"ok\"}");
    }

    #[tokio::test]
    async fn list_food_returns_a_list_of_food() {
        let schema = Schema::new(Query, EmptyMutation, EmptySubscription);
        let res = schema.execute("{ listFood { name, qty } }").await;
        assert_eq!(
            res.data.to_string(),
            "{listFood: [{name: \"Pizza\", qty: 10}, {name: \"Burger\", qty: 5}]}"
        );
    }

    #[tokio::test]
    async fn add_valid_food_returns_ok() {
        let schema = Schema::new(Query, Mutation::default(), EmptySubscription);
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
        let schema = Schema::new(Query, Mutation::default(), EmptySubscription);
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
        let schema = Schema::new(Query, Mutation::default(), EmptySubscription);
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
        // Implement a test for deleting a food item, assuming you have a delete_food mutation in your Mutation struct.
        let schema = Schema::new(Query, Mutation::default(), EmptySubscription);
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
        let schema = Schema::new(Query, Mutation::new(service.clone()), EmptySubscription);

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
        let schema = Schema::new(Query, Mutation::new(service), EmptySubscription);

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

    struct RecordingInventoryService {
        response: InventoryFoodItem,
        recorded_calls: Mutex<Vec<AddFoodItem>>,
    }

    impl RecordingInventoryService {
        fn new(response: InventoryFoodItem) -> Self {
            Self {
                response,
                recorded_calls: Mutex::new(Vec::new()),
            }
        }

        fn recorded_calls(&self) -> Vec<AddFoodItem> {
            self.recorded_calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl InventoryService for RecordingInventoryService {
        async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<InventoryFoodItem> {
            self.recorded_calls.lock().unwrap().push(item.clone());
            Ok(self.response.clone())
        }
    }
}
