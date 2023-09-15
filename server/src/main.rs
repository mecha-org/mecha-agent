use dotenv::dotenv;
use routes::provisioning::{generate_provisioning_code, manifest_handler};
use routes::telemetry::{send_system_logs, send_system_metrics, send_user_logs, send_user_metrics};
use serde::{Deserialize, Serialize};
use std::thread;
use telemetry::messaging::service::MessagingService;
use telemetry::service::TelemetryService;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};
pub mod routes;
pub mod server_settings;
pub mod settings;

pub mod ServiceAgent {
    tonic::include_proto!("provisioning");
}

pub mod metrics {
    tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
}

pub mod trace {
    tonic::include_proto!("opentelemetry.proto.collector.trace.v1");
}

pub mod logs {
    tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
}

use metrics::{
    metrics_service_server::{MetricsService, MetricsServiceServer},
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
};

use logs::{
    logs_service_server::{LogsService, LogsServiceServer},
    ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use trace::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use ServiceAgent::provisioning_handler_server::{ProvisioningHandler, ProvisioningHandlerServer};

use ServiceAgent::{Empty, ProvisioningCodeResponse};

#[derive(Debug, Default)]
pub struct ProvisioningAgent {}

#[tonic::async_trait]
impl ProvisioningHandler for ProvisioningAgent {
    async fn generate_provisioning_code(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let response = match generate_provisioning_code().await {
            Ok(res) => ProvisioningCodeResponse { value: res },
            Err(e) => {
                return Err(Status::new(e.code, e.message));
            }
        };

        Ok(Response::new(response))
    }
    async fn manifest_handler(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let response = match manifest_handler().await {
            Ok(res) => ProvisioningCodeResponse { value: res },
            Err(e) => {
                return Err(Status::new(e.code, e.message));
            }
        };

        Ok(Response::new(response))
    }
}

// Telemetry Service

// Metrics

#[derive(Clone)]
pub struct MetricsAgent {
    telemetry_service: TelemetryService,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncodeData {
    encoded: Vec<u8>,
    user_type: String,
}

#[tonic::async_trait]
impl MetricsService for MetricsAgent {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let metrics_type = binding.get("user").unwrap().to_str().unwrap();
        let mut metrics = request.into_inner().clone().resource_metrics;
        let encoded: Vec<u8> = bincode::serialize(&metrics).unwrap();
        let content = serde_json::to_string(&EncodeData {
            encoded: encoded,
            user_type: metrics_type.to_string(),
        })
        .unwrap();

        match metrics_type {
            "User" => {
                let _ = send_user_metrics(content, self.telemetry_service.clone()).await;
            }
            "System" => {
                let _ = send_system_metrics(content, self.telemetry_service.clone()).await;
            }
            _ => {
                println!("all value ")
            }
        }

        let reply = ExportMetricsServiceResponse {};
        Ok(Response::new(reply))
    }
}

//Logs

#[derive(Clone)]
pub struct LogsAgent {
    telemetry_service: TelemetryService,
}

#[tonic::async_trait]
impl LogsService for LogsAgent {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let logs_type = binding.get("user").unwrap().to_str().unwrap();
        let mut logs = request.into_inner().clone().resource_logs;
        let encoded: Vec<u8> = bincode::serialize(&logs).unwrap();
        let content = serde_json::to_string(&EncodeData {
            encoded: encoded,
            user_type: logs_type.to_string(),
        })
        .unwrap();

        match logs_type {
            "User" => {
                let _ = send_user_logs(content, self.telemetry_service.clone()).await;
            }
            "System" => {
                let _ = send_system_logs(content, self.telemetry_service.clone()).await;
            }
            _ => {
                println!("all value ")
            }
        }

        let reply = ExportLogsServiceResponse {
            partial_success: None,
        };
        Ok(Response::new(reply))
    }
}

//Trace

#[derive(Clone)]
pub struct TraceAgent {
    telemetry_service: TelemetryService,
}

#[tonic::async_trait]
impl TraceService for TraceAgent {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let logs_type = binding.get("user").unwrap().to_str().unwrap();
        let mut logs = request.into_inner().clone().resource_spans;
        let encoded: Vec<u8> = bincode::serialize(&logs).unwrap();
        let content = serde_json::to_string(&EncodeData {
            encoded: encoded,
            user_type: logs_type.to_string(),
        })
        .unwrap();

        match logs_type {
            "User" => {
                let _ = send_user_logs(content, self.telemetry_service.clone()).await;
            }
            "System" => {
                let _ = send_system_logs(content, self.telemetry_service.clone()).await;
            }
            _ => {
                println!("all value ")
            }
        }

        let reply = ExportTraceServiceResponse {
            partial_success: None,
        };
        Ok(Response::new(reply))
    }
}

// provising service
#[derive(Debug, Deserialize, Serialize)]
struct Service {
    provision_request: String,
    cert_sign: String,
    health: String,
}

pub async fn server_initialize(telemetry_service: TelemetryService) -> Result<(), String> {
    dotenv().ok();
    let server_settings = match settings::read_yml() {
        Ok(v) => v.server,
        Err(_e) => return Err("Server initilize failed".to_string()),
    };
    let addr = format!("{}:{}", server_settings.url, server_settings.port)
        .parse()
        .unwrap();
    let provisioning_server = ProvisioningAgent::default();
    let metric_server = MetricsAgent {
        telemetry_service: telemetry_service.to_owned(),
    };
    let logs_server = LogsAgent {
        telemetry_service: telemetry_service.to_owned(),
    };

    let trace_server = TraceAgent {
        telemetry_service: telemetry_service.to_owned(),
    };

    tracing::info!(message = "Starting server.", %addr);

    let _ = Server::builder()
        .trace_fn(|_| tracing::info_span!("helloworld_server"))
        .add_service(ProvisioningHandlerServer::new(provisioning_server))
        .add_service(MetricsServiceServer::new(metric_server))
        .add_service(LogsServiceServer::new(logs_server))
        .add_service(TraceServiceServer::new(trace_server))
        .serve(addr)
        .await;
    Ok(())
}

pub fn telemetry_start(messaging_service: MessagingService) -> Option<TelemetryService> {
    println!("telemetry server");
    dotenv().ok();
    let server_settings = match settings::read_yml() {
        Ok(v) => Some(v.telemetry),
        Err(_e) => None,
    };
    if server_settings.is_some() {
        let settings = server_settings.unwrap();
        let telemetry_service: TelemetryService =
            TelemetryService::new(settings.clone(), messaging_service);
        if settings.enabled {
            telemetry_service.clone().init();
        }
        return Some(telemetry_service);
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv().ok();
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let message_settings = match settings::read_yml() {
        Ok(v) => v.messaging,
        Err(_e) => return Err("Messaging initilize failed".to_string()),
    };
    let messaging_service = MessagingService::new(message_settings);
    let telemetry_service = match telemetry_start(messaging_service) {
        Some(v) => v,
        None => return Err("Telemetry initilize failed".to_string()),
    };
    // Create a Tokio runtime for async-await
    let rt = Runtime::new().unwrap();

    // Spawn threads
    let t1 = thread::spawn(move || {
        let result = rt.block_on(server_initialize(telemetry_service));
        if let Err(err) = result {
            println!("Error in server_initialize: {}", err);
        }
    });

    // Wait for both threads to finish
    t1.join().unwrap();
    Ok(())
}
