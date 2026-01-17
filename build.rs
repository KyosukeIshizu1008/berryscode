fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false) // Client only
        .build_client(true)
        .compile_protos(
            &["proto/berry_api.proto", "proto/lsp_service.proto"],
            &["proto/"],
        )?;
    Ok(())
}
