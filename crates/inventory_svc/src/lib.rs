use anyhow::Result;
use async_trait::async_trait;
use inventory::dto::AddFoodItem;
use inventory::dto::FoodItem;
use inventory::traits::InventoryService;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

pub struct PostgresInventoryService {
    pool: PgPool,
}

#[async_trait]
impl InventoryService for PostgresInventoryService {
    async fn add_food_item(&self, item: &AddFoodItem) -> Result<FoodItem> {
        let id = add_food_item(&self.pool, &item.name, item.quantity).await?;
        Ok(FoodItem {
            id,
            name: item.name.clone(),
            quantity: item.quantity,
        })
    }
}

pub async fn add_food_item(pool: &PgPool, name: &str, quantity: i32) -> anyhow::Result<Uuid> {
    // TODO(londo): This validation should be done in the gateway and then the type system should be used to ensure that the service only receives valid data. For now, this is a quick way to prevent bad data from being written to the database.
    if name.trim().is_empty() {
        anyhow::bail!("Food item name cannot be blank");
    }
    if quantity == 0 {
        anyhow::bail!("Adding zero quantity does not change inventory");
    }
    if quantity < 0 {
        anyhow::bail!("add_food_item should only accept stock being added to the house");
    }

    let existing_item = sqlx::query("SELECT quantity FROM food_items WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    let result: (Uuid,) = if let Some(row) = existing_item {
        let existing_quantity: i32 = row.try_get("quantity")?;
        let new_quantity = existing_quantity + quantity;
        sqlx::query_as("UPDATE food_items SET quantity = $1 WHERE name = $2 RETURNING id")
            .bind(new_quantity)
            .bind(name)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_as("INSERT INTO food_items (name, quantity) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(quantity)
            .fetch_one(pool)
            .await?
    };
    Ok(result.0)
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use sqlx::Row;
    use uuid::Uuid;

    #[sqlx::test]
    async fn the_database_is_setup(pool: PgPool) -> anyhow::Result<()> {
        let id_row =
            sqlx::query("INSERT INTO food_items (name, quantity) VALUES ($1, $2) RETURNING id")
                .bind("Sausages")
                .bind(100)
                .fetch_one(&pool)
                .await?;

        let id: Uuid = id_row.try_get("id")?;

        let row = sqlx::query("SELECT name, quantity FROM food_items WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await?;

        let name: String = row.try_get("name")?;
        let quantity: i32 = row.try_get("quantity")?;

        assert_eq!(name, "Sausages");
        assert_eq!(quantity, 100);

        Ok(())
    }

    #[sqlx::test]
    async fn adding_a_new_food_item_works(pool: PgPool) -> anyhow::Result<()> {
        crate::add_food_item(&pool, "Pizza", 4).await?;

        let row = sqlx::query("SELECT name, quantity FROM food_items")
            .fetch_one(&pool)
            .await?;

        let name: String = row.try_get("name")?;
        let quantity: i32 = row.try_get("quantity")?;

        assert_eq!(name, "Pizza");
        assert_eq!(quantity, 4);

        Ok(())
    }

    #[sqlx::test]
    async fn deduplication_works(pool: PgPool) -> anyhow::Result<()> {
        crate::add_food_item(&pool, "Pizza", 4).await?;
        crate::add_food_item(&pool, "Pizza", 6).await?;

        let rows = sqlx::query("SELECT name, quantity FROM food_items")
            .fetch_all(&pool)
            .await?;

        assert_eq!(rows.len(), 1, "Expected only one row in the database");
        let row = rows
            .first()
            .expect("Expected at least one row in the database");

        let name: String = row.try_get("name")?;
        let quantity: i32 = row.try_get("quantity")?;

        assert_eq!(name, "Pizza");
        assert_eq!(quantity, 10);

        Ok(())
    }

    #[sqlx::test]
    async fn list_of_food_items_can_be_retrieved(pool: PgPool) -> anyhow::Result<()> {
        crate::add_food_item(&pool, "Pizza", 4).await?;
        crate::add_food_item(&pool, "Sausages", 100).await?;

        let rows = sqlx::query("SELECT name, quantity FROM food_items")
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
    async fn adding_zero_quantity_is_rejected(pool: PgPool) -> anyhow::Result<()> {
        let result = crate::add_food_item(&pool, "Milk", 0).await;

        assert!(
            result.is_err(),
            "Adding zero units should be rejected because it does not change household inventory"
        );

        let row = sqlx::query("SELECT COUNT(*) AS count FROM food_items")
            .fetch_one(&pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        assert_eq!(count, 0, "Rejected writes should not create inventory rows");

        Ok(())
    }

    #[sqlx::test]
    async fn adding_negative_quantity_is_rejected(pool: PgPool) -> anyhow::Result<()> {
        let result = crate::add_food_item(&pool, "Milk", -1).await;

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
        crate::add_food_item(&pool, "Eggs", 2).await?;

        let result = crate::add_food_item(&pool, "Eggs", -3).await;

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

    #[sqlx::test]
    async fn blank_food_names_are_rejected(pool: PgPool) -> anyhow::Result<()> {
        let result = crate::add_food_item(&pool, "", 1).await;

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
}
