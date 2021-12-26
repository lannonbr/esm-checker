use serde_json::Value;
use std::{fs, path::Path, process::exit};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "examine-top-packages")]
struct Opt {
    /// Output packages with any of `type`, `module`, or `exports` fields in them as CSV
    #[structopt(long)]
    tsv: bool,

    /// Output stats on the packages
    #[structopt(long)]
    stats: bool,
}

#[derive(Debug, Default)]
struct Package {
    name: String,
    module: Option<String>,
    exports: Option<String>,
    package_type: Option<String>,
}

impl Package {
    fn has_values(&self) -> bool {
        self.module.is_some() || self.exports.is_some() || self.package_type.is_some()
    }
}

fn main() {
    let args = Opt::from_args();

    if !args.tsv && !args.stats {
        println!("Please add one of the following flags `--tsv` or `--stats`, view `--help` for more information.");
        exit(1);
    }

    let mut files: Vec<String> = vec![];

    collect_files("package.json", Path::new("node/node_modules"), &mut files);

    let mut pkgs: Vec<Package> = Vec::new();

    for file in files.iter() {
        let package_json_str = fs::read_to_string(file).unwrap();
        let package_json: Value = serde_json::from_str(&package_json_str).unwrap();
        let name_opt = package_json.get("name");

        let mut new_package = Package::default();

        // Skip the current package if it doesn't have a "name" field (assume it is a sub-module)
        if name_opt.is_none() {
            continue;
        }

        let name = name_opt.unwrap().as_str().unwrap();
        new_package.name = name.to_string();

        if let Some(package_type) = package_json.get("type") {
            new_package.package_type = Some(package_type.as_str().unwrap().to_string());
        }
        if let Some(module_str) = package_json.get("module") {
            new_package.module = Some(module_str.as_str().unwrap().to_string());
        }
        if let Some(exports) = package_json.get("exports") {
            new_package.exports = Some(exports.to_string());
        }

        pkgs.push(new_package);
    }

    pkgs = pkgs.into_iter().filter(|p| p.has_values()).collect();

    let type_module_count = pkgs
        .iter()
        .filter(|&p| p.package_type == Some(String::from("module")))
        .count();

    let module_count = pkgs.iter().filter(|&p| p.module.is_some()).count();

    let require_count = pkgs
        .iter()
        .filter(|&p| p.exports.is_some())
        .filter(|&p| p.exports.as_ref().unwrap().contains("require"))
        .count();

    let esm_only = pkgs
        .iter()
        .filter(|&p| p.exports.is_some())
        .filter(|&p| !p.exports.as_ref().unwrap().contains("require"))
        .count();

    if args.tsv {
        println!("name\tpackage_type\tmodule\texports");
        for pkg in pkgs {
            println!(
                "{}\t{:?}\t{:?}\t{:?}",
                pkg.name,
                pkg.package_type.unwrap_or_default(),
                pkg.module.unwrap_or_default(),
                pkg.exports.unwrap_or_default()
            );
        }
    }

    if args.stats {
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
