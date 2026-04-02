fn main() {
    println!("Proxy binary");
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_proxy_binary() {
        assert_eq!(1, 1);
    }

    #[test]
    #[should_panic]
    fn it_will_fail() {
        assert_eq!(1, 2);
    }
}
