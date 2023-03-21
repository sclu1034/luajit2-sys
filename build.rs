use fs_extra::dir;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const LIB_NAME: &str = "luajit";
const LUAJIT_HEADERS: [&str; 4] = ["lua.h", "lualib.h", "lauxlib.h", "luajit.h"];

fn main() {
    let luajit_dir = format!("{}/luajit", env!("CARGO_MANIFEST_DIR"));
    let out_dir = env::var("OUT_DIR").unwrap();
    let src_dir = format!("{}/luajit/src", out_dir);
    let lib_path = format!("{}/lib{}.a", &src_dir, LIB_NAME);

    dbg!(&luajit_dir);
    dbg!(&out_dir);
    dbg!(&src_dir);
    dbg!(&lib_path);

    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;

    dir::copy(&luajit_dir, &out_dir, &copy_options).expect("Failed to copy LuaJIT source");

    let mut buildcmd = Command::new("make");
    buildcmd.current_dir(&src_dir);
    buildcmd.stderr(Stdio::inherit());
    buildcmd.arg("BUILDMODE=static");

    if env::var("CARGO_CFG_WINDOWS").is_ok() {
        buildcmd.arg("TARGET_SYS=Windows");
        buildcmd.arg("CROSS=x86_64-w64-mingw32-");
    }

    if cfg!(target_pointer_width = "32") {
        buildcmd.env("HOST_CC", "gcc -m32");
        buildcmd.arg("-e");
    } else {
        buildcmd.env("HOST_CC", "gcc");
    }

    let mut child = buildcmd.spawn().expect("failed to run make");

    if !child
        .wait()
        .map(|status| status.success())
        .map_err(|_| false)
        .unwrap_or(false)
    {
        panic!("Failed to build luajit");
    }

    println!("cargo:lib-name={}", LIB_NAME);
    println!("cargo:include={}", src_dir);
    println!("cargo:rustc-link-search=native={}", src_dir);
    println!("cargo:rustc-link-lib=static={}", LIB_NAME);

    let mut bindings = bindgen::Builder::default();

    for header in LUAJIT_HEADERS {
        println!("cargo:rerun-if-changed={}/src/{}", luajit_dir, header);
        bindings = bindings.header(format!("{}/src/{}", luajit_dir, header));
    }

    let bindings = bindings
        .allowlist_var("LUA.*")
        .allowlist_var("LUAJIT.*")
        .allowlist_type("lua_.*")
        .allowlist_type("luaL_.*")
        .allowlist_function("lua_.*")
        .allowlist_function("luaL_.*")
        .allowlist_function("luaJIT.*")
        .ctypes_prefix("libc")
        .impl_debug(true)
        .use_core()
        .clang_arg("-Iluajit/src")
        // Make it pretty
        .rustfmt_bindings(true)
        .sort_semantically(true)
        .merge_extern_blocks(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
