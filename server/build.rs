fn main() {
    println!("cargo:rerun-if-changed=../proto");
    tonic_build::compile_protos("../proto/fs.proto").unwrap();
}
