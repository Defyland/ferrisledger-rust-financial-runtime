//! Core financial domain types for FerrisLedger.
//!
//! This crate deliberately has no dependency on HTTP, files, queues, or async
//! runtimes. It models money, account identity, and account state transitions.

use ferrisledger_macros::validated_string_id;
use serde::{Deserialize, Serialize};
use thiserror::Error;

validated_string_id!(
    /// Tenant partition identifier.
    pub struct TenantId;
);
validated_string_id!(
    /// Financial account identifier.
    pub struct AccountId;
);
validated_string_id!(
    /// Append-only stream identifier.
    pub struct StreamId;
);
validated_string_id!(
    /// Event identifier.
    pub struct EventId;
);
validated_string_id!(
    /// Correlation identifier propagated across API, store, and replay.
    pub struct CorrelationId;
);
validated_string_id!(
    /// Client-provided idempotency key.
    pub struct IdempotencyKey;
);
validated_string_id!(
    /// Settlement batch or provider identifier.
    pub struct SettlementId;
);
validated_string_id!(
    /// Ledger entry identifier.
    pub struct LedgerEntryId;
);

/// Domain-level validation and invariant failures.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum DomainError {
    /// ID newtype validation failed.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    /// Money amount or currency validation failed.
    #[error("invalid money: {0}")]
    InvalidMoney(String),
    /// Two monetary values use different currencies.
    #[error("currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        /// Expected ISO-4217 currency.
        expected: String,
        /// Actual ISO-4217 currency.
        actual: String,
    },
    /// Monetary arithmetic exceeded the supported i64 minor-unit range.
    #[error("money arithmetic overflow")]
    MoneyArithmeticOverflow,
    /// Account state does not allow the requested operation.
    #[error("account is not open")]
    AccountNotOpen,
    /// Requested debit exceeds available balance.
    #[error("insufficient funds: available {available_cents}, requested {requested_cents}")]
    InsufficientFunds {
        /// Available balance in minor units.
        available_cents: i64,
        /// Requested debit in minor units.
        requested_cents: i64,
    },
}

/// Money in minor units.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Money {
    cents: i64,
    currency: String,
}

impl Money {
    /// Creates a positive monetary value.
    pub fn new(cents: i64, currency: impl Into<String>) -> Result<Self, DomainError> {
        if cents <= 0 {
            return Err(DomainError::InvalidMoney(
                "amount must be greater than zero".to_string(),
            ));
        }
        let currency = validate_currency(currency.into())?;
        Ok(Self { cents, currency })
    }

    /// Creates a zero value for an account balance.
    pub fn zero(currency: impl Into<String>) -> Result<Self, DomainError> {
        let currency = validate_currency(currency.into())?;
        Ok(Self { cents: 0, currency })
    }

    /// Amount in minor units.
    #[must_use]
    pub const fn cents(&self) -> i64 {
        self.cents
    }

    /// ISO-4217 currency code.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// Adds two monetary values with the same currency.
    pub fn checked_add(&self, other: &Self) -> Result<Self, DomainError> {
        self.ensure_same_currency(other)?;
        let cents = self
            .cents
            .checked_add(other.cents)
            .ok_or(DomainError::MoneyArithmeticOverflow)?;
        Ok(Self {
            cents,
            currency: self.currency.clone(),
        })
    }

    /// Subtracts two monetary values with the same currency.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, DomainError> {
        self.ensure_same_currency(other)?;
        let cents = self
            .cents
            .checked_sub(other.cents)
            .ok_or(DomainError::MoneyArithmeticOverflow)?;
        Ok(Self {
            cents,
            currency: self.currency.clone(),
        })
    }

    /// Verifies that two values can be combined.
    pub fn ensure_same_currency(&self, other: &Self) -> Result<(), DomainError> {
        if self.currency == other.currency {
            return Ok(());
        }
        Err(DomainError::CurrencyMismatch {
            expected: self.currency.clone(),
            actual: other.currency.clone(),
        })
    }
}

fn validate_currency(currency: String) -> Result<String, DomainError> {
    if currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Ok(currency);
    }
    Err(DomainError::InvalidMoney(
        "currency must be a 3-letter uppercase ISO code".to_string(),
    ))
}

/// Account lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    /// Account accepts deposits and transfers.
    Open,
    /// Account exists but cannot move money.
    Frozen,
    /// Account is terminal.
    Closed,
}

/// Rebuilt account projection from append-only events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountState {
    /// Tenant that owns the account.
    pub tenant_id: TenantId,
    /// Account identifier.
    pub account_id: AccountId,
    /// Current lifecycle status.
    pub status: AccountStatus,
    /// Booked account balance.
    pub balance: Money,
    /// Amount reserved by requested Pix transfers.
    pub pending_pix_out: Money,
    /// Number of events applied to this projection.
    pub version: u64,
}

impl AccountState {
    /// Opens a new zero-balance account.
    pub fn opened(
        tenant_id: TenantId,
        account_id: AccountId,
        currency: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let currency = currency.into();
        Ok(Self {
            tenant_id,
            account_id,
            status: AccountStatus::Open,
            balance: Money::zero(currency.clone())?,
            pending_pix_out: Money::zero(currency)?,
            version: 1,
        })
    }

