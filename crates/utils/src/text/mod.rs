#![forbid(unsafe_code)]

pub const ROOT_SYMBOL: &str = "»`,-";

#[cfg(test)]
mod tests {
    use super::ROOT_SYMBOL;

    #[test]
    fn root_symbol_matches_spec() {
        assert_eq!(ROOT_SYMBOL, "»`,-");
    }
}
