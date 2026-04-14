//! this is a stand alone rust file.

fn main() {
    println!("Simple Rust Calculator!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_two_numbers() {
        assert_eq!(2 + 3, 5);
    }
}