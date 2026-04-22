use crate::dto::{AddFoodItem, FoodItem};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait InventoryService: Send + Sync {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem>;
    async fn list_food_items(&self) -> anyhow::Result<Vec<FoodItem>>;
    async fn delete_food_item(&self, id: Uuid) -> anyhow::Result<FoodItem>;
}
