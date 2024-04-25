use anyhow::Result;
use messaging::handler::MessagingMessage;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

use crate::agent::messaging_service_server::MessagingService;
use crate::agent::SendMessageRequest;
use crate::agent::SendMessageResponse;

#[derive(Debug)]
pub struct MessagingServiceHandler {
    messaging_tx: mpsc::Sender<MessagingMessage>,
}

impl MessagingServiceHandler {
    // Add an opening brace here
    pub fn new(messaging_tx: mpsc::Sender<MessagingMessage>) -> Self {
        Self { messaging_tx }
    }
}

#[tonic::async_trait]
impl MessagingService for MessagingServiceHandler {
    async fn publish(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let messaging_tx = self.messaging_tx.clone();
        let message_request = request.into_inner().clone();
        // send message
        let (tx, rx) = oneshot::channel();
        let _ = messaging_tx
            .send(MessagingMessage::Send {
                reply_to: tx,
                message: message_request.message,
                subject: message_request.subject,
                headers: None,
            })
            .await;

        // TODO handle
        let reply = rx.await.unwrap_or(Err(
            Status::unavailable("messaging service unavailable").into()
        ));

        if reply.is_ok() {
            let status = reply.unwrap();
            Ok(Response::new(SendMessageResponse { status }))
        } else {
            Err(Status::from_error(reply.unwrap_err().into()))
        }
    }
}
