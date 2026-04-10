#![forbid(unsafe_code)]

use radroots_identity::{
    RadrootsIdentity, RadrootsIdentityEncryptedSecretKeyOptions,
    RadrootsIdentityEncryptedSecretKeySecurity,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadrootsAppApprovedFixtureIdentity {
    pub label: &'static str,
    pub account_id: &'static str,
    pub username: &'static str,
    pub email: &'static str,
    pub secret_key_hex: &'static str,
    pub nsec: &'static str,
    pub npub: &'static str,
}

pub const FIXTURE_ALICE: RadrootsAppApprovedFixtureIdentity = RadrootsAppApprovedFixtureIdentity {
    label: "fixture_alice",
    account_id: "fixture-account-alice",
    username: "fixture_alice",
    email: "fixture_alice@fixtures.test",
    secret_key_hex: "10c5304d6c9ae3a1a16f7860f1cc8f5e3a76225a2663b3a989a0d775919b7df5",
    nsec: "nsec1zrznqntvnt36rgt00ps0rny0tca8vgj6ye3m82vf5rthtyvm0h6syu7drz",
    npub: "npub1tp2ez55a5zatxxemrv0eses3ea05xhw2snuh3jy7azjqejn3q00s3vy5a9",
};

pub const FIXTURE_BOB: RadrootsAppApprovedFixtureIdentity = RadrootsAppApprovedFixtureIdentity {
    label: "fixture_bob",
    account_id: "fixture-account-bob",
    username: "fixture_bob",
    email: "fixture_bob@fixtures.test",
    secret_key_hex: "59392e9068f66431b12f70218fb61281cb6b433d7f27c55d61f1a63fe1a96ff8",
    nsec: "nsec1tyujayrg7ejrrvf0wqscldsjs89kksea0unu2htp7xnrlcdfdluqrjya9h",
    npub: "npub1uqnxu08mp55gd7guw06ls68nhxp8xuf7tlxe0sypvcl42x9ykwhsd55k2g",
};

pub const FIXTURE_CAROL: RadrootsAppApprovedFixtureIdentity = RadrootsAppApprovedFixtureIdentity {
    label: "fixture_carol",
    account_id: "fixture-account-carol",
    username: "fixture_carol",
    email: "fixture_carol@fixtures.test",
    secret_key_hex: "4d6c20fdd86857de77ff5cfa5c545751ba2efd126e0b6642dae9764d782d6509",
    nsec: "nsec1f4kzplwcdptaualltna9c4zh2xazalgjdc9kvsk6a9my67pdv5ys2pqkaj",
    npub: "npub1r9ft33558zvtemluludhdxwy5a66f5fmf2d6qztt5fh0q3yjhvwqgzmkl6",
};

pub const FIXTURE_DIEGO: RadrootsAppApprovedFixtureIdentity = RadrootsAppApprovedFixtureIdentity {
    label: "fixture_diego",
    account_id: "fixture-account-diego",
    username: "fixture_diego",
    email: "fixture_diego@fixtures.test",
    secret_key_hex: "9de56c1fdfce9ab00af85b3d7003c1d15cffb84cdf303c3a83c1a3fb1a2d0db0",
    nsec: "nsec1nhjkc87le6dtqzhctv7hqq7p69w0lwzvmucrcw5rcx3lkx3dpkcqkrmgp5",
    npub: "npub1t5l2kmncadlyv757r94xx3tvn7hmj0ac3dc99wpj9xrs3zvj82jqwwcglm",
};

pub const RELAY_PRIMARY_WSS: &str = "wss://relay.example.com";
pub const RELAY_SECONDARY_WSS: &str = "wss://relay-2.example.com";
pub const RELAY_TERTIARY_WSS: &str = "wss://relay-3.example.com";

pub const APP_PRIMARY_URL: &str = "https://app.example.com";
pub const API_PRIMARY_URL: &str = "https://api.example.com";
pub const CDN_PRIMARY_URL: &str = "https://cdn.example.com";
pub const FIXTURE_BACKUP_PASSWORD: &str = "fixture-backup-password";

pub fn fixture_identity(
    fixture: &RadrootsAppApprovedFixtureIdentity,
) -> Result<RadrootsIdentity, radroots_identity::IdentityError> {
    RadrootsIdentity::from_secret_key_str(fixture.secret_key_hex)
}

pub fn fixture_identity_ncryptsec(
    fixture: &RadrootsAppApprovedFixtureIdentity,
    password: &str,
) -> Result<String, radroots_identity::IdentityError> {
    fixture_identity(fixture)?.encrypt_secret_key_ncryptsec_with_options(
        password,
        RadrootsIdentityEncryptedSecretKeyOptions {
            log_n: 10,
            key_security: RadrootsIdentityEncryptedSecretKeySecurity::Weak,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_fixture_identities_match_exported_strings() {
        for fixture in [FIXTURE_ALICE, FIXTURE_BOB, FIXTURE_CAROL, FIXTURE_DIEGO] {
            let identity = fixture_identity(&fixture).expect("fixture identity");
            assert_eq!(identity.secret_key_hex(), fixture.secret_key_hex);
            assert_eq!(identity.nsec(), fixture.nsec);
            assert_eq!(identity.npub(), fixture.npub);
        }
    }
}
