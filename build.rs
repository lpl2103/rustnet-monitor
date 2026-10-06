fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("ProductName", "RustNet Monitor");
        res.set(
            "FileDescription",
            "RustNet Monitor - Monitor de Conectividade de Rede",
        );
        res.set("CompanyName", "RustNet");
        res.set("LegalCopyright", "Copyright (C) 2026");

        // Se rc.exe não estiver no PATH global, especifica o diretório do Windows SDK
        let sdk_x64 = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64";
        if std::path::Path::new(sdk_x64).join("rc.exe").exists() {
            res.set_toolkit_path(sdk_x64);
        }

        if let Err(e) = res.compile() {
            eprintln!("Warning: Falha ao compilar recurso do Windows: {}", e);
        }
    }
}
