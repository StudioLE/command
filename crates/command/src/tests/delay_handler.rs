use crate::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
pub struct DelayHandler;

impl Service for DelayHandler {
    type Error = Infallible;

    async fn from_services(_services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self)
    }
}

#[async_trait]
impl Execute<DelayRequest, (), Infallible> for DelayHandler {
    async fn execute(&self, request: &DelayRequest) -> Result<(), Infallible> {
        debug!(%request, "Executing {} ms delay", request.duration);
        sleep(Duration::from_millis(request.duration)).await;
        Ok(())
    }
}
