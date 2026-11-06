//! Optional filesystem metadata. No file contents are opened, and all variable
//! data is bounded and discarded after one entry. Plain and automatic compact
//! listings never call this module.
use crate::{cli::Options, display::escape};
use std::{
    ffi::{CStr, CString, OsStr},
    io::{self, Write},
    os::unix::ffi::OsStrExt,
    path::Path,
};

const BUFFER_LIMIT: usize = 64 * 1024;
const ENTRY_LIMIT: usize = 256;

fn path_string(path: &Path) -> Option<CString> {
    CString::new(path.as_os_str().as_bytes()).ok()
}

fn list_attributes(path: &CStr, link: bool, buffer: &mut [u8]) -> io::Result<usize> {
    let pointer = if buffer.is_empty() {
        std::ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast()
    };
    // SAFETY: NUL-terminated path and a writable buffer of the stated size.
    let result = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::listxattr(
                path.as_ptr(),
                pointer,
                buffer.len(),
                if link { libc::XATTR_NOFOLLOW } else { 0 },
            )
        }
        #[cfg(target_os = "linux")]
        {
            if link {
                libc::llistxattr(path.as_ptr(), pointer, buffer.len())
            } else {
                libc::listxattr(path.as_ptr(), pointer, buffer.len())
            }
        }
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result as usize)
    }
}

fn attribute(path: &CStr, name: &CStr, link: bool, buffer: &mut [u8]) -> io::Result<usize> {
    let pointer = if buffer.is_empty() {
        std::ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast()
    };
    // SAFETY: NUL-terminated path/name and writable buffer of the stated size.
    let result = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::getxattr(
                path.as_ptr(),
                name.as_ptr(),
                pointer,
                buffer.len(),
                0,
                if link { libc::XATTR_NOFOLLOW } else { 0 },
            )
        }
        #[cfg(target_os = "linux")]
        {
            if link {
                libc::lgetxattr(path.as_ptr(), name.as_ptr(), pointer, buffer.len())
            } else {
                libc::getxattr(path.as_ptr(), name.as_ptr(), pointer, buffer.len())
            }
        }
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result as usize)
    }
}

fn absent(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENOTSUP)) || {
        #[cfg(target_os = "macos")]
        {
            error.raw_os_error() == Some(libc::ENOATTR)
        }
        #[cfg(target_os = "linux")]
        {
            error.raw_os_error() == Some(libc::ENODATA)
        }
    }
}

/// A compact marker; a failed optional probe never conceals the entry or its
/// ordinary stat metadata. Explicit expansions report unavailable metadata.
pub fn marker(path: &Path, link: bool) -> Option<char> {
    let path = path_string(path)?;
    let attrs = list_attributes(&path, link, &mut []).unwrap_or(0) > 0;
    #[cfg(target_os = "macos")]
    {
        if attrs {
            return Some('@');
        }
        mac::Acl::get(&path, link)
            .ok()
            .filter(|acl| acl.has_entries())
            .map(|_| '+')
    }
    #[cfg(target_os = "linux")]
    {
        if !attrs {
            return None;
        }
        let acl = [c"system.posix_acl_access", c"system.posix_acl_default"]
            .iter()
            .any(|name| attribute(&path, name, link, &mut []).unwrap_or(0) > 4);
        Some(if acl { '+' } else { '@' })
    }
}

pub fn write(out: &mut impl Write, path: &Path, link: bool, opts: &Options) -> io::Result<()> {
    if !opts.xattrs && !opts.acl && !opts.context {
        return Ok(());
    }
    let Some(path) = path_string(path) else {
        return writeln!(out, "  metadata: unavailable");
    };
    if opts.xattrs {
        write_attributes(out, &path, link)?;
    }
    if opts.acl {
        write_acl(out, &path, link)?;
    }
    if opts.context {
        write_context(out, &path, link)?;
    }
    Ok(())
}

