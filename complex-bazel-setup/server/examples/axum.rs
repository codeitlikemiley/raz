fn main() {
    println!("Axum example");
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_axum_example() {
        assert_eq!(1, 1);
    }

    #[test]
    #[should_panic]
    fn it_will_fail() {
        assert_eq!(1, 2);
    }
}
