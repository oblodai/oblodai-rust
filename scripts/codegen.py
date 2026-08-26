#!/usr/bin/env python3
"""Generate src/contract/{routes,enums,requests,version}.rs from the contract snapshot.

Sources (never edited by hand):
  contract/contract.json        route registry, enums, error codes, signing vectors
  contract/descriptions.en.json English field documentation

Usage:
  python3 scripts/codegen.py            regenerate in place (and rustfmt the result)
  python3 scripts/codegen.py --check    fail when the committed files differ (CI drift gate)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "src", "contract")
GENERATED = ["routes.rs", "enums.rs", "requests.rs", "version.rs"]

SKIP_PREFIXES = ("/healthz", "/readyz", "/docs", "/openapi.json", "/internal")

# The core's auth vocabulary after the single-key cleanup. `key` = signed with the merchant's
# one API key, `public` = unsigned, `onboard` = the self-hosted gateway's admin token. A value
# outside this set means the contract and this SDK disagree about how a route is authenticated,
# which is not something to guess about: fail the generation instead.
ROUTE_AUTH = {"public": "Public", "key": "Key", "onboard": "Onboard"}

HEADER = (
    "// GENERATED FILE - do not edit. Source: contract/contract.json (core {commit}).\n"
    "// Regenerate with: python3 scripts/codegen.py\n"
)

# --- enums ---------------------------------------------------------------------------------

ENUM_NAMES = {
    "payment_status": ("PaymentStatus", "Invoice lifecycle, as `payment.status` carries it."),
    "payout_status": ("PayoutStatus", "Payout lifecycle, as `payout.status` carries it."),
    "payout_link_status": ("PayoutLinkStatus", "Payout-link (cheque) lifecycle."),
    "delivery_status": ("DeliveryStatus", "Webhook delivery lifecycle."),
    "network": ("Network", "Settlement networks the gateway supports."),
    "fee_bearer": ("FeeBearer", "Who pays the network fee, as requested."),
    "fee_bearer_result": ("FeeBearerResult", "Who paid the network fee, as settled."),
    "batch_on_error": ("BatchOnError", "What an asynchronous batch does after a failed row."),
    "webhook_kind": ("WebhookKind", "Kind of sample event `webhooks.test` delivers."),
    "error_kind": ("CoreErrorKind", "How the core itself classifies a failure (the envelope's own taxonomy)."),
}
# Vocabularies the core does not export as enums yet; pinned here from its handlers.
LOCAL_ENUMS = {
    "AmountMode": (["fixed", "open", "range"], "How a payment link prices its invoices."),
    "PayoutKind": (["payout", "refund"], "Which side of the payout ledger `payout.history` lists."),
    "ResolveAction": (
        ["accept", "refund"],
        "How an underpaid (`wrong_amount`) invoice is settled.",
    ),
}

# --- request bodies ------------------------------------------------------------------------

# Request fields drawn from a generated enum: emitted as the enum, whose `Other(String)` variant
# keeps a value newer than this snapshot decodable.
FIELD_ENUMS = {
    "network": "Network",
    "pinned_network": "Network",
    "on_error": "BatchOnError",
    "fee_bearer": "FeeBearer",
    "amount_mode": "AmountMode",
}
ROUTE_FIELD_ENUMS = {
    "POST /v1/payment/history#status": "PaymentStatus",
    "POST /v1/payout/history#status": "PayoutStatus",
    "POST /v1/payout/history#kind": "PayoutKind",
    "POST /v1/payment/resolve#action": "ResolveAction",
    "POST /v1/test-webhook/payment#status": "PaymentStatus",
    "POST /v1/test-webhook/payout#status": "PayoutStatus",
    "POST /v1/payment/testing-webhook#status": "PaymentStatus",
}
MONEY_FIELD = re.compile(r"(^|_)(amount|min_amount|max_amount|amount_fixed)$")
# Fields the handler requires although the shared DTO marks them optional (batch items reuse the
# single-create DTO, where the core backfills the key from the Idempotency-Key header).
REQUIRED_OVERRIDES = {
    "POST /v1/payment/batch": ["payments.order_id"],
    "POST /v1/payout/batch": ["payouts.order_id"],
    "POST /v1/refund/batch": ["refunds.reference"],
    "POST /v1/transfer/batch": [
        "transfers.order_id",
        "transfers.amount",
        "transfers.currency",
    ],
    "POST /v1/payout/link/batch": ["items.reference"],
    "POST /v1/transfer/to-user": ["amount", "currency"],
    "POST /v1/claim/{token}": ["address"],
}
# Fields the shared DTO marks required although the handler does not need them (the dry-run
# validate reuses the create DTO, but reserves nothing and so needs no merchant reference).
OPTIONAL_OVERRIDES = {
    "POST /v1/payout/validate": ["order_id"],
}
# Request schemas for routes whose core DTO is not declared in docsapi (kept to one place so a
# future undocumented route has somewhere to go; remove an entry once the core documents it).
REQUEST_OVERRIDES = {
    "POST /v1/merchants": {
        "type": "object",
        "required": ["email"],
        "properties": {
            "email": {"type": "string", "example": "owner@shop.example"},
            "name": {"type": "string", "example": "Acme"},
        },
    },
}

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
}


def pascal(text: str) -> str:
    return "".join(part[:1].upper() + part[1:] for part in re.split(r"[^0-9A-Za-z]+", text) if part)


def route_key(route: dict) -> str:
    return f"{route['method']} {route['path']}"


def const_name(key: str) -> str:
    return re.sub(r"[^0-9A-Za-z]+", "_", key).strip("_").upper()


def request_struct_name(path: str) -> str:
    segments = [s for s in path.split("/") if s and s != "v1" and not s.startswith("{")]
    return pascal("_".join(segments)) + "Request"


def is_safe(route: dict) -> bool:
    """The core's own hand-classified read-only flag. Never inferred from the path."""
    value = route.get("safe")
    if not isinstance(value, bool):
        raise SystemExit(
            f"contract.json: route {route_key(route)} has no boolean \"safe\" field - "
            "re-export the contract from the core"
        )
    return value


