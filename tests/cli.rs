use std::{
    ffi::OsStr,
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{PermissionsExt, symlink},
        net::UnixListener,
    },
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-fixtures")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn touch(&self, name: impl AsRef<OsStr>, content: &[u8]) {
        fs::write(self.0.join(name.as_ref()), content).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lsa"))
            .current_dir(&self.0)
            .arg("-F")
            .args(args)
            .env("TERM_PROGRAM", "ghostty")
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn mixed_entries_hidden_and_raw_names() {
    let f = Fixture::new();
    f.touch(".hidden", b"");
    f.touch("A.txt", b"hello");
    f.touch("桃.txt", b"");
    f.touch("bad\n\x1b[31m", b"");
    fs::create_dir(f.0.join("folder")).unwrap();
    symlink("missing", f.0.join("broken.png")).unwrap();
    let socket = UnixListener::bind(f.0.join("socket")).unwrap();
    let o = f.run(&["--grid", "--protocol=kitty"]);
    assert!(o.status.success());
    assert_eq!(
        o.stdout,
        "A.txt\nbad\\n\\u{1b}[31m\nbroken.png@\nfolder/\nsocket=\n桃.txt\n".as_bytes()
    );
    assert!(o.stderr.is_empty());
    assert!(f.run(&["-a"]).stdout.starts_with(b".hidden\n"));
    assert_eq!(f.run(&["-A"]).stdout, f.run(&["-a"]).stdout);
    drop(socket);
}

// APFS rejects these names; display's byte-level test runs on all Unix targets.
#[cfg(not(target_os = "macos"))]
#[test]
fn invalid_utf8_operand_and_entry() {
    let f = Fixture::new();
    let name = OsStr::from_bytes(b"invalid\xff");
    f.touch(name, b"");
    assert_eq!(f.run(&[]).stdout, b"invalid\\xff\n");
    let out = Command::new(env!("CARGO_BIN_EXE_lsa"))
        .current_dir(&f.0)
        .arg(name)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, b"invalid\\xff\n");
}

#[test]
fn sizes_times_reverse_and_long_symlinks() {
    let f = Fixture::new();
    f.touch("a", b"1");
    f.touch("b", b"12345");
    f.touch("c", b"12345");
    assert_eq!(f.run(&["-S"]).stdout, b"b\nc\na\n");
    assert_eq!(f.run(&["-Sr"]).stdout, b"a\nc\nb\n");
    let earlier = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    for name in ["b", "c"] {
        fs::File::options()
            .write(true)
            .open(f.0.join(name))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(earlier))
            .unwrap();
    }
    assert_eq!(f.run(&["-t"]).stdout, b"a\nb\nc\n");
    symlink("a", f.0.join("link")).unwrap();
    let out = String::from_utf8(f.run(&["-l"]).stdout).unwrap();
    assert!(out.contains("link@ -> a\n"));
    assert_eq!(out.lines().count(), 4);
}

#[test]
fn directories_first_preserves_groups_under_reverse_and_other_sorts() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("b-dir")).unwrap();
    fs::create_dir(f.0.join("y-dir")).unwrap();
    f.touch("a", b"1");
    f.touch("z", b"123456789");
    symlink("b-dir", f.0.join("c-link")).unwrap();
    assert_eq!(
        f.run(&["--dirs-first"]).stdout,
        b"b-dir/\ny-dir/\na\nc-link@\nz\n"
    );
    assert_eq!(
        f.run(&["--dirs-first", "-r"]).stdout,
        b"y-dir/\nb-dir/\nz\nc-link@\na\n"
    );
    assert_eq!(
        f.run(&["--dirs-first", "-S"]).stdout,
        b"b-dir/\ny-dir/\nz\nc-link@\na\n"
    );
    let earlier = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    for name in ["b-dir", "z"] {
        fs::File::open(f.0.join(name))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(earlier))
            .unwrap();
    }
    fs::File::open(f.0.join("a"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(earlier + std::time::Duration::from_secs(10)))
        .unwrap();
    assert_eq!(
        f.run(&["--dirs-first", "-t"]).stdout,
        b"y-dir/\nb-dir/\nc-link@\na\nz\n"
    );
    // Sorting applies inside directory listings; explicit operands retain order.
    assert_eq!(f.run(&["--dirs-first", "z", "a"]).stdout, b"z\na\n");
}

#[test]
fn selected_metadata_aligns_and_preserves_names_and_links() {
    let f = Fixture::new();
    f.touch("a", b"1");
    f.touch("large", &vec![0; 1536]);
    fs::set_permissions(f.0.join("a"), fs::Permissions::from_mode(0o640)).unwrap();
    fs::set_permissions(f.0.join("large"), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        f.run(&["--fields=size", "--bytes"]).stdout,
        b"   1 a\n1536 large\n"
    );
    assert_eq!(
        f.run(&["--fields=size,mode", "-h"]).stdout,
        b"   1 -rw-r----- a\n1.5K -rw------- large\n"
    );
    assert_eq!(f.run(&["--long"]).stdout, f.run(&["-l"]).stdout);
    symlink("target\n\x1b", f.0.join("link")).unwrap();
    let out = f.run(&["--fields=mode"]);
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .ends_with("link@ -> target\\n\\u{1b}\n")
    );
    assert_eq!(f.run(&["--fields=size,size"]).status.code(), Some(2));
}

