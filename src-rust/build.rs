fn main() {
    // Windows 资源文件（图标/版本信息），macOS 不需要
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-arg=app_res.o");
}
