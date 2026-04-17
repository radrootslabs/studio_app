#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    #[default]
    Farmer,
    Buyer,
}

impl AppMode {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Farmer => "farmer",
            Self::Buyer => "buyer",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmerSection {
    #[default]
    Today,
    Products,
    Orders,
    PackDay,
    Farm,
}

impl FarmerSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Today => "farmer.today",
            Self::Products => "farmer.products",
            Self::Orders => "farmer.orders",
            Self::PackDay => "farmer.pack_day",
            Self::Farm => "farmer.farm",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuyerSection {
    #[default]
    Marketplace,
    Search,
    Cart,
    Orders,
}

impl BuyerSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Marketplace => "buyer.marketplace",
            Self::Search => "buyer.search",
            Self::Cart => "buyer.cart",
            Self::Orders => "buyer.orders",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection {
    #[default]
    Account,
    Settings,
    About,
}

impl SettingsSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Account => "settings.account",
            Self::Settings => "settings.settings",
            Self::About => "settings.about",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "surface", content = "section", rename_all = "snake_case")]
pub enum ShellSection {
    #[default]
    Home,
    Farmer(FarmerSection),
    Buyer(BuyerSection),
    Settings(SettingsSection),
}

impl ShellSection {
    pub const fn mode(self) -> AppMode {
        match self {
            Self::Buyer(_) => AppMode::Buyer,
            Self::Home | Self::Farmer(_) | Self::Settings(_) => AppMode::Farmer,
        }
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Farmer(section) => section.storage_key(),
            Self::Buyer(section) => section.storage_key(),
            Self::Settings(section) => section.storage_key(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseShellSectionError;

impl fmt::Display for ParseShellSectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid shell section key")
    }
}

impl Error for ParseShellSectionError {}

impl FromStr for ShellSection {
    type Err = ParseShellSectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "home" => Ok(Self::Home),
            "farmer.today" => Ok(Self::Farmer(FarmerSection::Today)),
            "farmer.products" => Ok(Self::Farmer(FarmerSection::Products)),
            "farmer.orders" => Ok(Self::Farmer(FarmerSection::Orders)),
            "farmer.pack_day" => Ok(Self::Farmer(FarmerSection::PackDay)),
            "farmer.farm" => Ok(Self::Farmer(FarmerSection::Farm)),
            "buyer.marketplace" => Ok(Self::Buyer(BuyerSection::Marketplace)),
            "buyer.search" => Ok(Self::Buyer(BuyerSection::Search)),
            "buyer.cart" => Ok(Self::Buyer(BuyerSection::Cart)),
            "buyer.orders" => Ok(Self::Buyer(BuyerSection::Orders)),
            "settings.account" => Ok(Self::Settings(SettingsSection::Account)),
            "settings.settings" => Ok(Self::Settings(SettingsSection::Settings)),
            "settings.about" => Ok(Self::Settings(SettingsSection::About)),
            _ => Err(ParseShellSectionError),
        }
    }
}

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = uuid::Error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}

typed_id!(FarmId);
typed_id!(ProductId);
typed_id!(OrderId);
typed_id!(FulfillmentWindowId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmReadiness {
    Incomplete,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Draft,
    Published,
    Paused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    NeedsAction,
    Scheduled,
    Packed,
    Completed,
    Refunded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmSummary {
    pub farm_id: FarmId,
    pub display_name: String,
    pub readiness: FarmReadiness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodaySummary {
    pub farm_id: FarmId,
    pub orders_needing_action: u32,
    pub low_stock_products: u32,
    pub draft_products: u32,
}

impl TodaySummary {
    pub const fn has_attention_items(&self) -> bool {
        self.orders_needing_action > 0 || self.low_stock_products > 0 || self.draft_products > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductListRow {
    pub product_id: ProductId,
    pub farm_id: FarmId,
    pub title: String,
    pub status: ProductStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderListRow {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub order_number: String,
    pub customer_display_name: String,
    pub status: OrderStatus,
}

#[cfg(test)]
mod tests {
    use super::{
        AppMode, BuyerSection, FarmId, FarmerSection, SettingsSection, ShellSection, TodaySummary,
    };
    use std::{collections::BTreeSet, str::FromStr};
    use uuid::Uuid;

    #[test]
    fn shell_section_storage_keys_are_unique_and_round_trip() {
        let sections = [
            ShellSection::Home,
            ShellSection::Farmer(FarmerSection::Today),
            ShellSection::Farmer(FarmerSection::Products),
            ShellSection::Farmer(FarmerSection::Orders),
            ShellSection::Farmer(FarmerSection::PackDay),
            ShellSection::Farmer(FarmerSection::Farm),
            ShellSection::Buyer(BuyerSection::Marketplace),
            ShellSection::Buyer(BuyerSection::Search),
            ShellSection::Buyer(BuyerSection::Cart),
            ShellSection::Buyer(BuyerSection::Orders),
            ShellSection::Settings(SettingsSection::Account),
            ShellSection::Settings(SettingsSection::Settings),
            ShellSection::Settings(SettingsSection::About),
        ];
        let keys = sections
            .into_iter()
            .map(ShellSection::storage_key)
            .collect::<BTreeSet<_>>();

        assert_eq!(keys.len(), sections.len());

        for section in sections {
            let parsed =
                ShellSection::from_str(section.storage_key()).expect("section should parse");
            assert_eq!(parsed, section);
        }
    }

    #[test]
    fn shell_section_mode_tracks_farmer_and_buyer_surfaces() {
        assert_eq!(ShellSection::Home.mode(), AppMode::Farmer);
        assert_eq!(
            ShellSection::Farmer(FarmerSection::Today).mode(),
            AppMode::Farmer
        );
        assert_eq!(
            ShellSection::Buyer(BuyerSection::Marketplace).mode(),
            AppMode::Buyer
        );
        assert_eq!(
            ShellSection::Settings(SettingsSection::Settings).mode(),
            AppMode::Farmer
        );
    }

    #[test]
    fn typed_ids_round_trip_through_strings() {
        let uuid = Uuid::parse_str("018f4d61-19b0-7cc4-9d4e-6d0df7c0aa11")
            .expect("test uuid should parse");
        let farm_id = FarmId::from(uuid);
        let parsed = FarmId::from_str(&farm_id.to_string()).expect("farm id should parse");

        assert_eq!(parsed, farm_id);
        assert_eq!(parsed.as_uuid(), uuid);
    }

    #[test]
    fn today_summary_attention_state_is_explicit() {
        let quiet = TodaySummary {
            farm_id: FarmId::new(),
            orders_needing_action: 0,
            low_stock_products: 0,
            draft_products: 0,
        };
        let busy = TodaySummary {
            farm_id: FarmId::new(),
            orders_needing_action: 1,
            low_stock_products: 0,
            draft_products: 0,
        };

        assert!(!quiet.has_attention_items());
        assert!(busy.has_attention_items());
    }
}
