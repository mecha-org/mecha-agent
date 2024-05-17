fn main() {
  let provisioning_proto = "./proto/provisioning.proto";
  let identity_proto = "./proto/identity.proto";
  let settings_proto = "./proto/settings.proto";

  tonic_build::configure()
        .build_server(true)
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .compile(
            &[provisioning_proto, identity_proto, settings_proto],
            &[".proto"],
        )
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));


  tauri_build::build()
}
