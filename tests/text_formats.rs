use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-formats")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lsa"))
            .current_dir(&self.0)
            .args(args)
            .env("TERM", "dumb")
            .output()
            .unwrap()
    }
    fn output(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(out.status.success(), "{args:?}: {out:?}");
        assert!(out.stderr.is_empty(), "{args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn explicit_formats_work_in_pipes_and_last_format_wins() {
    let f = Fixture::new();
    for name in ["a", "bb", "ccc", "d", "ee"] {
        fs::write(f.0.join(name), b"").unwrap();
    }
    assert_eq!(f.output(&["-Cw10"]), "a    d\nbb   ee\nccc\n");
    assert_eq!(f.output(&["-x", "--width=10"]), "a    bb\nccc  d\nee\n");
    assert_eq!(f.output(&["-m", "-w", "10"]), "a, bb,\nccc, d,\nee\n");
    assert_eq!(f.output(&["-lC", "--width=10"]), f.output(&["-Cw10"]));
    assert_eq!(f.output(&["-Cl1"]), "a\nbb\nccc\nd\nee\n");
    assert!(f.output(&["-1l"]).starts_with("total "));
    assert_eq!(f.output(&["-ml1"]), f.output(&[]));
}

#[test]
fn inode_and_allocation_prefixes_preserve_the_requested_layout() {
    let f = Fixture::new();
    fs::write(f.0.join("data"), [1u8; 5000]).unwrap();
    let meta = fs::metadata(f.0.join("data")).unwrap();
    assert_eq!(f.output(&["-i1"]), format!("{} data\n", meta.ino()));
    assert_eq!(
        f.output(&["-1is", "--bytes"]),
        format!("{} {} data\n", meta.ino(), meta.blocks() * 512)
    );
    assert_eq!(
        f.output(&["-s1k"]),
        format!("{} data\n", meta.blocks().div_ceil(2))
    );
    fs::write(f.0.join("other"), b"").unwrap();
    for format in ["-C", "-x", "-m"] {
        let text = f.output(&["-is", format, "--bytes", "--width=200"]);
        assert_eq!(text.lines().count(), 1, "{text}");
        assert!(text.contains(&format!("{} {} data", meta.ino(), meta.blocks() * 512)));
        assert!(text.contains("other"));
    }
}

#[test]
fn question_mark_and_escape_modes_apply_to_every_name_format() {
    let f = Fixture::new();
    fs::write(f.0.join("a\n\x1b[31m\\桃"), b"").unwrap();
    for format in ["-1", "-C", "-m", "-l"] {
        let text = f.output(&[format, "-q"]);
        assert!(text.contains("a??[31m\\桃"), "{text}");
        assert!(!text.contains('\x1b'));
        let escaped = f.output(&[format, "-qb"]);
        assert!(escaped.contains("a\\n\\u{1b}[31m\\\\桃"), "{escaped}");
    }
}
