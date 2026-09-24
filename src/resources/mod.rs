//! The method tree: generated namespaces (`crate::generated::resources`) over the builders here.

pub mod base;

pub use base::{
    from_json, Call, Decode, FileResult, ItemStream, PageStream, Pager, RawApiResponse, Request,
    WithRawResponse,
};
#[cfg(feature = "blocking")]
pub use base::{BlockingItems, BlockingPages};
