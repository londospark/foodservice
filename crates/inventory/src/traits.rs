#[async_trait::async_trait]
pub trait GatewayInventoryService: Send + Sync {
    async fn add_food_item(
        &self,
        item: &crate::dto::gateway_dto::AddFoodItem,
    ) -> anyhow::Result<crate::dto::gateway_dto::FoodItem>;
    async fn list_food_items(&self) -> anyhow::Result<Vec<crate::dto::gateway_dto::FoodItem>>;
    async fn delete_food_item(
        &self,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::dto::gateway_dto::FoodItem>;
}

#[async_trait::async_trait]
pub trait ServiceInventoryService: Send + Sync {
    async fn add_food_item(
        &self,
        item: &crate::dto::inventory_dto::AddFoodItem,
    ) -> anyhow::Result<crate::dto::inventory_dto::FoodItem>;
    async fn list_food_items(&self) -> anyhow::Result<Vec<crate::dto::inventory_dto::FoodItem>>;
    async fn delete_food_item(
        &self,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::dto::inventory_dto::FoodItem>;
}
