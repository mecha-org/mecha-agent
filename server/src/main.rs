use axum::http::StatusCode;
use dotenv::dotenv;
use routes::provisioning::{generate_provisioning_code, manifest_handler};
use routes::telemetry::{send_system_metrics, send_user_metrics};
use serde::{Deserialize, Serialize};
use std::thread;
use telemetry::messaging::service::MessagingService;
use telemetry::service::TelemetryService;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};
pub mod routes;
pub mod server_settings;
pub mod settings;

pub mod agent {
    tonic::include_proto!("provisioning");
    tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
}

use agent::provisioning_handler_server::{ProvisioningHandler, ProvisioningHandlerServer};
use agent::{
    metrics_service_server::{MetricsService, MetricsServiceServer},
    Empty, ExportMetricsServiceRequest, ExportMetricsServiceResponse, ProvisioningCodeResponse,
};

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

#[tonic::async_trait]
impl MetricsService for MetricsAgent {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let binding = request.metadata().clone();
        let mut metrics = request.into_inner().clone().resource_metrics;
        let encoded: Vec<u8> = bincode::serialize(&metrics).unwrap();
        let content = serde_json::to_string(&encoded).unwrap();

        let metrics_type = binding.get("user").unwrap().to_str().unwrap();

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

#[derive(Debug, Deserialize, Serialize)]
struct Service {
    provision_request: String,
    cert_sign: String,
    health: String,
}

#[derive(Debug)]
pub struct ServerError {
    pub code: StatusCode,
    pub message: String,
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
        telemetry_service: telemetry_service,
    };

    println!("Server listening on {}", addr);

    let _ = Server::builder()
        .add_service(ProvisioningHandlerServer::new(provisioning_server))
        .add_service(MetricsServiceServer::new(metric_server))
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
