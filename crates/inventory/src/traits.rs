use crate::dto::{AddFoodItem, FoodItem};
use async_trait::async_trait;

#[async_trait]
pub trait InventoryService: Send + Sync {
    async fn add_food_item(&self, item: &AddFoodItem) -> anyhow::Result<FoodItem>;
}
