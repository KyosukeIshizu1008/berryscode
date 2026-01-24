// Test file for go-to-definition

fn test_function() {
    let v = Vec::new();
    v.push(1);
    v.push(2);
}

fn another_function() {
    test_function();
}
