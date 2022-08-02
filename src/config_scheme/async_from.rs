use async_trait::async_trait;

#[async_trait]
pub trait AsyncFrom<T> {
    async fn from(_: T) -> Self;
}
