use crate::metrics::{
    metrics_service_server::MetricsService, ExportMetricsServiceRequest,
    ExportMetricsServiceResponse,
};
use messaging::service::Messaging;
use messaging::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use settings::{read_settings_yml, AgentSettings};
use telemetry::service::TelemetryService;
use tonic::{Code, Request, Response, Status};

use crate::logs::{
    logs_service_server::LogsService, ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use crate::trace::{
    trace_service_server::TraceService, ExportTraceServiceRequest, ExportTraceServiceResponse,
};

async fn new_telemetry_service() -> TelemetryService {
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    TelemetryService::new(settings.telemetry).await
}

// Metrics
#[derive(Clone)]
pub struct TelemetryMetricsHandler {
    pub messaging_client: Messaging,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncodeData {
    encoded: Vec<u8>,
    user_type: String,
}

#[tonic::async_trait]
impl MetricsService for TelemetryMetricsHandler {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let telemetry_service = new_telemetry_service().await;
        let binding = request.metadata().clone();
        let metrics_type = binding.get("user").unwrap().to_str().unwrap();
        let metrics = request.into_inner().clone().resource_metrics;
        let encoded: Vec<u8> = bincode::serialize(&metrics).unwrap();
        let content: String = match serde_json::to_string(&EncodeData {
            encoded: encoded,
            user_type: metrics_type.to_string(),
        }) {
            Ok(k) => k,
            Err(e) => return Err(Status::new(Code::Aborted, format!("{}", e))),
        };
        let _ = match telemetry_service
            .user_metrics(content.into(), self.messaging_client.clone())
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return Err(Status::new(
                    Code::Unknown,
                    format!("Failed to send metrics{}", e),
                ))
            }
        };

        // match metrics_type {
        //     "User" => {
        //         let _ = send_user_metrics(content).await;
        //     }
        //     "System" => {
        //         let _ = send_system_metrics(content).await;
        //     }
        //     _ => {
        //         println!("all value ")
        //     }
        // }

        let reply = ExportMetricsServiceResponse {};
        Ok(Response::new(reply))
    }
}

//Logs

#[derive(Clone)]
pub struct TelemetryLogsHandler {
    pub messaging_client: Messaging,
}

#[tonic::async_trait]
impl LogsService for TelemetryLogsHandler {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let telemetry_service = new_telemetry_service().await;
        let binding = request.metadata().clone();
        let logs_type = binding.get("user").unwrap().to_str().unwrap();
        let logs = request.into_inner().clone().resource_logs;
        let encoded: Vec<u8> = bincode::serialize(&logs).unwrap();
        let content = match serde_json::to_string(&EncodeData {
            encoded: encoded,
            user_type: logs_type.to_string(),
        }) {
            Ok(res) => res,
            Err(e) => return Err(Status::new(Code::Aborted, format!("{}", e))),
        };
        let _ = match telemetry_service
            .user_logs(content.into(), self.messaging_client.clone())
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return Err(Status::new(
                    Code::Unknown,
                    format!("Failed to send logs{}", e),
                ))
            }
        };

        let reply = ExportLogsServiceResponse {
            partial_success: None,
        };
        Ok(Response::new(reply))
    }
}

//Trace

#[derive(Clone)]
pub struct TelemetryTraceHandler {
    pub messaging_client: Messaging,
}

#[tonic::async_trait]
impl TraceService for TelemetryTraceHandler {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let telemetry_service = new_telemetry_service().await;
        let binding = request.metadata().clone();
        let trace_type = binding.get("user").unwrap().to_str().unwrap();
        let trace = request.into_inner().clone().resource_spans;
        let encoded: Vec<u8> = bincode::serialize(&trace).unwrap();
        let content = EncodeData {
            encoded: encoded,
            user_type: trace_type.to_string(),
        };
        let payload_payload_json = json!(content);
        let _ = match telemetry_service
            .user_trace(
                Bytes::from(payload_payload_json.to_string()),
                self.messaging_client.clone(),
            )
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return Err(Status::new(
                    Code::Unknown,
                    format!("Failed to send Trace{}", e),
                ))
            }
        };

        let reply = ExportTraceServiceResponse {
            partial_success: None,
        };
        Ok(Response::new(reply))
    }
}
