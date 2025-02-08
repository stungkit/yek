use normalize_path::NormalizePath;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::tempdir;
use yek::config::YekConfig;
use yek::parallel::process_files_parallel;

#[test]
fn test_normalize_path_unix_style() {
    let input = Path::new("/usr/local/bin");
    let base = Path::new("/"); // Dummy base path
    let expected = "usr/local/bin".to_string();
    assert_eq!(
        input
            .strip_prefix(base)
            .unwrap()
            .normalize()
            .to_string_lossy()
            .to_string(),
        expected
    );
}

#[test]
fn test_normalize_path_windows_style() {
    let input = Path::new("C:\\Program Files\\Yek");
    let base = Path::new("C:\\"); // Dummy base for normalization
    let expected = "C:\\Program Files\\Yek".to_string();
    assert_eq!(
        input
            .strip_prefix(base)
            .unwrap()
            .normalize()
            .to_string_lossy()
            .to_string(),
        expected
    );
}

#[test]
fn test_process_files_parallel_empty() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts)
        .expect("process_files_parallel failed");
    assert_eq!(result.len(), 0);
}

#[test]
fn test_process_files_parallel_with_files() {
    use std::fs;
    let temp_dir = tempdir().expect("failed to create temp dir");
    let file_names = vec!["a.txt", "b.txt", "c.txt"];
    for &file in &file_names {
        let file_path = temp_dir.path().join(file);
        fs::write(file_path, "dummy content").expect("failed to write dummy file");
    }
    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let base = temp_dir.path();
    let result =
        process_files_parallel(base, &config, &boosts).expect("process_files_parallel failed");
    assert_eq!(result.len(), file_names.len());
    let names: Vec<&str> = result.iter().map(|pf| pf.rel_path.as_str()).collect();
    for file in file_names {
        assert!(names.contains(&file), "Missing file: {}", file);
    }
}

#[test]
fn test_process_files_parallel_file_read_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let file_path = temp_dir.path().join("unreadable.txt");
    fs::write(&file_path, "content").expect("failed to write file");

    // Make the file unreadable
    let mut permissions = fs::metadata(&file_path).unwrap().permissions();
    permissions.set_mode(0o000); // No permissions
    fs::set_permissions(&file_path, permissions).unwrap();

    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts)
        .expect("process_files_parallel failed");

    // The unreadable file should be skipped, so the result should be empty
    assert_eq!(result.len(), 0);

    // Restore permissions so the directory can be cleaned up
    let mut permissions = fs::metadata(&file_path).unwrap().permissions();
    permissions.set_mode(0o644); // Read permissions
    fs::set_permissions(&file_path, permissions).unwrap();
}

#[test]
fn test_process_files_parallel_gitignore_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let gitignore_path = temp_dir.path().join(".gitignore");
    fs::write(&gitignore_path, "[").expect("failed to write gitignore"); // Invalid gitignore

    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts);

    // Gitignore parse error should be propagated as Err
    assert!(result.is_err());
}

#[test]
fn test_process_files_parallel_walk_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("failed to create subdir");

    // Make the subdir unreadable, causing walk error
    let mut permissions = fs::metadata(&subdir).unwrap().permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&subdir, permissions).unwrap();

    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts);

    // Walk error should be propagated as Err
    assert!(result.is_ok()); // Walk errors are logged and skipped, not propagated as Err
    let processed_files = result.unwrap();
    assert_eq!(processed_files.len(), 0); // No files processed due to walk error
}
