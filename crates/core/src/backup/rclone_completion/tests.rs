use super::*;
#[test]
fn native_user_directory_rejects_an_environment_fallback_path() {
    let local = Path::new("/private/var/folders/ab/native-user/0");
    assert!(
        qualified_temp_path(local, Path::new("/tmp/inherited-helper-work/sh-rc-long")).is_err()
    );
    assert_eq!(
        qualified_temp_path(local, Path::new("/var/folders/ab/native-user/T/")).unwrap(),
        Path::new("/private/var/folders/ab/native-user/T")
    );
}
#[test]
fn native_socket_parent_does_not_use_long_work_or_inherited_helper_temp_paths() {
    let work = fixture();
    let deep = work.path().join("a".repeat(80)).join("b".repeat(80));
    std::fs::create_dir_all(&deep).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let mut lease = begin(&deep, count.clone()).unwrap();
    assert!(socket_path_fits(&lease.path));
    assert!(!lease.path.starts_with(&deep));
    assert_eq!(
        lease.path.parent().unwrap().parent().unwrap(),
        native_temp_directory().unwrap()
    );
    let endpoint = register_at(&lease.path, &lease.token).unwrap();
    drop(endpoint);
    lease.close_and_wait(Duration::from_secs(2)).unwrap();
    assert_eq!(count.load(Ordering::Acquire), 0);
    assert!(!socket_path_fits(&PathBuf::from(
        "/".to_owned() + &"x".repeat(103)
    )));
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("stillharbour-completion-")
        .tempdir()
        .unwrap();
    assert!(
        root.path()
            .canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap())
    );
    root
}
#[test]
fn registration_blocks_completion_until_every_wrapper_guardian_reference_is_closed() {
    let work = fixture();
    let count = Arc::new(AtomicUsize::new(0));
    let mut lease = begin(work.path(), count.clone()).unwrap();
    let stream = register_at(&lease.path, &lease.token).unwrap();
    let guardian = stream.try_clone().unwrap();
    assert!(lease.close_and_wait(Duration::from_millis(30)).is_err());
    assert_eq!(count.load(Ordering::Acquire), 1);
    drop(stream);
    assert!(lease.close_and_wait(Duration::from_millis(30)).is_err());
    drop(guardian);
    lease.close_and_wait(Duration::from_secs(2)).unwrap();
    assert_eq!(count.load(Ordering::Acquire), 0);
    assert!(register_at(&lease.path, &lease.token).is_err());
}
#[test]
fn invalid_token_never_receives_spawn_authority() {
    let work = fixture();
    let count = Arc::new(AtomicUsize::new(0));
    let mut lease = begin(work.path(), count.clone()).unwrap();
    assert!(register_at(&lease.path, "00000000000000000000000000000000").is_err());
    lease.close_and_wait(Duration::from_secs(2)).unwrap();
    assert_eq!(count.load(Ordering::Acquire), 0);
}
#[test]
fn unexpected_registered_channel_data_keeps_shutdown_uncertified() {
    let work = fixture();
    let count = Arc::new(AtomicUsize::new(0));
    let mut lease = begin(work.path(), count.clone()).unwrap();
    let mut stream = register_at(&lease.path, &lease.token).unwrap();
    stream.write_all(b"unexpected").unwrap();
    assert!(lease.close_and_wait(Duration::from_secs(2)).is_err());
    assert_eq!(count.load(Ordering::Acquire), 1);
    drop(stream);
}
#[test]
fn closing_admission_races_registration_without_issuing_late_spawn_authority() {
    for _ in 0..8 {
        let work = fixture();
        let count = Arc::new(AtomicUsize::new(0));
        let mut lease = begin(work.path(), count.clone()).unwrap();
        let path = lease.path.clone();
        let token = lease.token.to_string();
        let client = thread::spawn(move || {
            if let Ok(stream) = register_at(&path, &token) {
                drop(stream);
            }
        });
        lease.close_and_wait(Duration::from_secs(2)).unwrap();
        client.join().unwrap();
        assert_eq!(count.load(Ordering::Acquire), 0);
    }
}
