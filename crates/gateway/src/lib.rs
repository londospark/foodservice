use anyhow::{Ok, Result};
use async_graphql::Object;

pub struct Query;

#[Object]
impl Query {
    async fn health(&self) -> Result<String> {
        Ok(format!("ok"))
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
}
