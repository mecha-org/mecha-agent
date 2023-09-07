fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        // .out_dir("src/")
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize,)]")
        // .type_attribute("", "#[derive(serde::Deserialize, serde::Serialize)]") // adding attributes
        // adding attributes
        .compile(&["proto/provisioning.proto", "proto/metrics.proto"], &["."])
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));

    Ok(())
}
