fn main() {
    prost_build::compile_protos(&["src/models.proto"], &["src/"]).unwrap();
}
