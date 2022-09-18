fn main() -> Result<(), std::io::Error> {
    tonic_build::compile_protos("proto/playground.proto")
}
