//! Amounts: a decimal carried as its exact string.

use serde::de::{Deserializer, Visitor};

/// The error code of a floating-point amount, the same string in every Oblodai SDK.
pub const FLOAT_AMOUNT: &str = "sdk.float_amount";

/// Decimal amount (`"10.000000"` for USDT), exactly as the gateway renders it.
///
/// Never a float: `f64` cannot hold 18 decimals, and rounding a payout is a real loss. There is no
/// `From<f64>`, so `amount: 25.5.into()` does not compile; a JSON number with a fraction or an
/// exponent is refused with [`FLOAT_AMOUNT`] when a request is built from JSON
/// ([`crate::from_json`]). An integer is exact and accepted.
///
/// `Money` implements neither `PartialOrd` nor `Ord`: the derived versions would compare the
/// strings, so `"9.00" > "10.00"`. Order amounts with [`crate::helpers::compare_amounts`], and
/// test equality with [`crate::helpers::amounts_equal`] when trailing zeros may differ (derived
/// `PartialEq` is exact string equality: `"25"` is not `"25.000000"`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Money(pub String);

impl Money {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Money {
    fn from(v: &str) -> Self {
        Money(v.to_string())
    }
}

impl From<String> for Money {
    fn from(v: String) -> Self {
        Money(v)
    }
}

impl From<&String> for Money {
    fn from(v: &String) -> Self {
        Money(v.clone())
    }
}

impl std::ops::Deref for Money {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Money;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a decimal amount as a string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Money, E> {
                Ok(Money(v.to_string()))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Money, E> {
                Ok(Money(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Money, E> {
                Ok(Money(v.to_string()))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Money, E> {
                Ok(Money(v.to_string()))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Money, E> {
                Err(E::custom(format!(
                    "{FLOAT_AMOUNT}: amount {v} is a float; pass a decimal string (\"{v}\")"
                )))
            }
        }
        d.deserialize_any(V)
    }
}
