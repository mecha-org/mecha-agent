use telemetry::handler::TelemetryMessage;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

use metrics::{
    metrics_service_server::MetricsService, ExportMetricsServiceRequest,
    ExportMetricsServiceResponse,
};

use logs::{logs_service_server::LogsService, ExportLogsServiceRequest, ExportLogsServiceResponse};

use crate::logs;
use crate::metrics;
use crate::metrics::metric::Data;

#[derive(Debug, Clone)]
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
    pub telemetry_service_handler: TelemetryServiceHandler,
}

#[tonic::async_trait]
impl LogsService for LogsAgent {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let logs_type = match binding.get("user") {
            Some(v) => v.to_str().unwrap(),
            None => "User",
        };
        let logs = request.into_inner().clone().resource_logs;
        // to print logs value
        /* for met in logs.iter() {
            for key in met.scope_logs.iter() {
                for met_data in key.log_records.iter() {
                    println!("Log: {:?}", met_data.body);
                }
            }
        } */
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

//Metrics
pub struct MetricsAgent {
    pub telemetry_service_handler: TelemetryServiceHandler,
}

#[tonic::async_trait]
impl MetricsService for MetricsAgent {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let metrics_type = match binding.get("user") {
            Some(v) => v.to_str().unwrap(),
            None => "User",
        };
        let metrics = request.into_inner().clone().resource_metrics;
        // to print metrics value
        /*for met in metrics.iter() {
            for key in met.scope_metrics.iter() {
                for met_data in key.metrics.iter() {
                    for data in met_data.data.iter() {
                        match data {
                            Data::Sum(counter) => {
                                for val in counter.data_points.iter() {
                                    println!("Sum: {:?}: {:?}", val.value, val.attributes);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        } */
        let encoded: Vec<u8> = bincode::serialize(&metrics).unwrap();

        let (tx, _rx) = oneshot::channel();
        let _ = self
            .telemetry_service_handler
            .telemetry_tx
            .send(TelemetryMessage::SendMetrics {
                metrics: encoded,
                metrics_type: metrics_type.to_string(),
                reply_to: tx,
            })
            .await;

        let reply = ExportMetricsServiceResponse {};
        Ok(Response::new(reply))
    }
}
