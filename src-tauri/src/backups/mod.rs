/// The first public backup envelope will use this version. Keeping the value in
/// one backend-owned module prevents UI and persistence code from diverging.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::CURRENT_SCHEMA_VERSION;

    #[test]
    fn backup_schema_starts_at_version_one() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    }
}
