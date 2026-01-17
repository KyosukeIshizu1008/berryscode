//! Syntax highlighting test file

#[derive(Debug, Clone)]
#[test]
pub struct SyntaxTest {
    // Constants
    const MAX_SIZE: usize = 100;
    static GLOBAL_CONFIG: &str = "config";
}

fn test_macros() {
    // Macros with yellow color
    println!("Hello, world!");
    vec![1, 2, 3];
    dbg!(42);
    format!("test: {}", 123);
}

fn test_lifetimes<'a, 'b>(x: &'a str, y: &'b str) -> &'a str {
    // Lifetimes in teal
    let z: &'static str = "static lifetime";
    x
}

fn test_constants() {
    const PI: f64 = 3.14159;
    const BUFFER_SIZE: usize = 1024;
    let normal_var = 42;
}

use std::collections::HashMap;
use std::sync::Arc;

fn main() {
    test_macros();
    test_lifetimes("a", "b");
    test_constants();
}
