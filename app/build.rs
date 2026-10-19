fn main() {
    // Debug builds keep element info for the UI tests' accessibility lookups (ElementHandle).
    let config = slint_build::CompilerConfiguration::new().with_debug_info(std::env::var("PROFILE").as_deref() == Ok("debug"));
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint build failed");
}
