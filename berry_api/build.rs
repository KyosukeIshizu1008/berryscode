fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[
                "../proto/berry_api.proto",
                "../proto/lsp_service.proto",
            ],
            &["../proto"],
        )?;
    Ok(())
}
