use anyhow::Result;
use async_trait::async_trait;
use inventory::dto::inventory_dto::AddFoodItem;
use inventory::dto::inventory_dto::FoodItem;
use inventory::traits::ServiceInventoryService;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresInventoryService<'a> {
    pool: &'a PgPool,
}

impl<'a> PostgresInventoryService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceInventoryService for PostgresInventoryService<'_> {
    async fn add_food_item(&self, item: &AddFoodItem) -> Result<FoodItem> {
        let name = &item.name;
        let quantity = item.quantity;
        if name.trim().is_empty() {
            anyhow::bail!("Food item name cannot be blank");
        }
        if quantity < 0 {
            anyhow::bail!("Adding negative quantity is not allowed");
        }
        if quantity == 0 {
            anyhow::bail!("Adding zero quantity does not change inventory");
        }

        let existing_item = sqlx::query("SELECT quantity FROM food_items WHERE name = $1")
            .bind(name)
            .fetch_optional(self.pool)
            .await?;

        let result_row = if let Some(row) = existing_item {
            let existing_quantity: i32 = row.try_get("quantity")?;
            let new_quantity = existing_quantity + quantity as i32;
            sqlx::query(
                "UPDATE food_items SET quantity = $1 WHERE name = $2 RETURNING id, quantity",
            )
            .bind(new_quantity)
            .bind(name)
            .fetch_one(self.pool)
            .await?
        } else {
            sqlx::query(
                "INSERT INTO food_items (name, quantity) VALUES ($1, $2) RETURNING id, quantity",
            )
            .bind(name)
            .bind(quantity as i32)
            .fetch_one(self.pool)
            .await?
        };
        let id: Uuid = result_row.try_get("id")?;
        let quantity_val: i32 = result_row.try_get("quantity")?;
        Ok(FoodItem {
            id,
            name: item.name.clone(),
            quantity: quantity_val,
        })
    }

    async fn list_food_items(&self) -> Result<Vec<FoodItem>> {
        let rows = sqlx::query("SELECT id, name, quantity FROM food_items ORDER BY name ASC")
            .fetch_all(self.pool)
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(FoodItem {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                quantity: row.try_get("quantity")?,
            });
        }

