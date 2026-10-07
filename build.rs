fn main() {
    // Ensure correct linker arguments for PyO3 extension modules on all platforms.
    pyo3_build_config::add_extension_module_link_args();
}
