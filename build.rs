use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    build_web();
    embed_files();
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}

fn build_web() {
    let output = std::process::Command::new("npm")
        .current_dir("web")
        .args(["run", "build"])
        .output()
        .expect("Couldn't npm run build");
    if !output.status.success() {
        println!("stdout: {}", std::str::from_utf8(&output.stdout).unwrap());
        std::process::exit(1);
    }
}

fn embed_files() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets_dir = Path::new(&manifest_dir).join("web/dist");

    println!("cargo:rerun-if-changed=web/dist");

    let mut files = Vec::new();
    collect_files(&assets_dir, &assets_dir, &mut files);

    // ★ 二分探索のためにキー(相対パス文字列)でソートしておく
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut code = String::new();
    code.push_str("pub static EMBEDDED_FILES: &[(&str, &[u8])] = &[\n");

    for (rel_str, abs_path) in &files {
        let abs_path_str = abs_path.to_string_lossy().replace('\\', "\\\\");
        code.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            rel_str, abs_path_str
        ));
    }

    code.push_str("];\n");
    let dest_path = Path::new("./embedded_files.rs");
    fs::write(&dest_path, code).unwrap();
}

fn collect_files(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    for entry in fs::read_dir(dir).expect("read_dir failed") {
        let entry = entry.expect("dir entry failed");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, base, out);
        } else {
            let rel = path.strip_prefix(base).unwrap();
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            out.push((rel_str, path));
        }
    }
}
