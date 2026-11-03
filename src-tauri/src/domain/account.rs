use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Personal,
    PropFirm,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self { AccountKind::Personal => "personal", AccountKind::PropFirm => "prop_firm" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "personal" => Some(AccountKind::Personal),
            "prop_firm" => Some(AccountKind::PropFirm),
            _ => None,
        }
    }
    pub fn is_prop_firm(&self) -> bool { matches!(self, AccountKind::PropFirm) }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub currency: String,
    pub initial_balance: Decimal,
    pub is_active: bool,
    pub kind: AccountKind,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewAccount {
    pub name: String,
    pub currency: String,
    pub initial_balance: Decimal,
    pub kind: AccountKind,
}
