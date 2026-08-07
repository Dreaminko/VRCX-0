use std::path::Path;

pub fn is_path_inside_directory(path: &Path, directory: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(directory) = directory.canonicalize() else {
        return false;
    };
    path.starts_with(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: std::path::PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-path-utils-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn accepts_a_file_directly_inside_the_directory() {
        let dir = TestDir::new("inside");
        let file = dir.path.join("photo.png");
        std::fs::write(&file, b"data").unwrap();

        assert!(is_path_inside_directory(&file, &dir.path));
    }

    #[test]
    fn accepts_a_file_nested_in_a_subdirectory() {
        let dir = TestDir::new("nested");
        let nested = dir.path.join("sub").join("deep");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("photo.png");
        std::fs::write(&file, b"data").unwrap();

        assert!(is_path_inside_directory(&file, &dir.path));
    }

    #[test]
    fn rejects_a_path_traversal_that_escapes_the_directory() {
        let dir = TestDir::new("escape-parent");
        let allowed = dir.path.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let outside_file = dir.path.join("secret.png");
        std::fs::write(&outside_file, b"data").unwrap();
        let traversal = allowed.join("..").join("secret.png");

        assert!(!is_path_inside_directory(&traversal, &allowed));
    }

    #[test]
    fn rejects_a_sibling_directory_with_a_shared_name_prefix() {
        let dir = TestDir::new("sibling-prefix");
        let allowed = dir.path.join("ugc");
        let sibling = dir.path.join("ugc-other");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        let file = sibling.join("photo.png");
        std::fs::write(&file, b"data").unwrap();

        assert!(!is_path_inside_directory(&file, &allowed));
    }

    #[test]
    fn rejects_a_path_that_does_not_exist() {
        let dir = TestDir::new("missing");

        assert!(!is_path_inside_directory(
            &dir.path.join("missing.png"),
            &dir.path
        ));
    }
}