        Ok(items)
    }

    async fn delete_food_item(&self, id: Uuid) -> Result<FoodItem> {
        let row = sqlx::query("DELETE FROM food_items WHERE id = $1 RETURNING id, name, quantity")
            .bind(id)
            .fetch_one(self.pool)
            .await?;

        Ok(FoodItem {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            quantity: row.try_get("quantity")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inventory::dto::inventory_dto::AddFoodItem;
    use inventory::traits::ServiceInventoryService;
    use sqlx::PgPool;
    use sqlx::Row;
    use uuid::Uuid;

    #[sqlx::test]
    async fn deduplication_works(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        sut.add_food_item(&AddFoodItem {
            name: "Pizza".to_string(),
            quantity: 4,
        })
        .await?;
        sut.add_food_item(&AddFoodItem {
            name: "Pizza".to_string(),
            quantity: 6,
        })
        .await?;

        let rows = sqlx::query("SELECT name, quantity FROM food_items")
            .fetch_all(&pool)
            .await?;

        assert_eq!(rows.len(), 1, "Expected only one row in the database");
        let row = rows.first().expect("Expected at least one row");
        let name: String = row.try_get("name")?;
        let quantity: i32 = row.try_get("quantity")?;

        assert_eq!(name, "Pizza");
        assert_eq!(quantity, 10);

        Ok(())
    }

    #[sqlx::test]
    async fn list_of_food_items_can_be_retrieved(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        sut.add_food_item(&AddFoodItem {
            name: "Pizza".to_string(),
            quantity: 4,
        })
        .await?;
        sut.add_food_item(&AddFoodItem {
            name: "Sausages".to_string(),
            quantity: 100,
        })
        .await?;

        let rows = sqlx::query("SELECT name, quantity FROM food_items ORDER BY name ASC")
            .fetch_all(&pool)
            .await?;

        assert_eq!(rows.len(), 2, "Expected two rows in the database");
        let mut items = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let quantity: i32 = row.try_get("quantity")?;
            items.push((name, quantity));
        }

        assert!(items.contains(&("Pizza".to_string(), 4)));
        assert!(items.contains(&("Sausages".to_string(), 100)));

        Ok(())
    }

    #[sqlx::test]
    async fn newly_added_food_appears_in_inventory_reads(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        sut.add_food_item(&AddFoodItem {
            name: "Milk".to_string(),
            quantity: 2,
        })
        .await?;

        let rows = sqlx::query("SELECT name, quantity FROM food_items")
            .fetch_all(&pool)
            .await?;

        let mut items = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let quantity: i32 = row.try_get("quantity")?;
            items.push((name, quantity));
        }

        assert!(
            items.contains(&("Milk".to_string(), 2)),
            "Newly added food should be visible from inventory reads"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn food_item_ids_are_stable_across_repeated_reads(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        let added = sut
            .add_food_item(&AddFoodItem {
                name: "Pizza".to_string(),
                quantity: 4,
            })
            .await?;

        let first = sqlx::query("SELECT id FROM food_items WHERE name = $1")
            .bind("Pizza")
            .fetch_one(&pool)
            .await?;
        let second = sqlx::query("SELECT id FROM food_items WHERE name = $1")
            .bind("Pizza")
            .fetch_one(&pool)
            .await?;

        let first_id: Uuid = first.try_get("id")?;
        let second_id: Uuid = second.try_get("id")?;

        assert_eq!(
            first_id, added.id,
            "Stored inventory IDs should match the ID returned when the item was created"
        );
        assert_eq!(
            first_id, second_id,
            "Inventory item IDs should be stable across repeated reads of the same row"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn blank_food_names_are_rejected(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        let result = sut
            .add_food_item(&AddFoodItem {
                name: "   ".to_string(),
                quantity: 1,
            })
            .await;

        assert!(
            result.is_err(),
            "Inventory items should require a non-blank name"
        );

        let row = sqlx::query("SELECT COUNT(*) AS count FROM food_items")
            .fetch_one(&pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        assert_eq!(count, 0, "Rejected writes should not create inventory rows");

        Ok(())
    }

    #[sqlx::test]
    async fn delete_food_item_returns_error_for_non_existent_id(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        let random_id = Uuid::now_v7();
        let result = sut.delete_food_item(random_id).await;

        assert!(
            result.is_err(),
            "Deleting a non-existent ID should return an error"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn adding_negative_quantity_is_rejected(pool: PgPool) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        let result = sut
            .add_food_item(&AddFoodItem {
                name: "Milk".to_string(),
                quantity: -1,
            })
            .await;

        assert!(
            result.is_err(),
            "add_food_item should only accept stock being added to the house"
        );

        let row = sqlx::query("SELECT COUNT(*) AS count FROM food_items")
            .fetch_one(&pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        assert_eq!(count, 0, "Rejected writes should not create inventory rows");

        Ok(())
    }

    #[sqlx::test]
    async fn reducing_existing_stock_cannot_make_quantity_negative(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let sut = PostgresInventoryService::new(&pool);
        sut.add_food_item(&AddFoodItem {
            name: "Eggs".to_string(),
            quantity: 2,
        })
        .await?;
        let result = sut
            .add_food_item(&AddFoodItem {
                name: "Eggs".to_string(),
                quantity: -3,
            })
            .await;

        assert!(
            result.is_err(),
            "Inventory quantities should never drop below zero"
        );

        let row = sqlx::query("SELECT quantity FROM food_items WHERE name = $1")
            .bind("Eggs")
            .fetch_one(&pool)
            .await?;

        let quantity: i32 = row.try_get("quantity")?;
        assert_eq!(
            quantity, 2,
            "A rejected stock change should leave the existing quantity untouched"
        );

        Ok(())
    }
}
