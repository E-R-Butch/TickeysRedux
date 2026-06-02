fn main() {
    // arm64 port: no native dylib dependencies.
    // Audio via rodio (CoreAudio backend).
    // SMAppService needs ServiceManagement framework.
    println!("cargo:rustc-link-lib=framework=ServiceManagement");
}
