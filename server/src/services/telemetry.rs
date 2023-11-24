use crate::metrics::{
    metrics_service_server::MetricsService, ExportMetricsServiceRequest,
    ExportMetricsServiceResponse,
};
use messaging::service::Messaging;
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

    TelemetryService::new(settings).await
}

// Metrics
#[derive(Clone)]
pub struct TelemetryMetricsHandler {
    pub messaging_client: Messaging,
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
        let metrics_encoded: Vec<u8> = bincode::serialize(&metrics).unwrap();
        let _ = match telemetry_service
            .user_metrics(metrics_encoded, metrics_type, self.messaging_client.clone())
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

        let _ = match telemetry_service
            .user_logs(encoded, logs_type, self.messaging_client.clone())
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
        let _ = match telemetry_service
            .user_trace(encoded, trace_type, self.messaging_client.clone())
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
