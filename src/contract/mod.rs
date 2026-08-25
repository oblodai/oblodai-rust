//! The gateway's contract: routes, enums, request bodies and response models.
//!
//! `enums`, `requests`, `routes` and `version` are generated from `contract/contract.json` by
//! `scripts/codegen.py`; `models` and `types` are hand-written and verified against the golden
//! response bodies in `contract/fixtures/`.

pub mod enums;
pub mod models;
pub mod requests;
pub mod routes;
pub mod types;
pub mod version;