#[test]
fn operands_and_partial_errors() {
    let f = Fixture::new();
    f.touch("-name", b"");
    fs::create_dir(f.0.join("dir")).unwrap();
    f.touch("dir/inside", b"");
    symlink("dir", f.0.join("linkdir")).unwrap();
    assert_eq!(f.run(&["--", "-name"]).stdout, b"-name\n");
    assert_eq!(f.run(&["linkdir"]).stdout, b"inside\n");
    assert_eq!(f.run(&["-d", "linkdir"]).stdout, b"linkdir@\n");
    assert!(
        String::from_utf8(f.run(&["-l", "linkdir"]).stdout)
            .unwrap()
            .contains("linkdir@ -> dir")
    );
    let out = f.run(&["missing", "dir", "--", "-name"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"dir:\ninside\n-name\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing:"));
    assert_eq!(f.run(&["--unknown"]).status.code(), Some(2));
    assert_eq!(f.run(&["dir"]).stdout, b"inside\n");
    let diagnostic = f.run(&["--diagnose", "missing", "dir"]);
    assert_eq!(diagnostic.status.code(), Some(1));
    let report = String::from_utf8(diagnostic.stdout).unwrap();
    assert!(report.contains("path=dir\nentries=1\npreview_candidates=0\nlayout=lines\n"));
    assert!(String::from_utf8_lossy(&diagnostic.stderr).contains("missing:"));
}

#[test]
fn inaccessible_directory_reports_error() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let f = Fixture::new();
    let path = f.0.join("locked");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let out = f.run(&["locked"]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn image_named_fifo_never_blocks_text_path() {
    let f = Fixture::new();
    let path = std::ffi::CString::new(f.0.join("pipe.png").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    assert_eq!(f.run(&["--no-images"]).stdout, b"pipe.png|\n");
}

#[test]
fn empty_and_closed_pipe() {
    let f = Fixture::new();
    assert!(f.run(&[]).stdout.is_empty());
    for i in 0..1000 {
        f.touch(format!("entry-{i:04}"), b"");
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_lsa"))
        .current_dir(&f.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
}

#[test]
fn defaults_are_plain_and_natural_and_classification_is_explicit() {
    let f = Fixture::new();
    for name in ["photo10.png", "photo2.png", "Photo1.png", "run"] {
        f.touch(name, b"");
    }
    fs::set_permissions(f.0.join("run"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::create_dir(f.0.join("folder")).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_lsa"))
            .current_dir(&f.0)
            .args(args)
            .output()
            .unwrap()
    };
    assert_eq!(
        run(&[]).stdout,
        b"folder\nPhoto1.png\nphoto2.png\nphoto10.png\nrun\n"
    );
    assert_eq!(
        run(&["-F"]).stdout,
        b"folder/\nPhoto1.png\nphoto2.png\nphoto10.png\nrun*\n"
    );
    let out = run(&["--color=always", "--icons=always"]);
    assert!(out.status.success());
    assert!(out.stdout.windows(5).any(|w| w == b"\x1b[32m"));
}

#[test]
fn readable_long_defaults_and_numeric_escape_hatch() {
    let f = Fixture::new();
    f.touch("a", &vec![0; 1536]);
    let long = String::from_utf8(f.run(&["-l"]).stdout).unwrap();
    assert!(long.contains("1.5K"));
    assert!(
        String::from_utf8(f.run(&["-ln", "--bytes"]).stdout)
            .unwrap()
            .contains("1536")
    );
    let header = String::from_utf8(f.run(&["--header"]).stdout).unwrap();
    assert!(header.starts_with("Permissions"));
    assert_eq!(header.lines().count(), 2);
    assert_eq!(
        f.run(&["--fields=user,group,uid,gid,links,size,mode,modified"])
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn no_sort_retains_enumeration_order_and_does_not_require_stats() {
    let f = Fixture::new();
    for i in [20, 1, 13, 4] {
        f.touch(format!("file{i}"), b"");
    }
    let expected: Vec<_> = fs::read_dir(&f.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let out = String::from_utf8(f.run(&["-U"]).stdout).unwrap();
    assert_eq!(out.lines().collect::<Vec<_>>(), expected);
    let out = String::from_utf8(f.run(&["-Ur"]).stdout).unwrap();
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        expected
            .iter()
            .rev()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
}

#[test]
fn long_timestamps_preserve_epoch_boundaries_and_repeated_values() {
    let f = Fixture::new();
    for (name, time) in [("a", -1_i64), ("b", 0), ("c", 64), ("d", 0)] {
        f.touch(name, b"");
        let epoch = std::time::UNIX_EPOCH;
        let modified = if time < 0 {
            epoch - std::time::Duration::from_secs(time.unsigned_abs())
        } else {
            epoch + std::time::Duration::from_secs(time as u64)
        };
        fs::File::options()
            .write(true)
            .open(f.0.join(name))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_lsa"))
        .current_dir(&f.0)
        .env("TZ", "UTC0")
        .arg("--fields=modified")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        out.stdout,
        b"1969-12-31 23:59 a\n1970-01-01 00:00 b\n1970-01-01 00:01 c\n1970-01-01 00:00 d\n"
    );
}
