//! Test file with kebab-case name

fn main() {
    println!("Testing kebab-case to snake_case conversion!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}