use std::{fs, path::Path};

use serde_json::Value;

fn main() {
    let mut files: Vec<String> = vec![];

    visit_dirs("package.json", Path::new("node/node_modules"), &mut files);

    let mut type_module_count = 0;
    let mut module_count = 0;
    let mut require_count = 0;
    let mut esm_only = 0;

    for file in files.iter() {
        let file = fs::read_to_string(file).unwrap();
        let package_json: Value = serde_json::from_str(&file).unwrap();
        if package_json.get("type").is_some() {
            if package_json.get("type").unwrap().as_str().unwrap() == "module" {
                type_module_count += 1;
            }
        }
        if package_json.get("module").is_some() && package_json.get("name").is_some() {
            module_count += 1;
        }
        if package_json.get("exports").is_some() {
            let exports_str = package_json.get("exports").unwrap().to_string();
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

fn visit_dirs(filename: &str, dir: &Path, files: &mut Vec<String>) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let e = entry.unwrap();
            let path = e.path();
            if path.is_dir() {
                visit_dirs(filename, &path, files);
            } else {
                if path.ends_with("package.json") {
                    let path_str = path.as_path().to_str().unwrap().to_string();
                    files.push(path_str);
                }
            }
        }
    }
}
