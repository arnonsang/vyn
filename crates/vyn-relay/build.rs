fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    println!("cargo:rerun-if-changed=proto/vyn.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/vyn.proto"], &["proto"])
        .expect("failed to compile protobuf definitions");
}
