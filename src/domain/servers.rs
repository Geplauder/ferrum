pub struct NewServer {
    pub name: ServerName,
}

pub struct ServerName(String);

impl ServerName {
    pub fn parse(value: String) -> Result<ServerName, String> {
        if validator::validate_length(&value, Some(4), Some(64), None) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid server name!", value))
        }
    }
}

impl AsRef<str> for ServerName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::ServerName;
    use fake::faker::company::en::CompanyName;
    use fake::Fake;

    #[test]
    fn name_too_short_is_rejected() {
        for x in ["", "abc"] {
            let name = x.to_string();

            assert!(ServerName::parse(name).is_err());
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=65).map(|_| "x").collect::<String>();

        assert!(ServerName::parse(name).is_err());
    }

    #[test]
    fn name_display_trait_implementation_is_valid() {
        let name = "foobar".to_string();

        let server_name = ServerName::parse(name).unwrap();

        assert_eq!("foobar", server_name.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidNameFixture(pub String);

    impl quickcheck::Arbitrary for ValidNameFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let name = CompanyName().fake_with_rng(g);

            println!("{:?}", name);

            Self(name)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_names_are_parsed_successfully(valid_name: ValidNameFixture) -> bool {
        ServerName::parse(valid_name.0).is_ok()
    }
}
