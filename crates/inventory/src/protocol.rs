use crate::dto::gateway_dto::{AddFoodItem, FoodItem};

pub const INVENTORY_V1_BINCODE_MEDIA_TYPE: &str =
    "application/vnd.foodservice.inventory.v1+bincode";

pub fn encode_add_food_item(item: &AddFoodItem) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(item)?)
}

pub fn decode_add_food_item(bytes: &[u8]) -> anyhow::Result<AddFoodItem> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn encode_food_item(item: &FoodItem) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(item)?)
}

pub fn decode_food_item(bytes: &[u8]) -> anyhow::Result<FoodItem> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn encode_food_items(items: &[FoodItem]) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(items)?)
}

pub fn decode_food_items(bytes: &[u8]) -> anyhow::Result<Vec<FoodItem>> {
    Ok(bincode::deserialize(bytes)?)
}

#[cfg(test)]
mod tests {
    use crate::dto::gateway_dto::{AddFoodItem, FoodItem};
    use crate::protocol::{
        decode_add_food_item, decode_food_item, decode_food_items, encode_add_food_item,
        encode_food_item, encode_food_items,
    };
    use uuid::Uuid;

    #[test]
    fn add_food_item_round_trips_as_transport_agnostic_binary() {
        let item = AddFoodItem {
            name: "Milk".to_string(),
            quantity: 2,
        };

        let encoded = encode_add_food_item(&item).expect("binary protocol should encode");
        let decoded =
            decode_add_food_item(&encoded).expect("binary protocol should decode its own bytes");

        assert_eq!(
            decoded, item,
            "Inventory commands should round-trip without depending on HTTP-specific types"
        );
    }

    #[test]
    fn food_item_round_trips_as_transport_agnostic_binary() {
        let item = FoodItem {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            name: "Milk".to_string(),
            quantity: 2,
        };

        let encoded = encode_food_item(&item).expect("binary protocol should encode");
        let decoded =
            decode_food_item(&encoded).expect("binary protocol should decode its own bytes");

        assert_eq!(
            decoded, item,
            "Inventory results should round-trip without depending on any one transport"
        );
    }

    #[test]
    fn food_item_lists_round_trip_as_transport_agnostic_binary() {
        let items = vec![
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                name: "Milk".to_string(),
                quantity: 2,
            },
            FoodItem {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                name: "Eggs".to_string(),
                quantity: 12,
            },
        ];

        let encoded = encode_food_items(&items).expect("binary protocol should encode");
        let decoded =
            decode_food_items(&encoded).expect("binary protocol should decode its own bytes");

        assert_eq!(
            decoded, items,
            "Inventory read results should round-trip without depending on any one transport"
        );
    }
}
