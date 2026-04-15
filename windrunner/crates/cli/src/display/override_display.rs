pub fn print_creating_override(filepath: &str, line_num: Option<u32>) {
    println!("🔧 Creating override configuration...");
    println!("   📍 File: {filepath}");
    if let Some(l) = line_num {
        println!("   📍 Line: {}", l + 1);
    }
}

pub fn print_no_runnable() {
    println!("   📄 No specific runnable found, creating file-level override");
}

pub fn print_found_kind(kind: &str) {
    println!("   🎯 Found: {kind}");
}

pub fn print_file_type(ft: &str) {
    println!("   📝 File type: {ft}");
}

pub fn print_config_loading(path: &str) {
    println!("   📂 Loading configs for path: {path}");
}

pub fn print_section(section: &str) {
    println!("   🎨 Config section: {section}");
}

pub fn print_success_added() {
    println!("✅ Override added successfully!");
}

pub fn print_success_updated() {
    println!("✅ Override updated successfully!");
}

pub fn print_success_removed() {
    println!("✅ Override removed successfully!");
}

pub fn print_no_remove_match() {
    println!("❌ No matching override found to remove");
}
