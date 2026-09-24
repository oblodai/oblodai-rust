//! `Debug` of the generated models: the fields as `derive(Debug)` would print them, except that a
//! value under a secret-looking name ([`is_sensitive`]) is printed as `[redacted]` — in the
//! model's own fields and, at any depth, in its `extra` map. An absent (`None`) or empty secret is
//! printed as it is, so a debug line still says whether the gateway returned one.

use serde_json::Value;

use super::logger::is_sensitive;

const REDACTED: &str = "[redacted]";

struct Redacted;

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED)
    }
}

/// A copy of a JSON value with every secret-looking key's value replaced.
pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = if is_sensitive(k) {
                        Value::String(REDACTED.into())
                    } else {
                        redact_value(v)
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        other => other.clone(),
    }
}

/// The `Debug` builder the generated models use (`impl Debug for Model`).
pub struct ModelDebug<'a, 'b: 'a>(std::fmt::DebugStruct<'a, 'b>);

impl<'a, 'b: 'a> ModelDebug<'a, 'b> {
    pub fn new(f: &'a mut std::fmt::Formatter<'b>, name: &str) -> Self {
        Self(f.debug_struct(name))
    }

    /// One field, by its wire name.
    pub fn field<T: std::fmt::Debug>(&mut self, name: &str, value: &T) {
        if is_sensitive(name) {
            let shown = format!("{value:?}");
            if shown != "None" && shown != "\"\"" {
                self.0.field(name, &Redacted);
                return;
            }
        }
        self.0.field(name, value);
    }

    /// The fields this SDK version does not know, when there are any.
    pub fn extra(&mut self, extra: &serde_json::Map<String, Value>) {
        if !extra.is_empty() {
            self.0
                .field("extra", &redact_value(&Value::Object(extra.clone())));
        }
    }

    pub fn finish(&mut self) -> std::fmt::Result {
        self.0.finish()
    }
}
