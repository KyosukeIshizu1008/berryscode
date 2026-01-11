fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile Tauri resources
    tauri_build::build();

    // Compile protobuf files for gRPC client
    tonic_build::configure()
        .build_server(false)  // We only need the client
        .build_client(true)
        .compile_protos(&["proto/berry_api.proto"], &["proto"])?;

    // Compile LLM service proto
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/llm_service.proto"], &["proto"])?;

    Ok(())
}
