use anyhow::{Ok, Result};
use async_graphql::{Object, SimpleObject};

pub struct Query;
pub struct Mutation;

#[derive(SimpleObject)]
pub struct Food {
    name: String,
    qty: i32,
}

#[Object]
impl Query {
    async fn health(&self) -> Result<String> {
        Ok(format!("ok"))
    }

    async fn list_food(&self) -> Result<Vec<Food>> {
        Ok(vec![
            Food {
                name: "Pizza".to_string(),
                qty: 10,
            },
            Food {
                name: "Burger".to_string(),
                qty: 5,
            },
        ])
    }
}

#[Object]
impl Mutation {
    async fn add_food(&self, name: String, qty: i32) -> Result<Food> {
        Ok(Food { name, qty })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mutation, Query};
    use async_graphql::{EmptyMutation, EmptySubscription, Schema};

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
}
