use anyhow::{Ok, Result};
use async_graphql::{Object, SimpleObject};
use uuid::Uuid;

pub struct Query;
pub struct Mutation;

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

#[Object]
impl Mutation {
    async fn add_food(&self, name: String, qty: usize) -> Result<FoodItem> {
        if qty <= 0 {
            Err(anyhow::anyhow!("Quantity must be a positive integer"))
        } else {
            Ok(FoodItem {
                id: Uuid::now_v7(),
                name,
                qty,
            })
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

    fn schema() -> Schema<Query, Mutation, EmptySubscription> {
        Schema::new(Query, Mutation, EmptySubscription)
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
        let schema = Schema::new(Query, Mutation, EmptySubscription);
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
        let schema = Schema::new(Query, Mutation, EmptySubscription);
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
        let schema = Schema::new(Query, Mutation, EmptySubscription);
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
        let schema = Schema::new(Query, Mutation, EmptySubscription);
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
    async fn list_food_returns_stable_ids_across_queries() {
        let schema = schema();

        let first = schema.execute("{ listFood { id name qty } }").await;
        let second = schema.execute("{ listFood { id name qty } }").await;

        assert!(first.errors.is_empty());
        assert!(second.errors.is_empty());
        assert_eq!(
            first.data.to_string(),
            second.data.to_string(),
            "Inventory item IDs should be stable across repeated reads of the same data"
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
    async fn newly_added_food_appears_in_list_food() {
        let schema = schema();

        let add_res = schema
            .execute(
                r#"
                mutation {
                    addFood(name: "Milk", qty: 2) {
                        id
                        name
                        qty
                    }
                }
            "#,
            )
            .await;

        assert!(
            add_res.errors.is_empty(),
            "The mutation should succeed before the API exposes the new item"
        );

        let list_res = schema.execute("{ listFood { name qty } }").await;

        assert!(
            list_res
                .data
                .to_string()
                .contains("{name: \"Milk\", qty: 2}"),
            "Newly added food should be visible from the listFood query"
        );
    }

    #[tokio::test]
    async fn deleting_unknown_food_returns_error() {
        let schema = schema();

        let res = schema
            .execute(
                r#"
                mutation {
                    deleteFood(id: "00000000-0000-0000-0000-00000000ffff") {
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
            "Deleting an item that is not in inventory should return an error"
        );
    }
}
