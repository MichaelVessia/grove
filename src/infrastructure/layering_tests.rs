use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("infrastructure directory should be readable");
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn infrastructure_modules_do_not_depend_on_interface_layer() {
    let infra_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/infrastructure");
    for path in rust_files(&infra_root) {
        if path
            .file_name()
            .is_some_and(|file_name| file_name == "layering_tests.rs")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("source file should be readable");
        assert!(
            !source.contains("crate::interface::"),
            "infrastructure may not depend on interface, found in {}",
            path.display()
        );
    }
}