fn write_attributes(out: &mut impl Write, path: &CStr, link: bool) -> io::Result<()> {
    let count = match list_attributes(path, link, &mut []) {
        Ok(0) => return Ok(()),
        Ok(count) if count <= BUFFER_LIMIT => count,
        Ok(_) => return writeln!(out, "  xattrs: omitted (name list exceeds 64 KiB)"),
        Err(error) if absent(&error) => return Ok(()),
        Err(_) => return writeln!(out, "  xattrs: unavailable"),
    };
    let mut names = vec![0; count];
    let count = match list_attributes(path, link, &mut names) {
        Ok(count) if count <= names.len() => count,
        _ => return writeln!(out, "  xattrs: unavailable (attributes changed)"),
    };
    let mut keys: Vec<_> = names[..count]
        .split(|b| *b == 0)
        .filter(|key| !key.is_empty())
        .take(ENTRY_LIMIT + 1)
        .collect();
    let truncated = keys.len() > ENTRY_LIMIT;
    keys.truncate(ENTRY_LIMIT);
    keys.sort_unstable();
    for key in keys {
        let Ok(name) = CString::new(key) else {
            continue;
        };
        let size = attribute(path, &name, link, &mut []);
        let label = escape(OsStr::from_bytes(key));
        match size {
            Ok(size) => writeln!(out, "  {label}\t{size} bytes")?,
            Err(_) => writeln!(out, "  {label}\t? bytes")?,
        }
    }
    if truncated {
        writeln!(out, "  xattrs: remaining attributes omitted (limit 256)")?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn read_attribute(path: &CStr, name: &CStr, link: bool) -> io::Result<Vec<u8>> {
    let count = attribute(path, name, link, &mut [])?;
    if count > BUFFER_LIMIT {
        return Err(io::Error::other("attribute exceeds 64 KiB"));
    }
    let mut value = vec![0; count];
    if count != 0 {
        let read = attribute(path, name, link, &mut value)?;
        if read > count {
            return Err(io::Error::other("attribute changed"));
        }
        value.truncate(read);
    }
    Ok(value)
}

#[cfg(target_os = "linux")]
fn write_context(out: &mut impl Write, path: &CStr, link: bool) -> io::Result<()> {
    let context = match read_attribute(path, c"security.selinux", link) {
        Ok(mut value) => {
            while value.last() == Some(&0) {
                value.pop();
            }
            escape(OsStr::from_bytes(&value))
        }
        Err(error) if absent(&error) => "-".into(),
        Err(_) => "?".into(),
    };
    writeln!(out, "  context: {context}")
}
#[cfg(target_os = "macos")]
fn write_context(out: &mut impl Write, _path: &CStr, _link: bool) -> io::Result<()> {
    writeln!(out, "  context: -")
}

#[cfg(target_os = "linux")]
fn write_acl(out: &mut impl Write, path: &CStr, link: bool) -> io::Result<()> {
    for (name, prefix) in [
        (c"system.posix_acl_access", ""),
        (c"system.posix_acl_default", "default:"),
    ] {
        match read_attribute(path, name, link) {
            Ok(bytes) => write_posix_acl(out, &bytes, prefix)?,
            Err(error) if absent(&error) => (),
            Err(_) => writeln!(out, "  {prefix}acl: unavailable")?,
        }
    }
    Ok(())
}

// Linux's public POSIX ACL xattr wire format: little-endian version 2 followed
// by eight-byte (tag, permissions, id) records. No libacl dependency is needed.
#[cfg(any(target_os = "linux", test))]
fn write_posix_acl(out: &mut impl Write, bytes: &[u8], prefix: &str) -> io::Result<()> {
    if bytes.len() < 4 || bytes[..4] != 2_u32.to_le_bytes() || !(bytes.len() - 4).is_multiple_of(8)
    {
        return writeln!(out, "  {prefix}acl: unavailable (invalid ACL)");
    }
    for entry in bytes[4..].as_chunks::<8>().0.iter().take(ENTRY_LIMIT) {
        let tag = u16::from_le_bytes([entry[0], entry[1]]);
        let permissions = u16::from_le_bytes([entry[2], entry[3]]);
        let id = u32::from_le_bytes(entry[4..8].try_into().unwrap());
        let label = match tag {
            1 => "user:".into(),
            2 => format!("user:{id}"),
            4 => "group:".into(),
            8 => format!("group:{id}"),
            16 => "mask:".into(),
            32 => "other:".into(),
            _ => format!("tag-{tag}:{id}"),
        };
        writeln!(
            out,
            "  {prefix}{label}:{}{}{}",
            if permissions & 4 != 0 { 'r' } else { '-' },
            if permissions & 2 != 0 { 'w' } else { '-' },
            if permissions & 1 != 0 { 'x' } else { '-' }
        )?;
    }
    if (bytes.len() - 4) / 8 > ENTRY_LIMIT {
        writeln!(out, "  {prefix}acl: remaining entries omitted (limit 256)")?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn write_acl(out: &mut impl Write, path: &CStr, link: bool) -> io::Result<()> {
    match mac::Acl::get(path, link) {
        Ok(acl) => acl.write(out),
        // Darwin reports ENOENT when FILESEC_ACL is absent, even for an
        // existing ordinary file. The listing's stat reports missing files.
        Err(error) if absent(&error) || error.raw_os_error() == Some(libc::ENOENT) => Ok(()),
        Err(_) => writeln!(out, "  acl: unavailable"),
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::ffi::{c_int, c_void};
    // Darwin ACLs are opaque and libc-rs does not expose these functions.
    // Apple's sys/acl.h fixes ACL_TYPE_EXTENDED=0x100, FIRST=0, NEXT=-1,
    // qualifiers as 16-byte UUIDs, and ACL_MAX_ENTRIES=128.
    unsafe extern "C" {
        fn acl_get_file(path: *const libc::c_char, kind: c_int) -> *mut c_void;
        fn acl_get_link_np(path: *const libc::c_char, kind: c_int) -> *mut c_void;
        fn acl_free(value: *mut c_void) -> c_int;
        fn acl_get_entry(acl: *mut c_void, id: c_int, entry: *mut *mut c_void) -> c_int;
        fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
        fn acl_get_qualifier(entry: *mut c_void) -> *mut c_void;
        fn acl_get_permset_mask_np(entry: *mut c_void, mask: *mut u64) -> c_int;
        fn acl_get_flagset_np(entry: *mut c_void, flags: *mut *mut c_void) -> c_int;
        fn acl_get_flag_np(flags: *mut c_void, flag: c_int) -> c_int;
    }
    pub(super) struct Acl(*mut c_void);
    impl Drop for Acl {
        fn drop(&mut self) {
            unsafe {
                acl_free(self.0);
            }
        }
    }
    impl Acl {
        pub(super) fn get(path: &CStr, link: bool) -> io::Result<Self> {
            let acl = unsafe {
                if link {
                    acl_get_link_np(path.as_ptr(), 0x100)
                } else {
                    acl_get_file(path.as_ptr(), 0x100)
                }
            };
            if acl.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self(acl))
            }
        }
        pub(super) fn has_entries(&self) -> bool {
            let mut entry = std::ptr::null_mut();
            unsafe { acl_get_entry(self.0, 0, &mut entry) == 0 }
        }
        pub(super) fn write(&self, out: &mut impl Write) -> io::Result<()> {
            for index in 0..128 {
                let mut entry = std::ptr::null_mut();
                if unsafe { acl_get_entry(self.0, if index == 0 { 0 } else { -1 }, &mut entry) }
                    != 0
                {
                    break;
                }
                let mut tag = 0;
                let mut perms = 0;
                let mut flags = std::ptr::null_mut();
                let qualifier = unsafe { acl_get_qualifier(entry) };
                if qualifier.is_null() {
                    writeln!(out, "  {index}: acl: unavailable")?;
                    continue;
                }
                // SAFETY: a successful Darwin qualifier is a 16-byte UUID;
                // copy before freeing it. Other values remain owned by Acl.
                let uuid: [u8; 16] = unsafe { std::ptr::read(qualifier.cast()) };
                unsafe {
                    acl_free(qualifier);
                }
                if unsafe { acl_get_tag_type(entry, &mut tag) } != 0
                    || unsafe { acl_get_permset_mask_np(entry, &mut perms) } != 0
                {
                    writeln!(out, "  {index}: acl: unavailable")?;
                    continue;
                }
                write!(out, "  {index}: uuid:")?;
                for (i, byte) in uuid.iter().enumerate() {
                    if [4, 6, 8, 10].contains(&i) {
                        write!(out, "-")?;
                    }
                    write!(out, "{byte:02x}")?;
                }
                write!(
                    out,
                    " {}",
                    if tag == 1 {
                        "allow"
                    } else if tag == 2 {
                        "deny"
                    } else {
                        "unknown"
                    }
                )?;
                let mut separator = " ";
                for (bit, name) in [
                    (1, "read"),
                    (2, "write"),
                    (3, "execute"),
                    (4, "delete"),
                    (5, "append"),
                    (6, "delete_child"),
                    (7, "readattr"),
                    (8, "writeattr"),
                    (9, "readextattr"),
                    (10, "writeextattr"),
                    (11, "readsecurity"),
                    (12, "writesecurity"),
                    (13, "chown"),
                    (20, "synchronize"),
                ] {
                    if perms & (1 << bit) != 0 {
                        write!(out, "{separator}{name}")?;
                        separator = ",";
                    }
                }
                if unsafe { acl_get_flagset_np(entry, &mut flags) } == 0 {
                    for (bit, name) in [
                        (4, "inherited"),
                        (5, "file_inherit"),
                        (6, "directory_inherit"),
                        (7, "limit_inherit"),
                        (8, "only_inherit"),
                    ] {
                        if unsafe { acl_get_flag_np(flags, 1 << bit) } == 1 {
                            write!(out, "{separator}{name}")?;
                            separator = ",";
                        }
                    }
                }
                writeln!(out)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn posix_acls_decode_ids_masks_and_default_entries() {
        let mut bytes = 2_u32.to_le_bytes().to_vec();
        for (tag, perms, id) in [(1_u16, 7_u16, u32::MAX), (2, 4, 1234), (16, 5, u32::MAX)] {
            bytes.extend(tag.to_le_bytes());
            bytes.extend(perms.to_le_bytes());
            bytes.extend(id.to_le_bytes());
        }
        let mut out = Vec::new();
        write_posix_acl(&mut out, &bytes, "default:").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "  default:user::rwx\n  default:user:1234:r--\n  default:mask::r-x\n"
        );
    }
    #[test]
    fn malformed_and_oversized_acls_are_bounded() {
        for bytes in [vec![], vec![0; 4], vec![2, 0, 0, 0, 1]] {
            let mut out = Vec::new();
            write_posix_acl(&mut out, &bytes, "").unwrap();
            assert!(String::from_utf8(out).unwrap().contains("invalid ACL"));
        }
        let mut bytes = 2_u32.to_le_bytes().to_vec();
        bytes.resize(4 + 8 * 300, 0);
        let mut out = Vec::new();
        write_posix_acl(&mut out, &bytes, "").unwrap();
        assert_eq!(out.iter().filter(|&&c| c == b'\n').count(), ENTRY_LIMIT + 1);
    }
}