def route_auth(route: dict) -> str:
    """The `RouteAuth` variant for a route, rejecting any vocabulary this SDK does not model."""
    value = route.get("auth")
    variant = ROUTE_AUTH.get(value) if isinstance(value, str) else None
    if variant is None:
        raise SystemExit(
            f"contract.json: route {route_key(route)} has auth {value!r}; expected one of "
            + ", ".join(sorted(ROUTE_AUTH))
            + " - re-export the contract from the core"
        )
    return variant


def doc_lines(text: str, indent: str = "") -> str:
    out = []
    for line in text.split("\n"):
        line = line.strip()
        out.append(f"{indent}/// {line}".rstrip())
    return "\n".join(out) + "\n"


def load() -> tuple[dict, dict, bytes]:
    with open(os.path.join(ROOT, "contract", "contract.json"), "rb") as fh:
        raw = fh.read()
    contract = json.loads(raw.decode("utf-8"))
    unclassified = [
        route_key(r) for r in contract.get("routes", []) if not isinstance(r.get("safe"), bool)
    ]
    if unclassified:
        raise SystemExit(
            "contract.json: "
            + str(len(unclassified))
            + ' route(s) lack a boolean "safe" field: '
            + ", ".join(unclassified[:5])
            + " - re-export the contract from the core"
        )
    unknown_auth = sorted(
        {
            str(r.get("auth"))
            for r in contract.get("routes", [])
            if r.get("auth") not in ROUTE_AUTH
        }
    )
    if unknown_auth:
        raise SystemExit(
            "contract.json: unknown auth value(s) "
            + ", ".join(unknown_auth)
            + "; expected one of "
            + ", ".join(sorted(ROUTE_AUTH))
            + " - re-export the contract from the core"
        )
    desc_path = os.path.join(ROOT, "contract", "descriptions.en.json")
    descriptions = {"request": {}, "response": {}}
    if os.path.exists(desc_path):
        with open(desc_path, "r", encoding="utf-8") as fh:
            descriptions = json.load(fh)
    return contract, descriptions, raw


# --- routes.rs -----------------------------------------------------------------------------


