fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "common.proto",
                "sidecar.proto",
                "gateway.proto",
                "control.proto",
            ],
            &["../proto/aegis/v1"],
        )?;
    Ok(())
}
