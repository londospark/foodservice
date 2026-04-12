use sqlx::{PgPool, Row};
pub async fn add_food_item(pool: &PgPool, name: &str, quantity: i32) -> anyhow::Result<()> {
    let existing_item = sqlx::query("SELECT quantity FROM food_items WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = existing_item {
        let existing_quantity: i32 = row.try_get("quantity")?;
        let new_quantity = existing_quantity + quantity;
        sqlx::query("UPDATE food_items SET quantity = $1 WHERE name = $2")
            .bind(new_quantity)
            .bind(name)
            .execute(pool)
            .await?;
    } else {
        sqlx::query("INSERT INTO food_items (name, quantity) VALUES ($1, $2)")
            .bind(name)
            .bind(quantity)
            .execute(pool)
            .await?;
    }

    Ok(())
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
}