def gen_routes(contract: dict, header: str) -> str:
    routes = sorted(
        (r for r in contract["routes"] if not r["path"].startswith(SKIP_PREFIXES)),
        key=lambda r: (r["path"], r["method"]),
    )
    out = [header]
    out.append("use super::types::{ListKind, Method, RouteAuth, RouteSpec};\n")
    for r in routes:
        key = route_key(r)
        list_kind = (
            "None"
            if not r.get("list")
            else f"Some(ListKind::{ 'Paged' if r['list'] == 'paged' else 'Plain' })"
        )
        out.append(
            f"/// `{key}`\n"
            f"pub static {const_name(key)}: RouteSpec = RouteSpec {{\n"
            f'    key: "{key}",\n'
            f"    method: Method::{r['method'].capitalize()},\n"
            f'    path: "{r["path"]}",\n'
            f"    auth: RouteAuth::{route_auth(r)},\n"
            f"    idempotent: {str(r['idempotent']).lower()},\n"
            f"    safe: {str(is_safe(r)).lower()},\n"
            f"    bare: {str(r['bare']).lower()},\n"
            f"    list: {list_kind},\n"
            f"}};\n"
        )
    listed = ", ".join(f"&{const_name(route_key(r))}" for r in routes)
    out.append(
        "/// Every merchant-facing route the core declares, in the order its conformance table\n"
        "/// declares them. The SDK can call exactly these and nothing else.\n"
        f"pub static ROUTES: &[&RouteSpec] = &[{listed}];\n"
    )
    return "\n".join(out), len(routes)


# --- enums.rs ------------------------------------------------------------------------------


def gen_enum(name: str, values: list, doc: str) -> str:
    variants = []
    arms = []
    parse_arms = []
    for v in values:
        variant = pascal(v)
        variants.append(f'    #[serde(rename = "{v}")]\n    {variant},')
        arms.append(f'            Self::{variant} => "{v}",')
        parse_arms.append(f'            "{v}" => Self::{variant},')
    consts = ", ".join(f'"{v}"' for v in values)
    upper = re.sub(r"(?<!^)(?=[A-Z])", "_", name).upper()
    plural = upper + ("ES" if upper.endswith("S") else "S")
    return (
        f"{doc_lines(doc)}"
        "///\n"
        "/// `Other` keeps a value newer than this contract snapshot decodable instead of failing\n"
        "/// the whole response.\n"
        "#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]\n"
        "#[non_exhaustive]\n"
        f"pub enum {name} {{\n" + "\n".join(variants) + "\n"
        "    /// A value this snapshot does not know.\n"
        "    #[serde(untagged)]\n"
        "    Other(String),\n"
        "}\n\n"
        f"impl {name} {{\n"
        "    /// The wire value.\n"
        "    pub fn as_str(&self) -> &str {\n"
        "        match self {\n" + "\n".join(arms) + "\n"
        "            Self::Other(v) => v.as_str(),\n"
        "        }\n"
        "    }\n"
        "}\n\n"
        f"impl std::fmt::Display for {name} {{\n"
        "    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n"
        "        f.write_str(self.as_str())\n"
        "    }\n"
        "}\n\n"
        f"impl From<&str> for {name} {{\n"
        "    fn from(v: &str) -> Self {\n"
        "        match v {\n" + "\n".join(parse_arms) + "\n"
        "            other => Self::Other(other.to_string()),\n"
        "        }\n"
        "    }\n"
        "}\n\n"
        f"impl From<String> for {name} {{\n"
        "    fn from(v: String) -> Self {\n"
        "        Self::from(v.as_str())\n"
        "    }\n"
        "}\n\n"
        f"impl Default for {name} {{\n"
        "    fn default() -> Self {\n"
        "        Self::Other(String::new())\n"
        "    }\n"
        "}\n\n"
        f"impl std::str::FromStr for {name} {{\n"
        "    type Err = std::convert::Infallible;\n"
        "    fn from_str(v: &str) -> Result<Self, Self::Err> {\n"
        "        Ok(Self::from(v))\n"
        "    }\n"
        "}\n\n"
        f"/// Every `{name}` value in this contract snapshot.\n"
        f"pub const {plural}: &[&str] = &[{consts}];\n"
    )


