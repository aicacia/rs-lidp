use std::pin::Pin;

pub async fn run_transaction<T, F>(
    connection: &libsql::Connection,
    transaction_fn: F,
) -> libsql::Result<T>
where
    F: for<'a> FnOnce(
        &'a mut libsql::Transaction,
    ) -> Pin<Box<dyn Send + Future<Output = libsql::Result<T>> + 'a>>,
{
    let mut transaction = connection.transaction().await?;
    let result = match transaction_fn(&mut transaction).await {
        Ok(result) => result,
        Err(e) => match transaction.rollback().await {
            Ok(_) => return Err(e),
            Err(e2) => {
                log::error!("failed to rollback transaction: {}", e2);
                return Err(e);
            }
        },
    };
    transaction.commit().await?;
    Ok(result)
}
