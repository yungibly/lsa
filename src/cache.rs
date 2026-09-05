//! Opt-in, bounded thumbnail experiment. No directory scans, access-time
//! writes, terminal IDs, or work until a budgeted preview actually needs it.
use image::RgbaImage;
use std::{
    collections::hash_map::RandomState,
    ffi::CString,
    fs::{self, File, Metadata, OpenOptions},
    hash::BuildHasher,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

// Bump BOTH for changes to the record, source limits, decoder, orientation,
// resize filter, transparency checker, or other pixel transforms.
pub const NAMESPACE: &str = "lsa-thumbnails-v1";
const MAGIC: &[u8; 8] = b"LSATHM01";
pub const SLOTS: usize = 64;
const WAYS: usize = 8;
const KEY_LEN: usize = 80;
const HEADER_LEN: usize = KEY_LEN + 4;
pub const STORAGE_LIMIT: usize = (SLOTS + 1) * (HEADER_LEN + 320 * 240 * 4);
const STAGING: &str = "staging.tmp";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key([u8; KEY_LEN]);

impl Key {
    pub fn new(meta: &Metadata, width: u32, height: u32) -> Self {
        let mut bytes = [0; KEY_LEN];
        bytes[..8].copy_from_slice(MAGIC);
        for (chunk, value) in bytes[8..].as_chunks_mut::<8>().0.iter_mut().zip([
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime() as u64,
            meta.mtime_nsec() as u64,
            meta.ctime() as u64,
            meta.ctime_nsec() as u64,
            u64::from(width),
            u64::from(height),
        ]) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Self(bytes)
    }

    fn dimensions(&self) -> io::Result<(u32, u32)> {
        let width = u64::from_le_bytes(self.0[64..72].try_into().unwrap());
        let height = u64::from_le_bytes(self.0[72..80].try_into().unwrap());
        if !(1..=320).contains(&width) || !(1..=240).contains(&height) {
            return Err(invalid("invalid cache dimensions"));
        }
        Ok((width as u32, height as u32))
    }

    fn slots(&self) -> std::ops::Range<usize> {
        let start = crc32fast::hash(&self.0) as usize % (SLOTS / WAYS) * WAYS;
        start..start + WAYS
    }
}

#[derive(Default)]
pub struct Stats {
    pub hits: usize,
    pub misses: usize,
    pub writes: usize,
    pub errors: usize,
}

pub struct Cache<'a> {
    path: Option<&'a Path>,
    // Remember failed initialization: inaccessible storage costs one attempt.
    store: Option<io::Result<Store>>,
    pub stats: Stats,
}

