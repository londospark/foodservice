-- Add migration script here
CREATE TABLE IF NOT EXISTS food_items (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name VARCHAR(255) UNIQUE NOT NULL,
    quantity INTEGER NOT NULL
);

