fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provisioning_client = "./proto/provisioning.proto";
    let identity_client = "./proto/identity.proto";
    let settings_client = "./proto/settings.proto";

    tonic_build::configure()
        .build_server(true)
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .compile(
            &[provisioning_client, identity_client, settings_client],
            &[".proto"],
        )
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));

    Ok(())
}
