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
