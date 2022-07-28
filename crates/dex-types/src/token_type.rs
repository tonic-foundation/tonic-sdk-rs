use std::convert::TryFrom;

/// Implements structs representing token types supported on the Tonic CLOB.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId};

// TODO: Once MFT standard impl is merged, remove this and use
// `near_contract_standards::multi_token::token::TokenId`
pub type TokenId = String;

#[derive(Eq, Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "type")]
pub enum TokenType {
    #[serde(rename = "near")]
    NativeNear,
    #[serde(rename = "ft")]
    FungibleToken { account_id: AccountId },
    #[serde(rename = "mft")]
    MultiFungibleToken {
        account_id: AccountId,
        subtoken_id: TokenId,
    },
}

impl From<AccountId> for TokenType {
    fn from(account_id: AccountId) -> TokenType {
        TokenType::FungibleToken { account_id }
    }
}

impl From<&AccountId> for TokenType {
    fn from(account_id: &AccountId) -> TokenType {
        TokenType::FungibleToken {
            account_id: account_id.clone(),
        }
    }
}

impl ToString for TokenType {
    fn to_string(&self) -> String {
        match self {
            TokenType::NativeNear => "NativeNear".to_string(),
            TokenType::FungibleToken { account_id } => {
                format!("FungibleToken{{account_id: {}}}", account_id)
            }
            TokenType::MultiFungibleToken {
                account_id,
                subtoken_id,
            } => format!(
                "MultiFungibleToken{{account_id: {}, subtoken_id: {}}}",
                account_id, subtoken_id
            ),
        }
    }
}

impl TokenType {
    pub fn key(&self) -> String {
        match self {
            TokenType::NativeNear => "NEAR".to_string(),
            TokenType::FungibleToken { account_id } => format!("ft:{}", account_id),
            TokenType::MultiFungibleToken {
                account_id,
                subtoken_id,
            } => format!("mft:{}:{}", account_id, subtoken_id),
        }
    }

    pub fn from_key(key: &str) -> TokenType {
        if key == "NEAR" {
            TokenType::NativeNear
        } else if key.starts_with("ft:") {
            let parts: Vec<&str> = key.split(':').collect();
            TokenType::FungibleToken {
                account_id: AccountId::try_from(parts[1].to_string()).unwrap(),
            }
        } else if key.starts_with("mft:") {
            let parts: Vec<&str> = key.split(':').collect();
            TokenType::MultiFungibleToken {
                account_id: AccountId::try_from(parts[1].to_string()).unwrap(),
                subtoken_id: parts[2].to_string(),
            }
        } else {
            env::panic_str("invalid token ID")
        }
    }

    pub fn from_account_id(account_id: AccountId) -> TokenType {
        TokenType::FungibleToken { account_id }
    }
}
