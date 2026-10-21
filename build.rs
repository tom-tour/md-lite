fn main() {
    println!("cargo:rerun-if-changed=md-lite.rc");
    println!("cargo:rerun-if-changed=md-lite.ico");
    println!("cargo:rerun-if-changed=md-lite.manifest");
    println!("cargo:rerun-if-changed=md-lite.html");
    // Icône, manifeste et informations de version : uniquement pour l'exe Windows.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("md-lite.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
