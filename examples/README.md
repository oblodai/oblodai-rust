# Examples

| example                | what it shows                                                                 |
| ---------------------- | ----------------------------------------------------------------------------- |
| `accept_payment.rs`    | create an invoice, read the pay page and address, watch it settle             |
| `payout.rs`            | quote a payout, dry-run it, send it, and handle a retryable refusal           |
| `webhook_receiver.rs`  | verify a delivery over the raw bytes, deduplicate it, drop stale events       |

Each one also runs in `tests/examples.rs` against a fake gateway, so what you copy is code that works.

Run one against the sandbox (one API key signs every route):

```sh
export OBLODAI_PUBLIC_ID=test_oblodai_… OBLODAI_SECRET=oblodai_test_…
cargo run --example accept_payment
cargo run --example payout

OBLODAI_WEBHOOK_SECRET=whsec_… cargo run --example webhook_receiver
```

Against a local gateway, add `OBLODAI_BASE_URL=http://127.0.0.1:8095`.