def gen_enums(contract: dict, header: str) -> str:
    out = [header]
    for key, (name, doc) in ENUM_NAMES.items():
        values = contract["enums"].get(key)
        if not values:
            raise SystemExit(f"enum {key} missing from contract.json")
        out.append(gen_enum(name, values, doc))
    for name, (values, doc) in LOCAL_ENUMS.items():
        out.append(gen_enum(name, values, doc))
    events = ", ".join(f'"{v}"' for v in contract["event_types"])
    out.append(
        "/// Webhook event types: `invoice.<status>`, `payout.<status>`, `wallet.paid`.\n"
        f"pub const EVENT_TYPES: &[&str] = &[{events}];\n"
    )
    codes = ", ".join(f'"{c}"' for c in contract["error_codes"])
    out.append(
        "/// Every error code the core source can emit (`family.reason`).\n"
        f"pub const ERROR_CODES: &[&str] = &[{codes}];\n"
    )
    return "\n".join(out)


# --- requests.rs ---------------------------------------------------------------------------


def rust_type(schema: dict, route: str, field: str, prefix: str, struct_base: str, nested: list):
    kind = schema.get("type")
    if kind == "string":
        enum_name = ROUTE_FIELD_ENUMS.get(f"{route}#{field}") or FIELD_ENUMS.get(field)
        if enum_name:
            return enum_name
        if MONEY_FIELD.search(field or ""):
            return "Money"
        return "String"
    if kind == "integer":
        return "i64"
    if kind == "number":
        return "f64"
    if kind == "boolean":
        return "bool"
    if kind == "array":
        item = schema.get("items") or {}
        if item.get("type") == "object" and item.get("properties"):
            name = f"{struct_base}{pascal(field)}Item"
            nested.append((name, item, route, f"{prefix}{field}."))
            return f"Vec<{name}>"
        inner = rust_type(item, route, field, prefix, struct_base, nested)
        return f"Vec<{inner}>"
    if kind == "object":
        return "serde_json::Value"
    return "serde_json::Value"


def gen_struct(
    name: str,
    schema: dict,
    route: str,
    prefix: str,
    descriptions: dict,
    doc: str,
    missing: list,
) -> tuple[str, list]:
    required = set(schema.get("required") or [])
    for f in REQUIRED_OVERRIDES.get(route, []):
        if f.startswith(prefix) and "." not in f[len(prefix) :]:
            required.add(f[len(prefix) :])
    for f in OPTIONAL_OVERRIDES.get(route, []):
        if f.startswith(prefix) and "." not in f[len(prefix) :]:
            required.discard(f[len(prefix) :])
    nested: list = []
    struct_base = name[: -len("Request")] if name.endswith("Request") else name
    fields = []
    for prop in sorted((schema.get("properties") or {}).keys()):
        p = schema["properties"][prop]
        ty = rust_type(p, route, prop, prefix, struct_base, nested)
        key = f"{prefix}{prop}"
        text = (descriptions.get("request", {}).get(route, {}) or {}).get(key)
        if not text and p.get("description"):
            missing.append(f"{route}#{key}")
        example = p.get("example")
        doc_text = text or ""
        if example is not None:
            doc_text = (doc_text + " " if doc_text else "") + f"Example: `{json.dumps(example)}`."
        block = doc_lines(doc_text, "    ") if doc_text else ""
        ident = f"r#{prop}" if prop in RUST_KEYWORDS else prop
        if prop in required:
            fields.append(f"{block}    pub {ident}: {ty},")
        else:
            fields.append(
                f"{block}"
                '    #[serde(default, skip_serializing_if = "Option::is_none")]\n'
                f"    pub {ident}: Option<{ty}>,"
            )
    body = "\n".join(fields) if fields else ""
    out = (
        f"{doc_lines(doc)}"
        "#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]\n"
        f"pub struct {name} {{\n" + (body + "\n" if body else "") + "}\n"
    )
    return out, nested


