use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, UNIX_EPOCH},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-traversal")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, bytes: &[u8]) {
        fs::write(self.0.join(name), bytes).unwrap();
    }
    fn dir(&self, name: &str) {
        fs::create_dir_all(self.0.join(name)).unwrap();
    }
    fn link(&self, target: &str, name: &str) {
        symlink(target, self.0.join(name)).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lsa"))
            .current_dir(&self.0)
            .args(args)
            .env("TERM", "dumb")
            .output()
            .unwrap()
    }
    fn text(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn operands_sort_together_before_directory_sections() {
    let f = Fixture::new();
    f.file("z", b"12345");
    f.file("a", b"1");
    f.dir("b-dir");
    f.dir("y-dir");
    f.file("b-dir/inside", b"");
    assert_eq!(
        f.text(&["y-dir", "z", "b-dir", "a"]),
        "a\nz\n\nb-dir:\ninside\n\ny-dir:\n"
    );
    assert_eq!(
        f.text(&["-r", "y-dir", "z", "b-dir", "a"]),
        "z\na\n\ny-dir:\n\nb-dir:\ninside\n"
    );
    assert_eq!(
        f.text(&["-d", "y-dir", "z", "b-dir", "a"]),
        "a\nb-dir\ny-dir\nz\n"
    );
    assert_eq!(f.text(&["-S", "a", "z"]), "z\na\n");
    let old = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    fs::File::open(f.0.join("z"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(old))
        .unwrap();
    assert_eq!(f.text(&["-t", "z", "a"]), "a\nz\n");
    assert_eq!(f.text(&["-U", "z", "a"]), "z\na\n");
    let out = f.run(&["missing", "b-dir", "z", "a"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"a\nz\n\nb-dir:\ninside\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing:"));
}

#[test]
fn hidden_entries_and_dot_entries_are_distinct() {
    let f = Fixture::new();
    f.file("visible", b"");
    f.file(".hidden", b"");
    assert_eq!(f.text(&[]), "visible\n");
    assert_eq!(f.text(&["-A"]), ".hidden\nvisible\n");
    assert_eq!(f.text(&["-a"]), ".\n..\n.hidden\nvisible\n");
    assert_eq!(f.text(&["-a", "-A"]), ".hidden\nvisible\n");
    assert_eq!(f.text(&["-A", "-a"]), ".\n..\n.hidden\nvisible\n");
    assert_eq!(f.text(&["-aF"]), "./\n../\n.hidden\nvisible\n");
}

#[test]
fn dot_metadata_keeps_the_directory_base_and_operand_paths() {
    use std::os::unix::fs::MetadataExt;
    let f = Fixture::new();
    f.dir("folder");
    // A same-named parent entry catches silently reading the wrong metadata,
    // not just ENOENT after processing synthesized dot entries.
    f.file("data", &[0; 99]);
    f.file("folder/data", &[0; 5]);
    let output = f.text(&["-la", "--fields=size,inode", "--bytes", "folder"]);
    let rows: Vec<_> = output
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .collect();
    assert_eq!(rows.len(), 3, "{output}");
    for (row, name) in rows.iter().zip([".", "..", "data"]) {
        let meta = fs::symlink_metadata(f.0.join("folder").join(name)).unwrap();
        assert_eq!(row[1], meta.ino().to_string());
        assert_eq!(row[2], name);
        assert_eq!(row[0], if name == "data" { "5" } else { "-" });
    }
    let long = f.text(&["-la", "folder"]);
    assert!(!long.contains('?'), "{long}");
    assert!(long.lines().any(|line| line.ends_with(" data")));
    f.dir("links");
    f.link("../folder/data", "links/good");
    f.link("missing", "links/broken");
    let links = f.text(&[
        "-lP",
        "--fields=size",
        "--bytes",
        "links/good",
        "links/broken",
    ]);
    assert!(links.contains("7 links/broken -> missing"), "{links}");
    assert!(links.contains("14 links/good -> ../folder/data"), "{links}");
    assert_eq!(
        f.text(&[
            "-lH",
            "--fields=size",
            "--bytes",
            "links/good",
            "folder/data"
        ]),
        "5 folder/data\n5 links/good\n"
    );
}

#[test]
fn symlink_controls_apply_to_operands_and_entries() {
    let f = Fixture::new();
    f.dir("folder");
    f.file("folder/inside", b"12345");
    f.file("data", b"123456789");
    f.link("folder", "dirlink");
    f.link("data", "filelink");
    assert_eq!(f.text(&["dirlink"]), "inside\n");
    assert_eq!(f.text(&["-F", "dirlink"]), "dirlink@\n");
    assert_eq!(f.text(&["-P", "dirlink"]), "dirlink\n");
    assert_eq!(f.text(&["-HPF", "dirlink"]), "dirlink@\n");
    assert_eq!(f.text(&["-PHF", "dirlink"]), "inside\n");
    assert_eq!(f.text(&["-dHF", "dirlink"]), "dirlink/\n");
    assert_eq!(f.text(&["-dPF", "dirlink"]), "dirlink@\n");
    assert_eq!(
        f.text(&["-lH", "--fields=size", "--bytes", "filelink"]),
        "9 filelink\n"
    );
    assert!(f.text(&["-lP", "filelink"]).contains("filelink -> data"));
    assert!(f.text(&["-HF"]).contains("dirlink@\n"));
    let followed = f.text(&["-LF"]);
    assert!(followed.contains("dirlink/\n"));
    assert!(followed.contains("filelink\n"));
    assert!(!followed.contains('@'));
    // Broken links remain visible even when a requested target is unavailable.
    f.link("missing", "broken");
    let out = f.run(&["-LF", "broken"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"broken@\n");
    let out = f.run(&["-LF"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("broken@\n"));
    assert!(f.run(&["-PF", "broken"]).status.success());
}

#[test]
fn recursion_is_depth_first_sorted_and_skips_dot_descent() {
    let f = Fixture::new();
    f.dir("root/z");
    f.dir("root/a/nested");
    f.dir("root/.hidden");
    f.file("root/a/nested/leaf", b"");
    f.file("root/file", b"");
    assert_eq!(
        f.text(&["-R", "root"]),
        "root:\na\nfile\nz\n\nroot/a:\nnested\n\nroot/a/nested:\nleaf\n\nroot/z:\n"
    );
    let reversed = f.text(&["-Rr", "root"]);
    assert!(reversed.find("root/z:").unwrap() < reversed.find("root/a:").unwrap());
    let all = f.text(&["-aR", "root"]);
    assert!(all.contains("root/.hidden:\n.\n..\n"));
    assert!(!all.contains("root/.:"));
    assert!(!all.contains("root/..:"));
    assert_eq!(f.text(&["-RdF", "root"]), "root/\n");
}

#[test]
fn recursive_link_policies_detect_ancestor_cycles_and_allow_sibling_aliases() {
    let f = Fixture::new();
    f.dir("root/dir");
    f.file("root/dir/leaf", b"");
    f.link("dir", "root/alias");
    f.link("root", "operand-link");
    let default = f.text(&["-R", "operand-link"]);
    assert!(default.contains("operand-link/dir:\nleaf\n"));
    assert!(!default.contains("operand-link/alias:"));
    let command_line = f.text(&["-RH", "operand-link"]);
    assert!(command_line.contains("operand-link/dir:\nleaf\n"));
    assert!(!command_line.contains("operand-link/alias:"));
    assert_eq!(f.text(&["-RPF", "operand-link"]), "operand-link@\n");
    let all = f.text(&["-RL", "root"]);
    assert!(all.contains("root/alias:\nleaf\n"));
    assert!(all.contains("root/dir:\nleaf\n"));
    f.link("..", "root/dir/back");
    let cycle = f.run(&["-RL", "root"]);
    assert_eq!(cycle.status.code(), Some(1));
    let errors = String::from_utf8_lossy(&cycle.stderr);
    assert!(errors.contains("root/alias/back: recursive directory cycle"));
    assert!(errors.contains("root/dir/back: recursive directory cycle"));
    assert!(cycle.stdout.len() < 1000);
    assert!(String::from_utf8_lossy(&cycle.stdout).contains("leaf\n"));
}

#[test]
fn recursive_errors_continue_to_other_directories() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let f = Fixture::new();
    f.dir("root/a-locked");
    f.dir("root/z-readable");
    f.file("root/z-readable/leaf", b"");
    let locked = f.0.join("root/a-locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
    let out = f.run(&["-R", "root"]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("root/a-locked:"));
    assert!(String::from_utf8_lossy(&out.stdout).contains("root/z-readable:\nleaf\n"));
}

#[test]
fn deep_recursion_does_not_use_the_call_stack() {
    let f = Fixture::new();
    let mut path = String::from("root");
    for _ in 0..160 {
        path.push_str("/d");
    }
    f.dir(&path);
    f.file(&format!("{path}/leaf"), b"");
    let out = f.text(&["-R", "root"]);
    assert_eq!(out.lines().filter(|line| line.ends_with(':')).count(), 161);
    assert!(out.ends_with("leaf\n"));
}
