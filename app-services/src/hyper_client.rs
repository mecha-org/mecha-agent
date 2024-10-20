use std::collections::HashMap;
use tokio::time::{timeout, Duration};

use anyhow::{bail, Result};
use http_body_util::{Empty, Full};
use hyper::body::Bytes;
use hyper::Response;
use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, Request, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::errors::{AppServicesError, AppServicesErrorCodes};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub async fn make_request_without_body(
    remote_host_addr: String,
    url: Uri,
    headers: HashMap<String, String>,
) -> Result<Response<hyper::body::Incoming>> {
    let fn_name = "make_request_without_body";
    // Open a TCP connection to the remote host
    let duration = Duration::from_secs(5);
    let stream = match timeout(duration, TcpStream::connect(remote_host_addr)).await {
        Ok(Ok(tcp_stream)) => tcp_stream,
        Ok(Err(e)) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(e)
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(e)
        }
    };

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error polling connection, error -  {:?}",
                err
            );
        }
    });

    // Convert HashMap to HeaderMap
    let request_headers: HeaderMap = hashmap_to_header_map(headers)?;
    println!("request_headers: {:?}", request_headers);

    let empty_body = Empty::<Bytes>::new();
    // Create an HTTP request with an empty body and a HOST header
    let mut req: Request<_> = Request::builder().uri(url).body(empty_body)?; //TODO: Make body optional

    //TODO: figure out the issue about the accept-encoding header
    for (key, value) in request_headers.iter() {
        if key == "accept-encoding" {
            continue;
        }
        req.headers_mut().insert(key, value.clone());
    }

    match sender.send_request(req).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            println!("error sending request, error time elapsed -  {:?}", e);
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(e)
        }
    }
}

pub async fn make_request_with_body(
    remote_host_addr: String,
    url: Uri,
    req_headers: HashMap<String, String>,
    req_method: String,
    req_body: Bytes,
) -> Result<Response<hyper::body::Incoming>> {
    let fn_name = "make_request_with_body";
    info!(func = fn_name, package = PACKAGE_NAME, "url: {:?}", url);

    // Open a TCP connection to the remote host
    let duration = Duration::from_secs(5);
    let stream = match timeout(duration, TcpStream::connect(remote_host_addr)).await {
        Ok(Ok(tcp_stream)) => tcp_stream,
        Ok(Err(e)) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(AppServicesError::new(
                AppServicesErrorCodes::TcpStreamConnectError,
                format!("error connecting to remote host, error -  {:?}", e)
            ))
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(AppServicesError::new(
                AppServicesErrorCodes::TcpStreamConnectTimeoutError,
                format!("tcp connection timeout -  {:?}", e)
            ))
        }
    };

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok((sender, conn)) => (sender, conn),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error creating hyper client, error -  {:?}",
                e
            );
            bail!(e)
        }
    };

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error polling connection, error -  {:?}",
                err
            );
        }
    });

    // Convert HashMap to HeaderMap
    let request_headers: HeaderMap = match hashmap_to_header_map(req_headers) {
        Ok(header_map) => header_map,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error converting hashmap to header map, error -  {:?}",
                e
            );
            bail!(e)
        }
    };

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "request_headers: {:?}",
        request_headers
    );

    println!("url to execute api: {:?}", url);
    // Create an HTTP request with an empty body and a HOST header
    let mut req: Request<_> = match Request::builder()
        .uri(url)
        .method(req_method.as_str())
        .body(Full::new(req_body))
    {
        //TODO: Make body optional
        Ok(req) => req,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error creating request, error -  {:?}",
                e
            );
            bail!(e)
        }
    };

    info!(func = fn_name, package = PACKAGE_NAME, "request formed!",);
    //TODO: figure out the issue about the accept-encoding header
    for (key, value) in request_headers.iter() {
        if key == "accept-encoding" {
            continue;
        }
        req.headers_mut().insert(key, value.clone());
    }

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "sending request started",
    );

    // Set the timeout duration
    let duration = Duration::from_secs(5);
    match timeout(duration, sender.send_request(req)).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(e)) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(e)
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending request, error -  {:?}",
                e
            );
            bail!(e)
        }
    }
}

pub fn hashmap_to_header_map(map: HashMap<String, String>) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::new();

    for (key, value) in map {
        // Convert the key to HeaderName
        let header_name = HeaderName::from_bytes(key.as_bytes())?;
        // Convert the value to HeaderValue
        let header_value = HeaderValue::from_str(&value)?;

        // Insert into HeaderMap
        header_map.insert(header_name, header_value);
    }

    Ok(header_map)
}
