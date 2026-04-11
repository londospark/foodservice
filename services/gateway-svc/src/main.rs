use anyhow::*;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};
use gateway::Query;

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

async fn graphiql_handler() -> impl IntoResponse {
    Html(playground("/"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let app = Router::new().route(
        "/",
        get(graphiql_handler).post_service(GraphQL::new(schema)),
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
