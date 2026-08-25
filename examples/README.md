# Examples

| example                | what it shows                                                                 |
| ---------------------- | ----------------------------------------------------------------------------- |
| `accept_payment.rs`    | create an invoice, read the pay page and address, watch it settle             |
| `payout.rs`            | quote a payout, dry-run it, send it, and handle a retryable refusal           |
| `webhook_receiver.rs`  | verify a delivery over the raw bytes, deduplicate it, drop stale events       |

Run one against the sandbox (a `test_` key is both key kinds at once):

```sh
export OBLODAI_PUBLIC_ID=pk_test_… OBLODAI_SECRET=…
cargo run --example accept_payment
cargo run --example payout

OBLODAI_WEBHOOK_SECRET=whsec_… cargo run --example webhook_receiver
```

Against a local gateway, add `OBLODAI_BASE_URL=http://127.0.0.1:8095`.
