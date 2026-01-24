use std::collections::HashMap;

fn main() {
    // Test HashMap go-to-definition
    let mut map: HashMap<String, i32> = HashMap::new();
    map.insert("test".to_string(), 42);

    println!("Map: {:?}", map);
}