def gen_requests(contract: dict, descriptions: dict, header: str) -> tuple[str, list, int]:
    routes = sorted(
        (r for r in contract["routes"] if not r["path"].startswith(SKIP_PREFIXES)),
        key=lambda r: (r["path"], r["method"]),
    )
    missing: list = []
    out = [header]
    out.append(
        "#![allow(clippy::struct_excessive_bools)]\n"
        "use super::enums::*;\n"
        "use crate::contract::models::Money;\n"
    )
    seen = {}
    count = 0
    for r in routes:
        key = route_key(r)
        schema = r.get("request_schema") or REQUEST_OVERRIDES.get(key)
        if not schema:
            continue
        name = request_struct_name(r["path"])
        if name in seen:
            raise SystemExit(f"request struct name collision: {name} ({seen[name]} and {key})")
        seen[name] = key
        pending = [(name, schema, key, "", f"Request body of `{key}`.")]
        while pending:
            sname, sschema, sroute, sprefix, sdoc = pending.pop(0)
            text, nested = gen_struct(
                sname, sschema, sroute, sprefix, descriptions, sdoc, missing
            )
            out.append(text)
            for nname, nschema, nroute, nprefix in nested:
                pending.append(
                    (nname, nschema, nroute, nprefix, f"Item of `{sname}::{nprefix[:-1]}`.")
                )
        count += 1
    return "\n".join(out), missing, count


# --- version.rs ----------------------------------------------------------------------------


def gen_version(contract: dict, raw: bytes, header: str) -> tuple[str, str]:
    digest = hashlib.sha256(raw).hexdigest()
    return (
        f"{header}\n"
        "/// Commit of the core the contract snapshot was exported from.\n"
        f'pub const CONTRACT_CORE_COMMIT: &str = "{contract["core_commit"]}";\n\n'
        "/// When the snapshot was exported.\n"
        f'pub const CONTRACT_EXPORTED_AT: &str = "{contract["exported_at"]}";\n\n'
        "/// SHA-256 of `contract/contract.json`.\n"
        f'pub const CONTRACT_HASH: &str = "{digest}";\n'
    ), digest


def rustfmt(paths: list) -> None:
    exe = shutil.which("rustfmt")
    if not exe:
        print("codegen: rustfmt not found, leaving generated files unformatted", file=sys.stderr)
        return
    subprocess.run([exe, "--edition", "2021", *paths], check=True)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="fail when the committed files drift")
    args = ap.parse_args()

    contract, descriptions, raw = load()
    header = HEADER.format(commit=contract["core_commit"][:12])
    routes_rs, route_count = gen_routes(contract, header)
    enums_rs = gen_enums(contract, header)
    requests_rs, missing, request_count = gen_requests(contract, descriptions, header)
    version_rs, digest = gen_version(contract, raw, header)
    files = {
        "routes.rs": routes_rs,
        "enums.rs": enums_rs,
        "requests.rs": requests_rs,
        "version.rs": version_rs,
    }

    target = tempfile.mkdtemp() if args.check else OUT_DIR
    os.makedirs(target, exist_ok=True)
    for name, text in files.items():
        with open(os.path.join(target, name), "w", encoding="utf-8") as fh:
            fh.write(text)
    rustfmt([os.path.join(target, n) for n in GENERATED])

    if args.check:
        drifted = []
        for name in GENERATED:
            committed = os.path.join(OUT_DIR, name)
            fresh = os.path.join(target, name)
            a = open(committed, encoding="utf-8").read() if os.path.exists(committed) else ""
            b = open(fresh, encoding="utf-8").read()
            if a != b:
                drifted.append(name)
        shutil.rmtree(target)
        if drifted:
            print(
                "contract drift: "
                + ", ".join(drifted)
                + " differ from contract/contract.json - run python3 scripts/codegen.py",
                file=sys.stderr,
            )
            return 1
        print(f"check-drift: {len(GENERATED)} generated files are in sync ({digest[:12]})")
        return 0

    if missing:
        print(
            f"codegen: {len(missing)} request fields lack an English description:\n  "
            + "\n  ".join(missing),
            file=sys.stderr,
        )
    print(
        f"codegen: {route_count} routes, {request_count} request bodies, "
        f"{len(contract['error_codes'])} error codes, contract {digest[:12]}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
