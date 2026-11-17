fn main() {
    if std::env::var_os("CARGO_FEATURE_PINYIN_INPUT").is_none() {
        return;
    }
    let source = std::path::Path::new("../../third_party/aosp-pinyin/jni")
        .canonicalize()
        .expect("pinyin source");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("build directory"));
    let mut files: Vec<_> = std::fs::read_dir(source.join("share"))
        .unwrap()
        .map(|f| f.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "cpp"))
        .collect();
    files.sort();
    // Upstream's dictionary serializes size_t. Rebuild it on this 64-bit host
    // for the 64-bit little-endian host/Android targets, never load the old ARM32 blob.
    assert_eq!(std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap(), "64");
    assert_eq!(std::env::var("CARGO_CFG_TARGET_ENDIAN").unwrap(), "little");
    let host = std::env::var("HOST").unwrap();
    let compiler = cc::Build::new().cpp(true).host(&host).target(&host).get_compiler();
    let builder = out.join("pinyin-dictbuilder");
    let mut command = compiler.to_command();
    command
        .args(["-std=c++11", "-O2", "-w", "-D___BUILD_MODEL___"])
        .arg("-I")
        .arg(source.join("include"))
        .arg("-I")
        .arg("native");
    for file in &files {
        command.arg(file);
    }
    command
        .arg(source.join("command/pinyinime_dictbuilder.cpp"))
        .arg("-o")
        .arg(&builder);
    assert!(command.status().expect("compile dictionary builder").success());
    let cwd = out.join("jni/command");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(out.join("res/raw")).unwrap();
    let output = std::process::Command::new(builder)
        .current_dir(cwd)
        .arg(source.join("data/rawdict_utf16_65105_freq.txt"))
        .arg(source.join("data/valid_utf16.txt"))
        .output()
        .expect("generate 64-bit dictionary");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++11")
        .warnings(false)
        .include(source.join("include"))
        .include("native");
    for file in files {
        build.file(file);
    }
    build.file("native/pinyin.cpp");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        build.cpp_link_stdlib(None);
    }
    build.compile("skyward_pinyin");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        let compiler = cc::Build::new().cpp(true).get_compiler();
        let location = compiler.to_command().arg("-print-file-name=libc++_static.a").output().expect("locate NDK C++ runtime");
        assert!(location.status.success());
        let library = std::path::PathBuf::from(String::from_utf8(location.stdout).unwrap().trim());
        assert!(library.is_file(), "NDK static C++ runtime not found: {}", library.display());
        // NDK's common directory also holds libc.a/libm.a/libdl.a. Adding that
        // directory ahead of the API stubs silently turns recovery into a
        // dynamic executable with static Bionic, which crashes before main.
        // Expose only the two C++ archives, leaving Android's C runtime dynamic.
        let runtime = out.join("cxx-runtime");
        std::fs::create_dir_all(&runtime).expect("C++ archive staging");
        for name in ["libc++_static.a", "libc++abi.a"] {
            std::fs::copy(library.parent().unwrap().join(name), runtime.join(name)).expect("stage NDK C++ archive");
        }
        println!("cargo:rustc-link-search=native={}", runtime.display());
        println!("cargo:rustc-link-lib=static=c++_static");
        println!("cargo:rustc-link-lib=static=c++abi");
    }
    println!("cargo:rerun-if-changed=../../third_party/aosp-pinyin");
    println!("cargo:rerun-if-changed=native");
}