    /// Available balance after outgoing reservations.
    pub fn available_balance(&self) -> Result<Money, DomainError> {
        self.balance.checked_sub(&self.pending_pix_out)
    }

    /// Applies an inbound deposit.
    pub fn deposit(&mut self, amount: &Money) -> Result<(), DomainError> {
        self.ensure_open()?;
        self.balance = self.balance.checked_add(amount)?;
        self.version += 1;
        Ok(())
    }

    /// Reserves funds for an outgoing Pix transfer.
    pub fn reserve_pix_transfer(&mut self, amount: &Money) -> Result<(), DomainError> {
        self.ensure_open()?;
        let available = self.available_balance()?;
        available.ensure_same_currency(amount)?;
        if available.cents() < amount.cents() {
            return Err(DomainError::InsufficientFunds {
                available_cents: available.cents(),
                requested_cents: amount.cents(),
            });
        }
        self.pending_pix_out = self.pending_pix_out.checked_add(amount)?;
        self.version += 1;
        Ok(())
    }

    /// Settles a previously reserved outgoing transfer.
    pub fn settle_reserved_transfer(&mut self, amount: &Money) -> Result<(), DomainError> {
        self.ensure_open()?;
        self.pending_pix_out.ensure_same_currency(amount)?;
        if self.pending_pix_out.cents() < amount.cents() {
            return Err(DomainError::InsufficientFunds {
                available_cents: self.pending_pix_out.cents(),
                requested_cents: amount.cents(),
            });
        }
        self.pending_pix_out = self.pending_pix_out.checked_sub(amount)?;
        self.balance = self.balance.checked_sub(amount)?;
        self.version += 1;
        Ok(())
    }

    fn ensure_open(&self) -> Result<(), DomainError> {
        match self.status {
            AccountStatus::Open => Ok(()),
            AccountStatus::Frozen | AccountStatus::Closed => Err(DomainError::AccountNotOpen),
        }
    }
}

/// Builds the canonical account stream identifier.
pub fn account_stream_id(
    tenant_id: &TenantId,
    account_id: &AccountId,
) -> Result<StreamId, DomainError> {
    StreamId::new(format!(
        "tenant:{}:account:{}",
        tenant_id.as_str(),
        account_id.as_str()
    ))
    .map_err(DomainError::InvalidIdentifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn tenant() -> TenantId {
        TenantId::new("tenant_001").expect("valid tenant")
    }

    fn account() -> AccountId {
        AccountId::new("account_001").expect("valid account")
    }

    #[test]
    fn reserves_and_settles_pix_without_losing_money() {
        let mut state = AccountState::opened(tenant(), account(), "BRL").expect("opened");
        state
            .deposit(&Money::new(10_000, "BRL").expect("deposit"))
            .expect("deposit applied");
        state
            .reserve_pix_transfer(&Money::new(2_500, "BRL").expect("pix"))
            .expect("reserve applied");

        assert_eq!(state.available_balance().expect("available").cents(), 7_500);

        state
            .settle_reserved_transfer(&Money::new(2_500, "BRL").expect("settlement"))
            .expect("settlement applied");

        assert_eq!(state.balance.cents(), 7_500);
        assert_eq!(state.pending_pix_out.cents(), 0);
    }

    #[test]
    fn rejects_transfer_above_available_balance() {
        let mut state = AccountState::opened(tenant(), account(), "BRL").expect("opened");
        state
            .deposit(&Money::new(1_000, "BRL").expect("deposit"))
            .expect("deposit applied");

        let error = state
            .reserve_pix_transfer(&Money::new(1_001, "BRL").expect("pix"))
            .expect_err("insufficient funds");

        assert_eq!(
            error,
            DomainError::InsufficientFunds {
                available_cents: 1_000,
                requested_cents: 1_001,
            }
        );
    }

    #[test]
    fn rejects_money_addition_overflow() {
        let max = Money::new(i64::MAX, "BRL").expect("max");
        let one = Money::new(1, "BRL").expect("one");

        let error = max.checked_add(&one).expect_err("overflow");

        assert_eq!(error, DomainError::MoneyArithmeticOverflow);
    }

    #[test]
    fn rejects_money_subtraction_overflow() {
        let min_internal_state = Money {
            cents: i64::MIN,
            currency: "BRL".to_string(),
        };
        let one = Money::new(1, "BRL").expect("one");

        let error = min_internal_state.checked_sub(&one).expect_err("overflow");

        assert_eq!(error, DomainError::MoneyArithmeticOverflow);
    }

    proptest! {
        #[test]
        fn deposit_then_reserve_never_makes_available_negative(
            deposit_cents in 1_i64..1_000_000_i64,
            reserve_cents in 1_i64..1_000_000_i64,
        ) {
            let mut state = AccountState::opened(tenant(), account(), "BRL").expect("opened");
            state.deposit(&Money::new(deposit_cents, "BRL").expect("deposit")).expect("deposit applied");
            let reserve = Money::new(reserve_cents, "BRL").expect("reserve");
            let result = state.reserve_pix_transfer(&reserve);

            if reserve_cents <= deposit_cents {
                prop_assert!(result.is_ok());
                prop_assert!(state.available_balance().expect("available").cents() >= 0);
            } else {
                prop_assert!(
                    matches!(result, Err(DomainError::InsufficientFunds { .. })),
                    "expected insufficient funds"
                );
            }
        }
    }
}
