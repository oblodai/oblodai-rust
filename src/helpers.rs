//! Helpers for the two vocabularies every integration touches: decimal amounts and statuses.

use crate::core::money::Money;
use crate::generated::enums::{PaymentStatus, PayoutStatus};

/// Amounts are decimal strings; never parse them as `f64` (USDT has 6 decimals, BTC 8, ETH 18).
/// These compare and add at arbitrary precision using `i128` over a common scale.
///
/// ```
/// use oblodai::helpers::{add_amounts, compare_amounts};
/// # fn main() -> Result<(), oblodai::helpers::AmountError> {
/// assert_eq!(add_amounts("0.1", "0.2")?.as_str(), "0.3");
/// assert_eq!(compare_amounts("25", "25.000000")?, std::cmp::Ordering::Equal);
/// # Ok(()) }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a decimal amount: \"{0}\"")]
pub struct AmountError(pub String);

impl AmountError {
    /// The SDK error code this maps to, the same string in every Oblodai SDK.
    pub const CODE: &'static str = "sdk.bad_amount";

    /// The offending input.
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<AmountError> for crate::Error {
    /// A bad amount is a caller mistake caught before anything is sent, so it becomes a
    /// [`ErrorKind::Config`](crate::ErrorKind::Config) error with code `sdk.bad_amount`.
    fn from(err: AmountError) -> Self {
        crate::Error::config(AmountError::CODE, err.to_string(), Some("amount"))
    }
}

/// Longest string these helpers accept as an amount. Every real amount is far shorter (the widest
/// asset on the gateway has 18 decimals); the bound exists so hostile input costs nothing and can
/// never reach a formatting or allocation edge.
pub const MAX_AMOUNT_LEN: usize = 64;

struct Parts {
    negative: bool,
    integer: String,
    fraction: String,
}

fn parts(amount: &str) -> Result<Parts, AmountError> {
    let bad = || AmountError(amount.to_string());
    if amount.len() > MAX_AMOUNT_LEN {
        return Err(bad());
    }
    let negative = amount.starts_with('-');
    let body = if negative { &amount[1..] } else { amount };
    if body.is_empty() {
        return Err(bad());
    }
    let mut split = body.splitn(2, '.');
    let integer = split.next().unwrap_or("");
    let fraction = split.next().unwrap_or("");
    if integer.is_empty() || !integer.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    if body.contains('.') && (fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(bad());
    }
    Ok(Parts {
        negative,
        integer: integer.to_string(),
        fraction: fraction.to_string(),
    })
}

fn scaled(amount: &str, scale: usize) -> Result<i128, AmountError> {
    let p = parts(amount)?;
    let mut digits = p.integer;
    digits.push_str(&p.fraction);
    // Right-pad to the common scale. `"0".repeat` rather than a format width: the width argument
    // is a `u16`, so a long fraction would panic instead of returning `AmountError`.
    digits.push_str(&"0".repeat(scale.saturating_sub(p.fraction.len())));
    let value: i128 = digits
        .parse()
        .map_err(|_| AmountError(amount.to_string()))?;
    Ok(if p.negative { -value } else { value })
}

fn scale_of(a: &str, b: &str) -> Result<usize, AmountError> {
    Ok(parts(a)?.fraction.len().max(parts(b)?.fraction.len()))
}

fn unscale(value: i128, scale: usize) -> Money {
    let negative = value < 0;
    let digits = format!("{:0>width$}", value.unsigned_abs(), width = scale + 1);
    let split = digits.len() - scale;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&digits[..split]);
    if scale > 0 {
        out.push('.');
        out.push_str(&digits[split..]);
    }
    Money(out)
}

/// Compare two decimal amounts exactly. This is the only correct way to order [`Money`]: the type
/// deliberately has no `Ord`, because `"9.00"` sorts before `"10.00"` as a string.
pub fn compare_amounts(a: &str, b: &str) -> Result<std::cmp::Ordering, AmountError> {
    let scale = scale_of(a, b)?;
    Ok(scaled(a, scale)?.cmp(&scaled(b, scale)?))
}

/// Equality that ignores trailing zeros (`"25"` equals `"25.000000"`).
pub fn amounts_equal(a: &str, b: &str) -> Result<bool, AmountError> {
    Ok(compare_amounts(a, b)? == std::cmp::Ordering::Equal)
}

/// Exact decimal addition; the result keeps the wider scale of the two inputs.
pub fn add_amounts(a: &str, b: &str) -> Result<Money, AmountError> {
    let scale = scale_of(a, b)?;
    Ok(unscale(scaled(a, scale)? + scaled(b, scale)?, scale))
}

/// Exact decimal subtraction.
pub fn subtract_amounts(a: &str, b: &str) -> Result<Money, AmountError> {
    let scale = scale_of(a, b)?;
    Ok(unscale(scaled(a, scale)? - scaled(b, scale)?, scale))
}

/// Whether an amount is exactly zero, however it is written.
pub fn is_zero_amount(a: &str) -> Result<bool, AmountError> {
    let scale = parts(a)?.fraction.len();
    Ok(scaled(a, scale)? == 0)
}

// The status classes are facts of the contract (`x-status-classes`): `PaymentStatus::FINAL`,
// `PaymentStatus::SUCCESS` and their `is_final()` / `is_success()` are generated; the helpers below
// are the names this SDK has always offered for them.

/// Invoice statuses after which nothing else can happen.
pub const FINAL_PAYMENT_STATUSES: &[PaymentStatus] = PaymentStatus::FINAL;

/// Payout statuses after which nothing else can happen.
pub const FINAL_PAYOUT_STATUSES: &[PayoutStatus] = PayoutStatus::FINAL;

/// Nothing more will happen to this invoice.
pub fn is_payment_final(status: &PaymentStatus) -> bool {
    status.is_final()
}

/// The merchant has the money (`paid`, `paid_over`). `wrong_amount` is NOT paid: resolve it.
pub fn is_payment_paid(status: &PaymentStatus) -> bool {
    status.is_success()
}

/// The invoice is waiting for a merchant decision (underpaid): call `payments().resolve()`.
pub fn is_payment_underpaid(status: &PaymentStatus) -> bool {
    matches!(status, PaymentStatus::WrongAmount)
}

/// Nothing more will happen to this payout.
pub fn is_payout_final(status: &PayoutStatus) -> bool {
    status.is_final()
}

/// The payout reached the chain and is irreversible (`confirmed`).
pub fn is_payout_succeeded(status: &PayoutStatus) -> bool {
    status.is_success()
}
