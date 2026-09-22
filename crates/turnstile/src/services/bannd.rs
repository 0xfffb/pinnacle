use pinnacle_core::{Disposition, Next, Reply, Transaction};

use crate::state::TurnstileState;

pub async fn banned(
    state: TurnstileState,
    transaction: Transaction,
    next: Next<Transaction, Disposition>,
) -> Disposition {
    let ip = transaction.meta.get("ip").map(String::as_str).unwrap_or("");
    if state.store.is_banned(ip) {
        return Disposition::respond(Reply::text(403, "store_banned"));
    }
    next.forward(transaction).await
}