impl<'a> Cache<'a> {
    pub fn new(path: Option<&'a Path>) -> Self {
        Self {
            path,
            store: None,
            stats: Stats::default(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.path.is_some()
    }

    pub fn get(&mut self, key: &Key) -> Option<RgbaImage> {
        let path = self.path?;
        let store = self.store.get_or_insert_with(|| Store::open(path, true));
        let result = match store {
            Ok(store) => store.get(key),
            Err(_) => Err(invalid("cache unavailable")),
        };
        match result {
            Ok(Some(image)) => {
                self.stats.hits += 1;
                Some(image)
            }
            other => {
                self.stats.misses += 1;
                self.stats.errors += usize::from(other.is_err());
                None
            }
        }
    }

    pub fn put(&mut self, key: &Key, image: &RgbaImage) {
        if let Some(Ok(store)) = &self.store {
            match store.put(key, image) {
                Ok(()) => self.stats.writes += 1,
                Err(_) => self.stats.errors += 1,
            }
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

struct Store {
    directory: File,
    lock: File,
    // Seeded only when the optional cache opens. A fresh seed per invocation
    // avoids repeated listings cycling through FIFO victims in lockstep.
    eviction: RandomState,
}

struct Record {
    file: File,
    header: [u8; HEADER_LEN],
    length: u64,
}

impl Store {
    fn open(path: &Path, create: bool) -> io::Result<Self> {
        let path = path.join(NAMESPACE);
        if create {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&path)?;
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?;
        let meta = directory.metadata()?;
        // Thumbnails can contain private source content. Refuse a shared namespace.
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(invalid(
                "cache namespace must be private and owned by this user",
            ));
        }
        // Keep this inode permanently: unlinking a lock can split concurrent users.
        // Read-only open permits warm hits from a read-only cache.
        let lock = match open_at(&directory, "lock", libc::O_RDONLY) {
            Err(e) if create && e.kind() == io::ErrorKind::NotFound => {
                match open_at(
                    &directory,
                    "lock",
                    libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                ) {
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        open_at(&directory, "lock", libc::O_RDONLY)?
                    }
                    result => result?,
                }
            }
            result => result?,
        };
        let meta = lock.metadata()?;
        if !meta.is_file()
            || meta.len() != 0
            || meta.nlink() != 1
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.mode() & 0o077 != 0
        {
            return Err(invalid("invalid cache lock"));
        }
        Ok(Self {
            directory,
            lock,
            eviction: RandomState::new(),
        })
    }

    fn get(&self, key: &Key) -> io::Result<Option<RgbaImage>> {
        let (width, height) = key.dimensions()?;
        let _guard = self.acquire(libc::LOCK_SH)?;
        let mut error = None;
        for slot in key.slots() {
            let record = match self.record(slot) {
                Ok(record) => record,
                Err(e) => {
                    error = Some(e);
                    continue;
                }
            };
            let Some(mut record) = record else {
                continue;
            };
            // Slot selection is only a hint; compare the complete key.
            if record.header[..KEY_LEN] != key.0 {
                continue;
            }
            match Self::pixels(&mut record, key, width, height) {
                Ok(image) => return Ok(Some(image)),
                Err(e) => error = Some(e),
            }
        }
        match error {
            Some(e) => Err(e),
            None => Ok(None),
        }
    }

    // Read at most a bounded header per candidate, one open fd at a time.
    fn record(&self, slot: usize) -> io::Result<Option<Record>> {
        let mut file = match open_at(&self.directory, &format!("{slot:02}.rgba"), libc::O_RDONLY) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            result => result?,
        };
        let meta = file.metadata()?;
        if !meta.is_file()
            || meta.len() < HEADER_LEN as u64
            || meta.len() > (HEADER_LEN + 320 * 240 * 4) as u64
        {
            return Err(invalid("invalid thumbnail record size"));
        }
        let mut header = [0; HEADER_LEN];
        file.read_exact(&mut header)?;
        Ok(Some(Record {
            file,
            header,
            length: meta.len(),
        }))
    }

    fn pixels(record: &mut Record, key: &Key, width: u32, height: u32) -> io::Result<RgbaImage> {
        let length = width as usize * height as usize * 4;
        if record.length != (HEADER_LEN + length) as u64 {
            return Err(invalid("invalid thumbnail record size"));
        }
        let mut pixels = vec![0; length];
        record.file.read_exact(&mut pixels)?;
        if checksum(key, &pixels).to_le_bytes() != record.header[KEY_LEN..] {
            return Err(invalid("thumbnail checksum mismatch"));
        }
        RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| invalid("invalid thumbnail pixels"))
    }

    fn put(&self, key: &Key, image: &RgbaImage) -> io::Result<()> {
        if image.dimensions() != key.dimensions()? {
            return Err(invalid("thumbnail dimensions do not match key"));
        }
        // Never wait for another invocation; hold locks only for bounded cache I/O,
        // not source reads, decoding, or terminal output. Shared readers prevent
        // retaining unlinked records while concurrent writers fill the slots.
        let _guard = self.acquire(libc::LOCK_EX)?;
        let mut available = None;
        let mut existing = None;
        for slot in key.slots() {
            match self.record(slot) {
                Ok(Some(record)) if record.header[..KEY_LEN] == key.0 => {
                    existing = Some(slot);
                    break;
                }
                Ok(Some(_)) => {}
                // A missing/invalid record is preferable to evicting another key.
                _ => {
                    available.get_or_insert(slot);
                }
            }
        }
        let slot = existing
            .or(available)
            .unwrap_or_else(|| key.slots().start + self.eviction.hash_one(key.0) as usize % WAYS);
        remove_at(&self.directory, STAGING)?;
        let result = (|| {
            let mut file = open_at(
                &self.directory,
                STAGING,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )?;
            file.write_all(&key.0)?;
            file.write_all(&checksum(key, image.as_raw()).to_le_bytes())?;
            file.write_all(image.as_raw())?;
            // Disposable cache: atomic publication, no fsync durability promise.
            let from = CString::new(STAGING).unwrap();
            let to = CString::new(format!("{slot:02}.rgba")).unwrap();
            // SAFETY: both names and the directory fd remain valid for the call.
            if unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    from.as_ptr(),
                    self.directory.as_raw_fd(),
                    to.as_ptr(),
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = remove_at(&self.directory, STAGING);
        }
        result
    }

    fn acquire(&self, operation: libc::c_int) -> io::Result<Guard<'_>> {
        // SAFETY: lock owns an open regular-file descriptor.
        if unsafe { libc::flock(self.lock.as_raw_fd(), operation | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Guard(&self.lock))
    }
}

struct Guard<'a>(&'a File);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        // SAFETY: the borrowed file remains open until this guard is dropped.
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn checksum(key: &Key, pixels: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&key.0);
    hasher.update(pixels);
    hasher.finalize()
}

