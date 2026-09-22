use std::convert::Infallible;

use tower::util::BoxCloneSyncService;
use tower::ServiceExt;

pub struct Next<Transaction, Disposition> {
    pub(super) inner: BoxCloneSyncService<Transaction, Disposition, Infallible>,
}

impl<Transaction, Disposition> Next<Transaction, Disposition>
where
    Transaction: Send + 'static,
    Disposition: Send + 'static,
{
    pub async fn forward(self, transaction: Transaction) -> Disposition {
        match ServiceExt::oneshot(self.inner, transaction).await {
            Ok(disposition) => disposition,
            Err(e) => match e {},
        }
    }
}
