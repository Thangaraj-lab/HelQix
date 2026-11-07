//! A minimal ZIP writer: every file stored without compression, names in UTF-8, a fixed time stamp (1980-01-01) so the same files give the same
//! bytes. It exists because an evidence package is a handful of small files that a person downloads once; it avoids a dependency for that.
//! The format is the classic ZIP (no ZIP64): at most 65 535 files and 4 GiB in all, and a bigger package is refused, not truncated.

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// 1980-01-01, the earliest date ZIP can hold.
const DOS_DATE: u16 = 0x0021;

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 { c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }; }
        *slot = c;
    }
    table
}

/// CRC-32 (the ZIP / PNG polynomial).
pub fn crc32(bytes: &[u8]) -> u32 {
    let table = crc32_table();
    !bytes.iter().fold(!0u32, |c, b| table[((c ^ u32::from(*b)) & 0xFF) as usize] ^ (c >> 8))
}

fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.len() <= 512 && !name.starts_with('/') && !name.contains('\\') && !name.contains('\0') && name.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

/// A ZIP archive holding `files` (name, bytes) in the given order.
pub fn zip_stored(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    if files.len() > 0xFFFF { return Err("too many files for a ZIP archive".into()); }
    let mut seen = std::collections::BTreeSet::new();
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    for (name, data) in files {
        if !name_ok(name) { return Err(format!("{name:?} is not a safe file name for an archive")); }
        if !seen.insert(name.as_str()) { return Err(format!("{name:?} appears twice")); }
        let size = u32::try_from(data.len()).map_err(|_| "a file is too large for a ZIP archive".to_owned())?;
        let offset = u32::try_from(out.len()).map_err(|_| "the archive is too large".to_owned())?;
        let crc = crc32(data);
        let len = name.len() as u16;
        out.extend(LOCAL.to_le_bytes());
        out.extend([20, 0]);                  // version needed
        out.extend(0x0800u16.to_le_bytes());   // flags: UTF-8 names
        out.extend(0u16.to_le_bytes());        // method: stored
        out.extend(0u16.to_le_bytes());        // time
        out.extend(DOS_DATE.to_le_bytes());
        out.extend(crc.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend(0u16.to_le_bytes());        // extra length
        out.extend(name.as_bytes());
        out.extend(data);

        central.extend(CENTRAL.to_le_bytes());
        central.extend([20, 3]);               // made by: version 2.0, UNIX (so the permission bits below apply)
        central.extend([20, 0]);
        central.extend(0x0800u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(DOS_DATE.to_le_bytes());
        central.extend(crc.to_le_bytes());
        central.extend(size.to_le_bytes());
        central.extend(size.to_le_bytes());
        central.extend(len.to_le_bytes());
        central.extend(0u16.to_le_bytes());    // extra
        central.extend(0u16.to_le_bytes());    // comment
        central.extend(0u16.to_le_bytes());    // disk
        central.extend(0u16.to_le_bytes());    // internal attributes
        central.extend((0o100_644u32 << 16).to_le_bytes()); // a regular file, rw-r--r--
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let cd_offset = u32::try_from(out.len()).map_err(|_| "the archive is too large".to_owned())?;
    let cd_size = u32::try_from(central.len()).map_err(|_| "the archive is too large".to_owned())?;
    out.extend(central);
    out.extend(END.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend(cd_size.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());            // comment length
    if u32::try_from(out.len()).is_err() { return Err("the archive is too large".into()); }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An independent, bit-at-a-time CRC-32 (the table version above must agree with it).
    fn crc_slow(bytes: &[u8]) -> u32 {
        let mut c = !0u32;
        for b in bytes { c ^= u32::from(*b); for _ in 0..8 { c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }; } }
        !c
    }

    fn u16le(b: &[u8], at: usize) -> usize { u16::from_le_bytes([b[at], b[at + 1]]) as usize }
    fn u32le(b: &[u8], at: usize) -> usize { u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize }

    /// Reads an archive back through its central directory only (what an extractor does) and checks each file's CRC.
    fn read_back(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let end = zip.len() - 22;
        assert_eq!(u32le(zip, end) as u32, END);
        let (count, cd_size, cd_offset) = (u16le(zip, end + 10), u32le(zip, end + 12), u32le(zip, end + 16));
        assert_eq!(cd_offset + cd_size, end, "the central directory ends where the end record starts");
        let mut at = cd_offset;
        let mut out = Vec::new();
        for _ in 0..count {
            assert_eq!(u32le(zip, at) as u32, CENTRAL);
            let (crc, size, nlen, local) = (u32le(zip, at + 16), u32le(zip, at + 24), u16le(zip, at + 28), u32le(zip, at + 42));
            let name = String::from_utf8(zip[at + 46..at + 46 + nlen].to_vec()).unwrap();
            assert_eq!(u32le(zip, local) as u32, LOCAL);
            let (lnlen, lextra) = (u16le(zip, local + 26), u16le(zip, local + 28));
            assert_eq!(&zip[local + 30..local + 30 + lnlen], name.as_bytes(), "the local header names the same file");
            let data = zip[local + 30 + lnlen + lextra..local + 30 + lnlen + lextra + size].to_vec();
            assert_eq!(crc_slow(&data) as usize, crc, "{name}: stored CRC matches the bytes");
            out.push((name, data));
            at += 46 + nlen;
        }
        assert_eq!(at, end);
        out
    }

    #[test]
    fn crc32_matches_the_standard_check_value_and_the_slow_version() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
        let blob: Vec<u8> = (0..5000u32).map(|i| (i * 7 + 3) as u8).collect();
        assert_eq!(crc32(&blob), crc_slow(&blob));
    }

    #[test]
    fn files_come_back_with_their_names_and_bytes() {
        let files = vec![("dir/a.json".to_owned(), b"{\"a\":1}".to_vec()), ("dir/b.bin".to_owned(), (0..=255u8).collect()), ("empty.txt".to_owned(), vec![]), ("naïve.txt".to_owned(), b"utf8".to_vec())];
        let zip = zip_stored(&files).unwrap();
        assert_eq!(read_back(&zip), files);
    }

    #[test]
    fn the_same_files_give_the_same_bytes() {
        let files = vec![("a".to_owned(), b"x".to_vec())];
        assert_eq!(zip_stored(&files).unwrap(), zip_stored(&files).unwrap());
    }

    #[test]
    fn unsafe_or_repeated_names_are_refused() {
        for bad in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a\\b", "./a", "a/"] {
            assert!(zip_stored(&[(bad.to_owned(), vec![])]).is_err(), "{bad:?}");
        }
        assert!(zip_stored(&[("a".into(), vec![]), ("a".into(), vec![])]).is_err());
    }
}
