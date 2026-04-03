pub fn parse_filepath_with_line(filepath_arg: &str) -> (String, Option<usize>) {
    if let Some(colon_pos) = filepath_arg.rfind(':') {
        let path_part = &filepath_arg[..colon_pos];
        let line_part = &filepath_arg[colon_pos + 1..];

        // Check if it's a valid line number
        if let Ok(line_num) = line_part.parse::<usize>() {
            // Convert 1-based to 0-based
            (path_part.to_string(), Some(line_num.saturating_sub(1)))
        } else {
            // Not a valid line number, treat the whole thing as a path
            (filepath_arg.to_string(), None)
        }
    } else {
        (filepath_arg.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_with_line() {
        let (path, line) = parse_filepath_with_line("src/main.rs:42");
        assert_eq!(path, "src/main.rs");
        assert_eq!(line, Some(41)); // 1-based → 0-based
    }

    #[test]
    fn file_without_line() {
        let (path, line) = parse_filepath_with_line("src/main.rs");
        assert_eq!(path, "src/main.rs");
        assert_eq!(line, None);
    }

    #[test]
    fn file_with_line_1() {
        let (path, line) = parse_filepath_with_line("lib.rs:1");
        assert_eq!(path, "lib.rs");
        assert_eq!(line, Some(0)); // line 1 → index 0
    }

    #[test]
    fn file_with_line_0_saturates() {
        let (path, line) = parse_filepath_with_line("lib.rs:0");
        assert_eq!(path, "lib.rs");
        assert_eq!(line, Some(0)); // saturating_sub(1) from 0 → 0
    }

    #[test]
    fn file_with_invalid_line() {
        let (path, line) = parse_filepath_with_line("src/main.rs:abc");
        assert_eq!(path, "src/main.rs:abc");
        assert_eq!(line, None);
    }

    #[test]
    fn absolute_path_with_line() {
        let (path, line) = parse_filepath_with_line("/home/user/src/main.rs:10");
        assert_eq!(path, "/home/user/src/main.rs");
        assert_eq!(line, Some(9));
    }

    #[test]
    fn empty_string() {
        let (path, line) = parse_filepath_with_line("");
        assert_eq!(path, "");
        assert_eq!(line, None);
    }
}
