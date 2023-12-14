use telemetry::handler::TelemetryMessage;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

use crate::agent::logs_service_server::LogsService;
use crate::agent::ExportLogsServiceRequest;
use crate::agent::ExportLogsServiceResponse;

#[derive(Debug)]
pub struct TelemetryServiceHandler {
    pub telemetry_tx: mpsc::Sender<TelemetryMessage>,
}

impl TelemetryServiceHandler {
    // Add an opening brace here
    pub fn new(telemetry_tx: mpsc::Sender<TelemetryMessage>) -> Self {
        Self { telemetry_tx }
    }
}

//Logs
pub struct LogsAgent {
    // messaging_tx: Sender<MessagingMessage>,
    pub telemetry_service_handler: TelemetryServiceHandler,
}

#[tonic::async_trait]
impl LogsService for LogsAgent {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let logs_type = binding.get("user").unwrap().to_str().unwrap();
        let logs = request.into_inner().clone().resource_logs;
        let encoded: Vec<u8> = bincode::serialize(&logs).unwrap();

        let (tx, _rx) = oneshot::channel();
        let _ = self
            .telemetry_service_handler
            .telemetry_tx
            .send(TelemetryMessage::SendLogs {
                logs: encoded,
                logs_type: logs_type.to_string(),
                reply_to: tx,
            })
            .await;

        let reply = ExportLogsServiceResponse {
            partial_success: None,
        };
        Ok(Response::new(reply))
    }
}
