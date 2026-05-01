# Food Service Smoke Test Suite

## 1. List initial food items (Initial state check)
POST http://localhost:3000
Content-Type: application/json

{
  "query": "query { listFood { id name qty } }"
}

HTTP 200
[Asserts]
jsonpath "$.data.listFood" exists

## 2. Add first food item (Apples)
POST http://localhost:3000
Content-Type: application/json

{
  "query": "mutation { addFood(name: \"Apples\", qty: 10) { id name qty } }"
}

HTTP 200
[Captures]
apples_id: jsonpath "$.data.addFood.id"

## 3. Add second food item (Oranges)
POST http://localhost:3000
Content-Type: application/json

{
  "query": "mutation { addFood(name: \"Oranges\", qty: 5) { id name qty } }"
}

HTTP 200
[Captures]
oranges_id: jsonpath "$.data.addFood.id"

## 4. List and verify additions
POST http://localhost:3000
Content-Type: application/json

{
  "query": "query { listFood { id name qty } }"
}

HTTP 200
[Asserts]
jsonpath "$.data.listFood[*].name" contains "Apples"
jsonpath "$.data.listFood[*].name" contains "Oranges"

## 5. Delete first food item (Apples)
POST http://localhost:3000
Content-Type: application/json

{
  "query": "mutation { deleteFood(id: \"{{apples_id}}\") { id name qty } }"
}

HTTP 200

## 6. List and verify deletion
POST http://localhost:3000
Content-Type: application/json

{
  "query": "query { listFood { id name qty } }"
}

HTTP 200
[Asserts]
jsonpath "$.data.listFood[*].name" contains "Oranges"
jsonpath "$.data.listFood[*].name" not contains "Apples"