#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use sqlx::Row;
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_inventory(pool: PgPool) -> anyhow::Result<()> {
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
}
