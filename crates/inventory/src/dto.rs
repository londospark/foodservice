use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddFoodItem {
    pub name: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoodItem {
    pub id: Uuid,
    pub name: String,
    pub quantity: i32,
}
