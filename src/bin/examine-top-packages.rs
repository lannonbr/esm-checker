use serde_json::Value;
use std::{fs, path::Path};

fn main() {
    let mut files: Vec<String> = vec![];

    collect_files("package.json", Path::new("node/node_modules"), &mut files);

    let mut type_module_count = 0;
    let mut module_count = 0;
    let mut require_count = 0;
    let mut esm_only = 0;

    for file in files.iter() {
        let package_json_str = fs::read_to_string(file).unwrap();
        let package_json: Value = serde_json::from_str(&package_json_str).unwrap();

        if let Some(package_type) = package_json.get("type") {
            if package_type.as_str().unwrap() == "module" {
                type_module_count += 1;
            }
        }
        if package_json.get("module").is_some() {
            module_count += 1;
        }
        if let Some(exports) = package_json.get("exports") {
            let exports_str = exports.to_string();

            if exports_str.contains("require") {
                require_count += 1;
            } else {
                esm_only += 1;
            }
        }
    }

    println!(
        "Packages with a `type: module` field: {}",
        type_module_count
    );
    println!("Packages with a `module` field: {}", module_count);
    println!("Packages with a `exports.require` field: {}", require_count);
    println!(
        "Packages without an explicit `exports.require` that may be ESM only: {}",
        esm_only
    );
}

fn collect_files(filename: &str, dir: &Path, files: &mut Vec<String>) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let e = entry.unwrap();
            let path = e.path();
            if path.is_dir() {
                collect_files(filename, &path, files);
            } else {
                if path.ends_with("package.json") {
                    let path_str = path.as_path().to_str().unwrap().to_string();
                    files.push(path_str);
                }
            }
        }
    }
}
