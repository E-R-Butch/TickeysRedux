fn main() {
    // arm64 port: no native dylib dependencies.
    // Audio via rodio (CoreAudio backend).
    // SMAppService is resolved dynamically through the Objective-C runtime.
    // Force an LC_LOAD_DYLIB entry even though no C symbol is referenced, or
    // ld may dead-strip ServiceManagement and AnyClass::get will return None.
    println!("cargo:rustc-link-arg=-Wl,-needed_framework,ServiceManagement");
}
