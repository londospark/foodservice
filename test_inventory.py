import requests
import json
from uuid import UUID

# GraphQL endpoint
GRAPHQL_URL = "http://localhost:3000/"

def execute_query(query, variables=None):
    """Execute a GraphQL query"""
    payload = {
        "query": query,
        "variables": variables or {}
    }
    response = requests.post(GRAPHQL_URL, json=payload)
    response.raise_for_status()
    return response.json()

def test_inventory_service():
    print("Testing inventory service...")
    
    # 1. Get initial list of food items
    print("\n1. Getting initial food list...")
    result = execute_query("{ listFood { name qty } }")
    initial_items = result['data']['listFood']
    print(f"Initial items: {len(initial_items)}")
    for item in initial_items:
        print(f"  - {item['name']}: {item['qty']}")
    
    # 2. Add new food items
    print("\n2. Adding new food items...")
    new_items = [
        {"name": "Apples", "qty": 10},
        {"name": "Bananas", "qty": 5},
        {"name": "Oranges", "qty": 8}
    ]
    
    for item in new_items:
        query = """
        mutation($name: String!, $qty: Int!) {
            addFood(name: $name, qty: $qty) {
                name
                qty
            }
        }
        """
        result = execute_query(query, {"name": item["name"], "qty": item["qty"]})
        print(f"  Added: {result['data']['addFood']['name']} - {result['data']['addFood']['qty']}")
    
    # 3. Verify items were added
    print("\n3. Verifying added items...")
    result = execute_query("{ listFood { name qty } }")
    all_items = result['data']['listFood']
    print(f"Total items now: {len(all_items)}")
    
    # Check that our new items are present
    new_item_names = {item["name"] for item in new_items}
    found_items = {item["name"] for item in all_items}
    
    for name in new_item_names:
        if name in found_items:
            print(f"  ✓ Found {name}")
        else:
            print(f"  ✗ Missing {name}")
    
    # 4. Delete some items
    print("\n4. Deleting some items...")
    # Get IDs of items to delete (we'll delete the first one)
    if all_items:
        item_to_delete = all_items[0]
        print(f"  Deleting: {item_to_delete['name']} (qty: {item_to_delete['qty']})")
        
        delete_query = """
        mutation($id: ID!) {
            deleteFood(id: $id) {
                name
                qty
            }
        }
        """
        result = execute_query(delete_query, {"id": str(item_to_delete["id"])})
        print(f"  Deleted: {result['data']['deleteFood']['name']} - {result['data']['deleteFood']['qty']}")
    
    # 5. Verify deletion
    print("\n5. Verifying deletion...")
    result = execute_query("{ listFood { name qty } }")
    remaining_items = result['data']['listFood']
    print(f"Items after deletion: {len(remaining_items)}")
    
    # Check that deleted item is no longer present
    if all_items and len(all_items) > 0:
        deleted_name = item_to_delete["name"]
        remaining_names = {item["name"] for item in remaining_items}
        if deleted_name not in remaining_names:
            print(f"  ✓ {deleted_name} successfully removed")
        else:
            print(f"  ✗ {deleted_name} still present")
    
    # 6. Test validation rules
    print("\n6. Testing validation rules...")
    
    # Negative quantity
    query = """
    mutation($name: String!, $qty: Int!) {
        addFood(name: $name, qty: $qty) {
            name
            qty
        }
    }
    """
    result = execute_query(query, {"name": "Invalid", "qty": -5})
    if 'errors' in result:
        print("  ✓ Negative quantity rejected")
    else:
        print("  ✗ Negative quantity accepted")
    
    # Blank name
    query = """
    mutation($name: String!, $qty: Int!) {
        addFood(name: $name, qty: $qty) {
            name
            qty
        }
    }
    """
    result = execute_query(query, {"name": "", "qty": 2})
    if 'errors' in result:
        print("  ✓ Blank name rejected")
    else:
        print("  ✗ Blank name accepted")
    
    print("\nTest completed successfully!")

if __name__ == "__main__":
    try:
        test_inventory_service()
    except Exception as e:
        print(f"Error during test: {e}")
        import traceback
        traceback.print_exc()
