fn main() {
    let src = "../wit/plugin.wit";
    println!("cargo:rerun-if-changed={src}");
    if std::path::Path::new(src).exists() {
        std::fs::create_dir_all("wit").unwrap();
        std::fs::copy(src, "wit/plugin.wit").unwrap();
    }
}
