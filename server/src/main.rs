use axum::http::StatusCode;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use dotenv::dotenv;
use futures::executor::block_on;
use futures::join;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use routes::provisioning::generate_provisioning_code;
use sentry_tracing::{self, EventFilter};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{env, thread};
use tonic::Code;
use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter};
use url::Url;

pub mod errors;
pub mod routes;
pub mod server;
pub mod server_settings;
pub mod settings;

pub mod agent {
    tonic::include_proto!("provisioning");
}

use agent::provisioning_handler_server::{ProvisioningHandler, ProvisioningHandlerServer};
use agent::{Empty, ProvisioningCode, ProvisioningCodeResponse, RequestManifestResponse};

use crate::routes::provisioning::request_manifest_handler;

#[derive(Debug, Default)]
pub struct ProvisioningAgent {}

#[tonic::async_trait]
impl ProvisioningHandler for ProvisioningAgent {
    async fn generate_provisioning_code(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let response = match generate_provisioning_code().await {
            Ok(res) => ProvisioningCodeResponse { value: res.1 },
            Err(e) => {
                return Err(Status::new(e.code, e.message));
            }
        };

        Ok(Response::new(response))
    }

    async fn manifest_request(
        &self,
        request: Request<ProvisioningCode>,
    ) -> Result<Response<RequestManifestResponse>, Status> {
        let response = match request_manifest_handler(request.into_inner().code).await {
            Ok(res) => RequestManifestResponse { success: res.1 },
            Err(e) => {
                return Err(Status::new(e.code, e.message));
            }
        };
        Ok(Response::new(response))
    }
}

// Telemetry Service

// Metrics
#[derive(Debug, Default)]
pub struct MetricsAgent {}

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

pub async fn server_initialize() -> Result<(), String> {
    dotenv().ok();
    let server_settings = match settings::read_yml() {
        Ok(v) => v.server,
        Err(_e) => return Err("Server initilize failed".to_string()),
    };
    let addr = format!("{}:{}", server_settings.url, server_settings.port)
        .parse()
        .unwrap();
    let provisioning_server = ProvisioningAgent::default();
    let metric_server = MetricsAgent::default();

    println!("Server listening on {}", addr);

    let _ = Server::builder()
        .add_service(ProvisioningHandlerServer::new(provisioning_server))
        .serve(addr)
        .await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv().ok();
    let sentry_path = match env::var("SENTRY_PATH") {
        Ok(v) => v,
        Err(_e) => "".to_string(),
    };

    let _guard = sentry::init((
        sentry_path,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            trim_backtraces: true,
            ..Default::default()
        },
    ));

    let subscriber = tracing_subscriber::registry()
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
        .with(build_loglevel_filter_layer()) //temp for terminal log
        .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap()); // trace collection layer
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing::info!(
        //sample log
        task = "tracing_setup",
        result = "success",
        "tracing successfully set up",
    );
    let t1 = thread::spawn(server_initialize);

    block_on(async { join!(t1.join().unwrap()) });
    Ok(())
}