fn open_at(directory: &File, name: &str, flags: libc::c_int) -> io::Result<File> {
    let name = CString::new(name).unwrap();
    // SAFETY: valid directory fd and NUL-terminated internal name. O_NOFOLLOW
    // rejects symlinks; O_NONBLOCK avoids hanging on an unexpected FIFO.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: openat returned a new owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn remove_at(directory: &File, name: &str) -> io::Result<()> {
    let name = CString::new(name).unwrap();
    // SAFETY: valid directory fd and internal name; unlink never follows symlinks.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(error)
    }
}

pub fn clear(path: &Path) -> io::Result<()> {
    // Do not create storage on clear. A missing namespace is already empty;
    // a present namespace with a missing/bad lock must be reported, not skipped.
    match fs::symlink_metadata(path.join(NAMESPACE)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        result => {
            result?;
        }
    }
    let store = Store::open(path, false)?;
    let _guard = store.acquire(libc::LOCK_EX)?;
    for slot in 0..SLOTS {
        remove_at(&store.directory, &format!("{slot:02}.rgba"))?;
    }
    remove_at(&store.directory, STAGING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview;
    use image::Rgba;
    use std::{
        os::unix::fs::{PermissionsExt, symlink},
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/cache-tests")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn source(&self, name: &str, color: u8) -> PathBuf {
            let path = self.0.join(name);
            RgbaImage::from_pixel(8, 4, Rgba([color, 0, 0, 255]))
                .save(&path)
                .unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn record_path(root: &Path, key: &Key) -> PathBuf {
        key.slots()
            .map(|slot| root.join(NAMESPACE).join(format!("{slot:02}.rgba")))
            .find(|path| fs::read(path).is_ok_and(|bytes| bytes.starts_with(&key.0)))
            .expect("record was stored")
    }

    #[test]
    fn hits_match_decode_and_invalidate_edits_replacement_links_and_geometry() {
        let f = Fixture::new();
        let source = f.source("source.bmp", 255);
        let cache_path = f.0.join("cache");
        let mut cache = Cache::new(Some(&cache_path));
        let original = preview::load(&source, 32, 24, &mut cache).unwrap();
        assert_eq!(
            preview::load(&source, 32, 24, &mut cache).unwrap(),
            original
        );
        assert_eq!(
            (cache.stats.hits, cache.stats.misses, cache.stats.writes),
            (1, 1, 1)
        );
        // Same inode/size/mtime, different ctime: restoring mtime cannot hide edits.
        let before = fs::metadata(&source).unwrap();
        f.source("source.bmp", 100);
        File::options()
            .write(true)
            .open(&source)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(before.modified().unwrap()))
            .unwrap();
        let after = fs::metadata(&source).unwrap();
        assert_eq!(
            (before.ino(), before.len(), before.modified().unwrap()),
            (after.ino(), after.len(), after.modified().unwrap())
        );
        assert_ne!(
            preview::load(&source, 32, 24, &mut cache).unwrap(),
            original
        );
        let replacement = f.source("replacement.bmp", 50);
        File::open(&replacement)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(before.modified().unwrap()))
            .unwrap();
        fs::rename(replacement, &source).unwrap();
        let replaced = preview::load(&source, 32, 24, &mut cache).unwrap();
        assert_eq!(replaced.get_pixel(16, 12)[0], 50);
        let link = f.0.join("link.bmp");
        symlink(&source, &link).unwrap();
        assert_eq!(preview::load(&link, 32, 24, &mut cache).unwrap(), replaced);
        assert_eq!(cache.stats.hits, 2);
        let other = f.source("other.bmp", 0);
        fs::remove_file(&link).unwrap();
        symlink(other, &link).unwrap();
        assert_ne!(preview::load(&link, 32, 24, &mut cache).unwrap(), replaced);
        assert_eq!(
            preview::load(&source, 10, 12, &mut cache)
                .unwrap()
                .dimensions(),
            (10, 12)
        );
        assert_eq!(cache.stats.misses, 5);
    }

    #[test]
    fn corruption_truncation_oversize_and_old_version_recover() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let mut cache = Cache::new(Some(&f.0));
        let expected = preview::load(&source, 32, 24, &mut cache).unwrap();
        let key = Key::new(&fs::metadata(source.clone()).unwrap(), 32, 24);
        for damage in 0..4 {
            let record = record_path(&f.0, &key);
            let mut bytes = fs::read(&record).unwrap();
            match damage {
                0 => bytes[HEADER_LEN + 10] ^= 1,
                1 => bytes.truncate(4),
                2 => bytes[7] = b'0', // Obsolete transform version.
                _ => bytes.resize(HEADER_LEN + 320 * 240 * 4 + 1, 0),
            }
            fs::write(&record, bytes).unwrap();
            assert_eq!(
                preview::load(&source, 32, 24, &mut cache).unwrap(),
                expected
            );
            assert_eq!(
                preview::load(&source, 32, 24, &mut cache).unwrap(),
                expected
            );
        }
        assert_eq!(
            (cache.stats.misses, cache.stats.writes, cache.stats.hits),
            (5, 5, 4)
        );
        assert_eq!(cache.stats.errors, 3);
    }

    #[test]
    fn collisions_evict_without_false_hits_and_storage_stays_bounded() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let base = Key::new(&fs::metadata(source).unwrap(), 320, 240);
        let store = Store::open(&f.0, true).unwrap();
        let mut keys = Vec::new();
        for i in 0..256_u64 {
            let mut key = base.clone();
            key.0[16..24].copy_from_slice(&i.to_le_bytes());
            let image = RgbaImage::from_pixel(320, 240, Rgba([i as u8, 0, 0, 255]));
            store.put(&key, &image).unwrap();
            assert_eq!(store.get(&key).unwrap().unwrap(), image);
            keys.push((key, i as u8));
        }
        let mut retained = 0;
        for (key, color) in keys {
            if let Some(image) = store.get(&key).unwrap() {
                assert_eq!(image.get_pixel(0, 0), &Rgba([color, 0, 0, 255]));
                retained += 1;
            }
        }
        assert_eq!(retained, SLOTS);
        let namespace = f.0.join(NAMESPACE);
        // Interrupted writers leave only the single reserved staging pathname.
        fs::write(namespace.join(STAGING), vec![0; HEADER_LEN + 320 * 240 * 4]).unwrap();
        let records: Vec<_> = fs::read_dir(&namespace)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(records.len(), SLOTS + 2); // lock, staging, slots
        assert_eq!(
            records
                .iter()
                .map(|e| e.metadata().unwrap().len())
                .sum::<u64>(),
            STORAGE_LIMIT as u64
        );
        store.put(&base, &RgbaImage::new(320, 240)).unwrap();
        assert!(!namespace.join(STAGING).exists());
        fs::write(namespace.join("keep-me"), b"unrelated").unwrap();
        let lock_inode = fs::metadata(namespace.join("lock")).unwrap().ino();
        clear(&f.0).unwrap();
        assert_eq!(fs::read_dir(namespace.clone()).unwrap().count(), 2);
        assert_eq!(fs::read(namespace.join("keep-me")).unwrap(), b"unrelated");
        assert_eq!(
            fs::metadata(namespace.join("lock")).unwrap().ino(),
            lock_inode
        );
    }

    #[test]
    fn colliding_keys_coexist_and_only_a_full_set_evicts() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let base = Key::new(&fs::metadata(source).unwrap(), 32, 24);
        let store = Store::open(&f.0, true).unwrap();
        let image = RgbaImage::new(32, 24);
        let mut keys = Vec::new();
        for value in 0..10000_u64 {
            let mut key = base.clone();
            key.0[16..24].copy_from_slice(&value.to_le_bytes());
            if key.slots() == base.slots() {
                keys.push(key);
            }
            if keys.len() == WAYS + 1 {
                break;
            }
        }
        assert_eq!(keys.len(), WAYS + 1);
        for key in &keys[..WAYS] {
            store.put(key, &image).unwrap();
        }
        for key in &keys[..WAYS] {
            assert_eq!(store.get(key).unwrap(), Some(image.clone()));
        }
        // Another invocation fills a previously missed key while this one decodes;
        // insertion must replace that key, not consume a second slot for it.
        let concurrent = Store::open(&f.0, true).unwrap();
        concurrent.put(&keys[0], &image).unwrap();
        for key in &keys[..WAYS] {
            assert!(store.get(key).unwrap().is_some());
        }
        store.put(&keys[WAYS], &image).unwrap();
        assert!(store.get(&keys[WAYS]).unwrap().is_some());
        assert_eq!(
            keys.iter()
                .filter(|key| store.get(key).unwrap().is_some())
                .count(),
            WAYS
        );
        // A malformed early candidate cannot mask a valid hit later in the set.
        let first =
            f.0.join(NAMESPACE)
                .join(format!("{:02}.rgba", base.slots().start));
        fs::write(&first, b"broken").unwrap();
        assert_eq!(
            keys.iter()
                .filter(|key| matches!(store.get(key), Ok(Some(_))))
                .count(),
            WAYS - 1
        );
    }

    #[test]
    fn previous_slot_mapping_shares_the_same_bounded_namespace() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let base = Key::new(&fs::metadata(source).unwrap(), 32, 24);
        let store = Store::open(&f.0, true).unwrap();
        let mut previous = std::collections::HashMap::new();
        for value in 0..256_u64 {
            let mut key = base.clone();
            key.0[16..24].copy_from_slice(&value.to_le_bytes());
            let old_slot = crc32fast::hash(&key.0) as usize % SLOTS;
            previous.insert(old_slot, key);
        }
        assert_eq!(previous.len(), SLOTS);
        let image = RgbaImage::from_pixel(32, 24, Rgba([11, 22, 33, 255]));
        // Reproduce the prior binary's valid record layout and direct slot rule.
        for (slot, key) in &previous {
            let mut bytes = key.0.to_vec();
            bytes.extend(checksum(key, image.as_raw()).to_le_bytes());
            bytes.extend(image.as_raw());
            fs::write(f.0.join(NAMESPACE).join(format!("{slot:02}.rgba")), bytes).unwrap();
        }
        for (slot, key) in &previous {
            assert_eq!(
                store.get(key).unwrap().is_some(),
                key.slots().contains(slot)
            );
        }
        for key in previous.values() {
            store.put(key, &image).unwrap();
            assert_eq!(store.get(key).unwrap(), Some(image.clone()));
        }
        assert_eq!(
            fs::read_dir(f.0.join(NAMESPACE)).unwrap().count(),
            SLOTS + 1
        );
        clear(&f.0).unwrap();
        assert_eq!(fs::read_dir(f.0.join(NAMESPACE)).unwrap().count(), 1);
    }

    #[test]
    fn busy_storage_falls_back_and_clear_reports_contention() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let store = Store::open(&f.0, true).unwrap();
        let held = store.acquire(libc::LOCK_EX).unwrap();
        let mut cache = Cache::new(Some(&f.0));
        assert!(preview::load(&source, 32, 24, &mut cache).is_ok());
        assert_eq!(
            (cache.stats.hits, cache.stats.writes, cache.stats.errors),
            (0, 0, 2)
        );
        assert_eq!(clear(&f.0).unwrap_err().kind(), io::ErrorKind::WouldBlock);
        drop(held);
        assert!(preview::load(&source, 32, 24, &mut cache).is_ok());
        assert_eq!(cache.stats.writes, 1);
        let reader = store.acquire(libc::LOCK_SH).unwrap();
        assert!(preview::load(&source, 32, 24, &mut cache).is_ok());
        assert_eq!(cache.stats.hits, 1);
        assert!(clear(&f.0).is_err());
        drop(reader);
        clear(&f.0).unwrap();
    }

    #[test]
    fn readonly_storage_hits_and_misses_preserve_previews() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let mut cache = Cache::new(Some(&f.0));
        let expected = preview::load(&source, 32, 24, &mut cache).unwrap();
        let namespace = f.0.join(NAMESPACE);
        fs::set_permissions(&namespace, fs::Permissions::from_mode(0o500)).unwrap();
        let mut cache = Cache::new(Some(&f.0));
        let hit = preview::load(&source, 32, 24, &mut cache);
        let miss = preview::load(&source, 16, 12, &mut cache);
        let clear_result = clear(&f.0);
        fs::set_permissions(&namespace, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(hit.unwrap(), expected);
        assert!(miss.is_ok() && clear_result.is_err());
        assert_eq!(
            (cache.stats.hits, cache.stats.misses, cache.stats.writes),
            (1, 1, 0)
        );
        assert!(cache.stats.errors > 0);
        // Source permissions are still enforced even with a warm record.
        fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();
        let result = preview::load(&source, 32, 24, &mut cache);
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(result.is_err());
        assert_eq!(cache.stats.hits, 1);
    }

    #[test]
    fn unavailable_storage_and_symlinks_never_redirect_writes_or_clear() {
        let f = Fixture::new();
        let source = f.source("source.png", 255);
        let mut disabled = Cache::new(None);
        let expected = preview::load(&source, 32, 24, &mut disabled).unwrap();
        let unavailable = f.0.join("not-a-directory");
        fs::write(&unavailable, b"preserve").unwrap();
        let mut cache = Cache::new(Some(&unavailable));
        assert_eq!(
            preview::load(&source, 32, 24, &mut cache).unwrap(),
            expected
        );
        assert_eq!(cache.stats.errors, 1);
        assert_eq!(fs::read(&unavailable).unwrap(), b"preserve");
        let mut cache = Cache::new(Some(&f.0));
        preview::load(&source, 32, 24, &mut cache).unwrap();
        let key = Key::new(&fs::metadata(&source).unwrap(), 32, 24);
        let record = record_path(&f.0, &key);
        fs::remove_file(&record).unwrap();
        symlink(&unavailable, &record).unwrap();
        assert_eq!(
            preview::load(&source, 32, 24, &mut cache).unwrap(),
            expected
        );
        assert!(
            !fs::symlink_metadata(&record)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&unavailable).unwrap(), b"preserve");
        let redirected = f.0.join("redirected");
        fs::create_dir(&redirected).unwrap();
        symlink(f.0.join(NAMESPACE), redirected.join(NAMESPACE)).unwrap();
        let mut cache = Cache::new(Some(&redirected));
        assert_eq!(
            preview::load(&source, 32, 24, &mut cache).unwrap(),
            expected
        );
        assert_eq!(cache.stats.writes, 0);
        assert!(clear(&redirected).is_err());
        assert!(record.exists());
        let missing = f.0.join("missing");
        clear(&missing).unwrap();
        assert!(!missing.exists());
    }
}
