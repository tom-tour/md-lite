fn main() {
    println!("cargo:rerun-if-changed=md-lite.rc");
    println!("cargo:rerun-if-changed=md-lite.ico");
    println!("cargo:rerun-if-changed=md-lite.manifest");
    println!("cargo:rerun-if-changed=md-lite.html");
    embed_resource::compile("md-lite.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
