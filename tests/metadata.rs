use std::{
    ffi::CString,
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, symlink},
    },
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-fixtures")
            .join(format!(
                "metadata-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_lsa"))
            .current_dir(&self.0)
            .args(args)
            .env("NO_COLOR", "1")
            .env("TZ", "UTC0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn set_xattr(path: &Path, name: &str, value: &[u8]) -> bool {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let name = CString::new(name).unwrap();
    let result = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::setxattr(
                path.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
                0,
            )
        }
        #[cfg(target_os = "linux")]
        {
            libc::setxattr(
                path.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
            )
        }
    };
    if result == 0 {
        return true;
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOTSUP) {
        eprintln!("Skipping xattr fixture: filesystem does not support extended attributes");
        return false;
    }
    panic!("setxattr: {error}");
}

#[test]
fn explicit_details_include_link_count_and_both_owners() {
    let f = Fixture::new();
    fs::write(f.0.join("data"), b"content").unwrap();
    fs::hard_link(f.0.join("data"), f.0.join("second")).unwrap();
    let metadata = fs::metadata(f.0.join("data")).unwrap();
    let output = f.run(&["-ln", "--bytes", "data"]);
    let fields: Vec<_> = output.split_whitespace().collect();
    assert_eq!(fields.len(), 8, "{output}");
    assert_eq!(fields[1], "2");
    assert_eq!(fields[2], metadata.uid().to_string());
    assert_eq!(fields[3], metadata.gid().to_string());
    assert_eq!(fields[4], "7");
    assert_eq!(fields[7], "data");
    assert_eq!(f.run(&["-lgo", "data"]).split_whitespace().count(), 6);
    assert!(!f.run(&["-e", "data"]).contains("acl: unavailable"));
}

#[test]
fn totals_and_allocated_units_match_stat_and_respect_custom_fields() {
    let f = Fixture::new();
    fs::write(f.0.join("data"), vec![1; 5000]).unwrap();
    let blocks = fs::metadata(f.0.join("data")).unwrap().blocks();
    assert!(
        f.run(&["-l", "--bytes"])
            .starts_with(&format!("total {}\n", blocks * 512))
    );
    assert!(
        f.run(&["-lk"])
            .starts_with(&format!("total {}\n", blocks.div_ceil(2)))
    );
    assert_eq!(
        f.run(&["-sk1", "data"]),
        format!("{} data\n", blocks.div_ceil(2))
    );
    assert_eq!(f.run(&["--fields=size", "--bytes"]), "5000 data\n");
    assert!(f.run(&["-l", "--header"]).starts_with("Permissions"));
}

#[test]
fn attributes_are_visible_bounded_and_follow_the_selected_link_policy() {
    let f = Fixture::new();
    let name = if cfg!(target_os = "macos") {
        "com.lsa.test"
    } else {
        "user.lsa.test"
    };
    fs::write(f.0.join("data"), b"content").unwrap();
    if !set_xattr(&f.0.join("data"), name, b"private attribute contents") {
        return;
    }
    symlink("data", f.0.join("link")).unwrap();
    let long = f.run(&["-l", "data"]);
    assert!(
        long.split_whitespace().next().unwrap().ends_with('@'),
        "{long}"
    );
    let output = f.run(&["-@", "data"]);
    assert!(
        output.contains(&format!("  {name}\t26 bytes\n")),
        "{output}"
    );
    assert!(!output.contains("private attribute contents"));
    assert!(f.run(&["-@L", "link"]).contains(name));
    assert!(!f.run(&["-@P", "link"]).contains(name));
    assert_eq!(f.run(&["-@", "-1", "data"]), "data\n");
    assert_eq!(f.run(&["-Z", "-1", "data"]), "data\n");
}

#[cfg(target_os = "linux")]
#[test]
fn posix_acl_details_are_available_without_libacl() {
    let f = Fixture::new();
    fs::write(f.0.join("data"), b"content").unwrap();
    let mut bytes = 2_u32.to_le_bytes().to_vec();
    for (tag, permissions, id) in [
        (1_u16, 6_u16, u32::MAX),
        (2, 4, 12345),
        (4, 4, u32::MAX),
        (16, 4, u32::MAX),
        (32, 0, u32::MAX),
    ] {
        bytes.extend(tag.to_le_bytes());
        bytes.extend(permissions.to_le_bytes());
        bytes.extend(id.to_le_bytes());
    }
    if !set_xattr(&f.0.join("data"), "system.posix_acl_access", &bytes) {
        return;
    }
    let output = f.run(&["-e", "data"]);
    assert!(
        output.split_whitespace().next().unwrap().ends_with('+'),
        "{output}"
    );
    assert!(output.contains("  user:12345:r--\n"), "{output}");
    assert!(output.contains("  mask::r--\n"), "{output}");
}

#[cfg(target_os = "macos")]
#[test]
fn darwin_acls_and_file_flags_are_reported_without_opening_contents() {
    use std::ffi::{c_int, c_void};
    unsafe extern "C" {
        fn acl_init(count: c_int) -> *mut c_void;
        fn acl_create_entry(acl: *mut *mut c_void, entry: *mut *mut c_void) -> c_int;
        fn acl_set_tag_type(entry: *mut c_void, tag: c_int) -> c_int;
        fn acl_set_qualifier(entry: *mut c_void, qualifier: *const c_void) -> c_int;
        fn acl_set_permset_mask_np(entry: *mut c_void, mask: u64) -> c_int;
        fn acl_set_file(path: *const libc::c_char, kind: c_int, acl: *mut c_void) -> c_int;
        fn acl_free(value: *mut c_void) -> c_int;
    }
    let f = Fixture::new();
    fs::write(f.0.join("data"), b"content").unwrap();
    symlink("data", f.0.join("link")).unwrap();
    let path = CString::new(f.0.join("data").as_os_str().as_bytes()).unwrap();
    let mut acl = unsafe { acl_init(1) };
    assert!(!acl.is_null());
    let mut entry = std::ptr::null_mut();
    let uuid = [0x12_u8; 16];
    assert_eq!(unsafe { acl_create_entry(&mut acl, &mut entry) }, 0);
    assert_eq!(unsafe { acl_set_tag_type(entry, 1) }, 0);
    assert_eq!(unsafe { acl_set_qualifier(entry, uuid.as_ptr().cast()) }, 0);
    assert_eq!(unsafe { acl_set_permset_mask_np(entry, 1 << 1) }, 0);
    let result = unsafe { acl_set_file(path.as_ptr(), 0x100, acl) };
    let error = std::io::Error::last_os_error();
    unsafe {
        acl_free(acl);
    }
    if result != 0 && error.raw_os_error() == Some(libc::ENOTSUP) {
        eprintln!("Skipping ACL fixture: filesystem does not support ACLs");
        return;
    }
    assert_eq!(result, 0, "{error}");
    let output = f.run(&["-e", "data"]);
    assert!(
        output.contains("uuid:12121212-1212-1212-1212-121212121212 allow read"),
        "{output}"
    );
    assert!(f.run(&["-eL", "link"]).contains("allow read"));
    assert!(!f.run(&["-eP", "link"]).contains("allow read"));
    assert_eq!(unsafe { libc::chflags(path.as_ptr(), libc::UF_NODUMP) }, 0);
    assert!(f.run(&["-O", "data"]).contains("nodump"));
    assert_eq!(f.run(&["--fields=flags", "data"]), "nodump data\n");
}
