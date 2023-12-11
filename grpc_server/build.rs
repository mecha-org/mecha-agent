/// Builds a GRPC Server compiling the proto files in ./proto folder
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        // .out_dir("src/")
        // adding attributes
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .compile(
            &[
                "proto/provisioning.proto",
                "proto/identity.proto",
                "proto/messaging.proto",
                "proto/settings.proto",
                "proto/metrics.proto",
                "proto/logs.proto",
                "proto/trace.proto",
            ],
            &["."],
        )
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));

    Ok(())
}
