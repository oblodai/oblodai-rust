//! Generated PDF/CSV documents.
//!
//! The file-returning methods answer with [`FileResult`](crate::resources::FileResult) — the bytes,
//! the content type and the filename. `create_job` and `job_info` are ordinary JSON routes and
//! return [`DocumentJob`]; only `job_file` hands back the finished bytes. Payment key, except
//! `download`, which is public.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use super::files::FileBuilder;
use crate::contract::models::DocumentJob;
use crate::contract::requests::DocumentsJobsRequest;
use crate::contract::routes;

/// File format, where the document offers a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentFormat {
    /// Rendered, paginated, signed.
    Pdf,
    /// Raw rows — cheaper for a long statement and it opens in a spreadsheet.
    Csv,
}

/// `lang` is a 2-letter code (41 supported).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct DocumentQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

/// A document that also offers CSV.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct FormatQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<DocumentFormat>,
}

/// The query of a signed public document link: the `exp` and `sig` that came with a
/// `document_url`, plus the optional language. Mirrors the reference SDK's single query object.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct SignedLinkQuery {
    /// Expiry, as the `document_url` carried it.
    pub exp: i64,
    /// Signature, as the `document_url` carried it.
    pub sig: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

impl SignedLinkQuery {
    pub fn new(exp: i64, sig: impl Into<String>) -> Self {
        Self {
            exp,
            sig: sig.into(),
            lang: None,
        }
    }

    /// Render the document in this 2-letter language.
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }
}

/// A document over a date range. `from`/`to` are `YYYY-MM-DD`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct PeriodQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<DocumentFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// The `documents` namespace.
#[derive(Clone, Debug)]
pub struct Documents<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Documents<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/documents/jobs` — queue a large report; poll `job_info`, then `job_file`.
    pub fn create_job(&self, params: DocumentsJobsRequest) -> RequestBuilder<Tr, DocumentJob> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_DOCUMENTS_JOBS,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/documents/jobs/info`.
    pub fn job_info(&self, job_id: impl Into<String>) -> RequestBuilder<Tr, DocumentJob> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_DOCUMENTS_JOBS_INFO,
            Call::new().json(json!({ "job_id": job_id.into() })).done(),
        )
    }

    /// `GET /v1/documents/jobs/file` — the finished job's bytes.
    pub fn job_file(&self, job_id: impl Into<String>) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_JOBS_FILE,
            Call::new().query("job_id", job_id).done(),
        )
    }

    /// `GET /v1/documents/statement` — account statement for a period (PDF or CSV).
    pub fn statement(&self, query: PeriodQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_STATEMENT,
            Call::new().query_of(&query).done(),
        )
    }

    /// `GET /v1/documents/balance` — balance certificate (PDF).
    pub fn balance_certificate(&self, query: DocumentQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_BALANCE,
            Call::new().query_of(&query).done(),
        )
    }

    /// `GET /v1/documents/fees` — the fee schedule in force for the merchant (PDF).
    pub fn fee_schedule(&self, query: DocumentQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_FEES,
            Call::new().query_of(&query).done(),
        )
    }

    /// `GET /v1/documents/ledger` — full ledger export for a period (PDF or CSV).
    pub fn ledger(&self, query: PeriodQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_LEDGER,
            Call::new().query_of(&query).done(),
        )
    }

    /// `GET /v1/documents/split` — how one payment was split between partners (PDF).
    pub fn split_report(
        &self,
        payment_uuid: impl Into<String>,
        query: DocumentQuery,
    ) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_SPLIT,
            Call::new()
                .query_of(&query)
                .query("uuid", payment_uuid)
                .done(),
        )
    }

    /// `GET /v1/documents/batch` — per-row report of an asynchronous batch.
    pub fn batch_report(&self, batch_id: impl Into<String>, query: FormatQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_BATCH,
            Call::new().query_of(&query).query("uuid", batch_id).done(),
        )
    }

    /// `GET /v1/documents/link` — payment-link report (its invoices).
    pub fn link_report(&self, link_id: impl Into<String>, query: FormatQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_LINK,
            Call::new().query_of(&query).query("uuid", link_id).done(),
        )
    }

    /// `GET /v1/documents/wallet/statement` — static-wallet statement.
    pub fn wallet_statement(
        &self,
        wallet_uuid: impl Into<String>,
        query: PeriodQuery,
    ) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_WALLET_STATEMENT,
            Call::new()
                .query_of(&query)
                .query("uuid", wallet_uuid)
                .done(),
        )
    }

    /// `GET /v1/documents/referrals` — referral earnings report.
    pub fn referrals_report(&self, query: PeriodQuery) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_REFERRALS,
            Call::new().query_of(&query).done(),
        )
    }

    /// `GET /v1/documents/{kind}/{id}` — a public document by its signed link (`exp` and `sig`
    /// come from a `document_url`). No credentials needed; prefer fetching `document_url` directly.
    ///
    /// ```no_run
    /// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
    /// use oblodai::resources::documents::SignedLinkQuery;
    /// let pdf = client
    ///     .documents()
    ///     .download("payout", "abc", SignedLinkQuery::new(1_800_000_000, "deadbeef").lang("en"))
    ///     .await?;
    /// # let _ = pdf; Ok(()) }
    /// ```
    ///
    /// Codes to branch on: `document.link_expired`, `request.bad_id`, `request.not_found`.
    pub fn download(
        &self,
        kind: impl Into<String>,
        id: impl Into<String>,
        query: SignedLinkQuery,
    ) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_DOCUMENTS_KIND_ID,
            Call::new()
                .path("kind", kind)
                .path("id", id)
                .query_of(&query)
                .done(),
        )
    }
}
