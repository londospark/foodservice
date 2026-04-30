pub mod gateway_dto;
pub mod inventory_dto;

pub use gateway_dto::{AddFoodItem as GatewayAddFoodItem, FoodItem as GatewayFoodItem};
pub use inventory_dto::{AddFoodItem as InventoryAddFoodItem, FoodItem as InventoryFoodItem};

impl From<&GatewayAddFoodItem> for InventoryAddFoodItem {
    fn from(item: &GatewayAddFoodItem) -> Self {
        Self {
            name: item.name.clone(),
            quantity: item.quantity as i32, // Fix 4
        }
    }
}

impl From<&InventoryAddFoodItem> for GatewayAddFoodItem {
    fn from(item: &InventoryAddFoodItem) -> Self {
        Self {
            name: item.name.clone(),
            quantity: item.quantity as u32, // Fix 4
        }
    }
}

impl From<&GatewayFoodItem> for InventoryFoodItem {
    fn from(item: &GatewayFoodItem) -> Self {
        Self {
            id: item.id,
            name: item.name.clone(),
            quantity: item.quantity as i32,
        }
    }
}

impl From<&InventoryFoodItem> for GatewayFoodItem {
    fn from(item: &InventoryFoodItem) -> Self {
        Self {
            id: item.id,
            name: item.name.clone(),
            quantity: item.quantity as u32,
        }
    }
}
