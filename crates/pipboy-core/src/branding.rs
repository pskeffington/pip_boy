//! Canonical Rust-Boy product language; legacy identifiers remain stable.

/// Public-facing project name.
pub const PRODUCT_NAME: &str = "Rust-Boy";
/// Short product positioning statement.
pub const PRODUCT_DESCRIPTOR: &str = "Offline-first field systems console";
/// Existing executable name. Renaming requires a separate compatibility gate.
pub const CLI_COMMAND: &str = "pipboy";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_product_name_and_stable_command() {
        assert_eq!(PRODUCT_NAME, "Rust-Boy");
        assert_eq!(CLI_COMMAND, "pipboy");
        assert_eq!(PRODUCT_DESCRIPTOR, "Offline-first field systems console");
    }
}
