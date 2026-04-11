use anyhow::{Ok, Result};
use async_graphql::{Object, SimpleObject};

pub struct Query;

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

#[cfg(test)]
mod tests {
    use crate::Query;
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
}
