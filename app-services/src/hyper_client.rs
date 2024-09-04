use std::collections::HashMap;

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
use tracing::{debug, error};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub async fn make_request_without_body(
    remote_host_addr: String,
    url: Uri,
    headers: HashMap<String, String>,
) -> Result<Response<hyper::body::Incoming>> {
    let fn_name = "make_get_request";
    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(remote_host_addr).await?;

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
    let fn_name = "make_get_request";
    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(remote_host_addr).await?;

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
    let request_headers: HeaderMap = hashmap_to_header_map(req_headers)?;

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "request_headers: {:?}",
        request_headers
    );

    println!("url to execute api: {:?}", url);
    // Create an HTTP request with an empty body and a HOST header
    let mut req: Request<_> = Request::builder()
        .uri(url)
        .method(req_method.as_str())
        .body(Full::new(req_body))?; //TODO: Make body optional

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
