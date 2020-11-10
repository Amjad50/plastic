fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config = cbindgen::Config {
        no_includes: true,
        ..cbindgen::Config::default()
    };
    config.language = cbindgen::Language::C;
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .with_no_includes()
        // allow ffigen to detect int64_t
        .with_include("stdint.h")
        // for allo_isolate
        .with_trailer("void store_dart_post_cobject(char (*)(long, *Dart_CObject));")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("../target/binding.h");

    // dart_bindgen is not good enough, maybe later we can use it
    //
    // let config = DynamicLibraryConfig {
    //     linux: DynamicLibraryCreationMode::open("./libtest.so").into(),
    //     ..Default::default()
    // };
    // load the c header file, with config and lib name
    // let codegen = Codegen::builder()
    //     .with_src_header("binding.h")
    //     .with_lib_name("libtest")
    //     .with_config(config)
    //     .with_allo_isolate()
    //     .build()
    //     .unwrap();
    // // generate the dart code and get the bindings back
    // let bindings = codegen.generate().unwrap();
    // write the bindings to your dart package
    // and start using it to write your own high level abstraction.
    // bindings.write_to_file("../bin/libtest/ffi.dart").unwrap();
}
