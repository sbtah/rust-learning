# Day 15 — Binary I/O and File Formats

**Domain:** databases • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A byte-level record file format from scratch. Each record is a length-prefixed blob with a CRC32 checksum, packed into a file that starts with magic bytes and a version. You'll write records to disk, read them back, and detect corruption. This is the foundation for the whole week — Days 16-21 all build on the format you design today.

## What you'll learn

- Binary vs text I/O and why databases prefer binary
- **Endianness** and why everyone picks little-endian for new formats
- **`BufReader`** and **`BufWriter`** — why you always wrap raw files
- Packing primitive types (`u32`, `u64`) into byte slices via `to_le_bytes`/`from_le_bytes`
- **Length prefixing** — the standard way to store variable-length data
- **CRC32** — detecting corruption cheaply via `crc32fast`
- **Magic bytes** and **versioning** — making formats self-identifying and upgradable
- Detecting EOF cleanly with `read_exact` and `ErrorKind::UnexpectedEof`

## Background

### Why binary?

Python programmers reach for JSON or pickle. Databases don't — they store data as raw bytes in carefully laid-out records. Three reasons:

1. **Size.** A `u64` integer is 8 bytes binary, but up to 20 bytes as ASCII digits.
2. **Speed.** Parsing `"12345"` into an integer means scanning characters, multiplying by 10, checking bounds. Reading a `u64` is one CPU instruction.
3. **Alignment.** Binary formats can be designed so fields sit at fixed offsets — `mmap` the file and you have a data structure directly.

The tradeoff: binary is unreadable without a format spec. You can't `cat` a database file and see what's in it. So database formats document themselves with magic bytes and versions.

### Endianness

When you write the 4-byte integer `0x11223344` to disk, what order do the bytes go in?

- **Big-endian**: `11 22 33 44` — most significant byte first. "Network byte order." Human-readable when dumped as hex.
- **Little-endian**: `44 33 22 11` — least significant byte first. What x86, ARM, and RISC-V all use natively.

For a new format, **pick little-endian**. Every CPU you'll deploy on is little-endian, so reads and writes are zero-cost — the bytes on disk match the bytes in memory. Old network protocols used big-endian, but those decisions were made when mainframes still disagreed.

Rust's integer types have explicit methods:

```rust
let n: u32 = 0x11223344;
let le: [u8; 4] = n.to_le_bytes();   // [0x44, 0x33, 0x22, 0x11]
let be: [u8; 4] = n.to_be_bytes();   // [0x11, 0x22, 0x33, 0x44]

let back = u32::from_le_bytes(le);   // 0x11223344
```

No ambiguity, no platform surprises. Use `to_le_bytes`/`from_le_bytes` throughout this week.

### Buffered I/O

`std::fs::File` implements `Read` and `Write` by calling the OS — one syscall per read or write. Syscalls are expensive (~hundreds of nanoseconds each). If you write 1000 four-byte integers one at a time, that's 1000 syscalls.

`BufWriter<File>` keeps an in-memory buffer (default 8 KB) and flushes to the OS only when full or explicitly asked. 1000 four-byte writes become one syscall. `BufReader<File>` does the opposite — reads 8 KB at a time, then serves small reads from memory.

```rust
use std::io::{BufReader, BufWriter};
use std::fs::File;

let file = File::create("data.bin")?;
let mut writer = BufWriter::new(file);
// ... many small writes ...
writer.flush()?;  // MUST flush before the BufWriter drops, or errors get swallowed
```

One catch: `BufWriter` flushes on drop but **ignores any errors from that flush**. Always call `flush()` explicitly and check the result. We'll come back to this.

### Length-prefixed records

You've got variable-length data (a payload of unknown size) and a file with many of them glued together. How does the reader know where one record ends and the next begins?

Two options in wide use:

- **Delimiters.** Like `\n` for text lines. Doesn't work for binary — your payload could contain any byte.
- **Length prefix.** Write the length first, then that many bytes of payload. The reader reads the length, then reads exactly that many bytes.

Length prefixing is the standard. Almost every binary format uses it: Protobuf, Cap'n Proto, SQLite's page format, Kafka's message format. You'll use `u32` (4 bytes) for lengths, giving a 4 GB max record — more than enough for a KV store.

### CRC32 for integrity

Disks lie. Not often, but often enough to matter. Cosmic rays flip bits. SSDs occasionally return zeros where data used to be. Filesystems can truncate under a power cut.

A CRC (Cyclic Redundancy Check) is a 4-byte checksum computed from a payload. Store the CRC with the payload; when reading, recompute and compare. Mismatch means corruption. CRC32 is the standard choice — fast (~1 ns per byte with hardware instructions) and catches essentially all single-bit errors plus most random noise.

CRC32 is *not* cryptographic. It won't detect an attacker flipping bits on purpose (they could recompute the CRC too). For that you'd use SHA-256 or BLAKE3. For accidental corruption, CRC32 is perfect.

We'll use the `crc32fast` crate, which wraps SIMD-accelerated CRC32 on modern CPUs.

### Magic bytes and versioning

When you open a file of unknown origin, how do you know it's *your* format and not something else? Put a short "magic number" at the start. For example, PNG files begin with `89 50 4E 47 0D 0A 1A 0A`. The `50 4E 47` is ASCII `PNG`. The other bytes catch common text-mode mangling.

Right after the magic, write a version byte. When you change the format later (Day 16), bump the version. Readers check the version and either handle it or error out cleanly instead of silently returning garbage.

Our format:

```
File header (8 bytes):
  [0..4]  magic    b"RKVS"        (Rust Key-Value Store)
  [4..5]  version  0x01
  [5..8]  reserved 0x00 0x00 0x00 (for future flags)

Then, any number of records:
  [0..4]  payload_len  u32 LE
  [4..8]  crc32        u32 LE     (CRC of payload only)
  [8..]   payload      raw bytes
```

Fixed header, self-describing records. Works.

## Setting up

```sh
cargo new rkvs
cd rkvs
cargo add crc32fast
```

Your `Cargo.toml` should look like:

```toml
[package]
name = "rkvs"
version = "0.1.0"
edition = "2021"

[dependencies]
crc32fast = "1"
```

## Step 1 — File header: magic and version

Create `src/format.rs`:

```rust
use std::io::{self, Read, Write};

pub const MAGIC: &[u8; 4] = b"RKVS";
pub const VERSION: u8 = 0x01;
pub const HEADER_LEN: usize = 8;

pub fn write_header<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(MAGIC)?;
    w.write_all(&[VERSION])?;
    w.write_all(&[0, 0, 0])?;  // reserved
    Ok(())
}

pub fn read_header<R: Read>(r: &mut R) -> io::Result<()> {
    let mut buf = [0u8; HEADER_LEN];
    r.read_exact(&mut buf)?;

    if &buf[0..4] != MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not an rkvs file",
        ));
    }
    if buf[4] != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported version: {}", buf[4]),
        ));
    }
    Ok(())
}
```

Notice the generic bounds. `write_header` takes `&mut W: Write`, not `&mut File` specifically. This lets you call it with a `File`, a `BufWriter`, a `Vec<u8>`, or anything else that writes — including tests. Same for `read_header` with `R: Read`.

`read_exact` reads *exactly* N bytes or returns an error. Plain `read` may return fewer bytes on short reads (think: network sockets, slow disks), and handling that correctly is error-prone. Always prefer `read_exact` when you know how many bytes you need.

### Why `io::Error::new(...)` instead of a custom error?

Building the full error hierarchy (like Day 7's `thiserror` approach) is overkill for prototyping. Once we start composing errors from multiple sources in Day 17, we'll add a proper error type. For now, stuffing a message into `io::Error` is fine — they all bubble up to the caller.

## Step 2 — Writing a record

Add to `src/format.rs`:

```rust
pub fn write_record<W: Write>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    let len = payload.len() as u32;
    let crc = crc32fast::hash(payload);

    w.write_all(&len.to_le_bytes())?;
    w.write_all(&crc.to_le_bytes())?;
    w.write_all(payload)?;
    Ok(())
}
```

Three writes: length, CRC, payload. The CRC covers the *payload only* — we don't bother checksumming the length because a corrupt length would fail other sanity checks (read length of 4 GB, read fails at EOF).

`crc32fast::hash(&[u8]) -> u32` is a one-shot helper. There's also an incremental `Hasher` type if you're streaming — we don't need that today.

The `as u32` cast is safe if `payload.len() < 4 GiB`. Realistically this is fine for a KV store. Production-grade code would check explicitly:

```rust
if payload.len() > u32::MAX as usize {
    return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "payload too large",
    ));
}
```

Add that if you want. We'll leave it out for brevity.

## Step 3 — Reading a record

```rust
pub fn read_record<R: Read>(r: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match r.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let len = u32::from_le_bytes(len_buf) as usize;

    let mut crc_buf = [0u8; 4];
    r.read_exact(&mut crc_buf)?;
    let expected_crc = u32::from_le_bytes(crc_buf);

    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;

    let actual_crc = crc32fast::hash(&payload);
    if actual_crc != expected_crc {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CRC mismatch: expected {:08x}, got {:08x}", expected_crc, actual_crc),
        ));
    }

    Ok(Some(payload))
}
```

The return type is `io::Result<Option<Vec<u8>>>`. Three outcomes:

- `Ok(Some(bytes))` — successfully read a record.
- `Ok(None)` — clean EOF. No record left to read.
- `Err(e)` — something went wrong (partial read, bad CRC, I/O error).

The tricky bit is distinguishing "clean EOF" from "EOF in the middle of a record". We do that by catching `UnexpectedEof` *only on the length read*. If we fail to read the length, the file is cleanly done. If we read the length but then fail to read the CRC or payload — that's a truncated record, a real error.

`vec![0u8; len]` allocates a zeroed `Vec<u8>` of exactly `len` bytes. `read_exact` fills it.

## Step 4 — Wire up `main.rs`

Rewrite `src/main.rs`:

```rust
mod format;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use format::{read_header, read_record, write_header, write_record};

fn main() -> std::io::Result<()> {
    let path = "demo.rkvs";

    // Write some records
    {
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);

        write_header(&mut w)?;
        write_record(&mut w, b"hello")?;
        write_record(&mut w, b"world")?;
        write_record(&mut w, b"this is a longer payload with some bytes")?;

        w.flush()?;
    }

    // Read them back
    {
        let file = File::open(path)?;
        let mut r = BufReader::new(file);

        read_header(&mut r)?;

        let mut count = 0;
        while let Some(payload) = read_record(&mut r)? {
            println!("record {}: {:?}", count, std::str::from_utf8(&payload).unwrap_or("<binary>"));
            count += 1;
        }
        println!("read {} records total", count);
    }

    Ok(())
}
```

The two blocks are scoped `{ }` deliberately. When the inner `BufWriter` and `File` go out of scope, they're dropped — closing the file — *before* we try to open it for reading. Without the scope, the `BufWriter` might still hold unflushed bytes when we try to read.

Run it:

```sh
cargo run
```

Expected output:

```
record 0: "hello"
record 1: "world"
record 2: "this is a longer payload with some bytes"
read 3 records total
```

## Step 5 — Verify with a hex dump

On Linux or macOS:

```sh
xxd demo.rkvs
```

You should see something like:

```
00000000: 524b 5653 0100 0000 0500 0000 3610 a3a2  RKVS........6...
00000010: 6865 6c6c 6f05 0000 0095 87c8 4277 6f72  hello.......Bwor
00000020: 6c64 2800 0000 e4a6 b4ff 7468 6973 2069  ld(.........this i
00000030: 7320 6120 6c6f 6e67 6572 2070 6179 6c6f  s a longer paylo
00000040: 6164 2077 6974 6820 736f 6d65 2062 7974  ad with some byt
00000050: 6573                                     es
```

Breaking it down:
- `52 4b 56 53` = `RKVS` magic
- `01` = version
- `00 00 00` = reserved
- `05 00 00 00` = first record length (5, little-endian)
- `36 10 a3 a2` = CRC32 of `"hello"`
- `68 65 6c 6c 6f` = `"hello"`
- Then next record, and so on.

The CRCs on your machine may differ if you change the payloads but the rest of the structure should match exactly. This is what self-documenting binary means — once you know the spec, you can read the file by eye.

## Step 6 — Tests, including deliberate corruption

Add to the bottom of `src/format.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip_empty() {
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf);
        read_header(&mut cursor).unwrap();
        assert!(read_record(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn roundtrip_records() {
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        write_record(&mut buf, b"alpha").unwrap();
        write_record(&mut buf, b"bravo charlie").unwrap();
        write_record(&mut buf, b"").unwrap();  // zero-length record is allowed

        let mut cursor = Cursor::new(buf);
        read_header(&mut cursor).unwrap();

        assert_eq!(read_record(&mut cursor).unwrap().unwrap(), b"alpha");
        assert_eq!(read_record(&mut cursor).unwrap().unwrap(), b"bravo charlie");
        assert_eq!(read_record(&mut cursor).unwrap().unwrap(), b"");
        assert!(read_record(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn bad_magic_rejected() {
        let mut buf = vec![b'W', b'R', b'O', b'N', 0x01, 0, 0, 0];
        let mut cursor = Cursor::new(&mut buf);
        let err = read_header(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn bad_version_rejected() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.push(0xFF);  // future version
        buf.extend_from_slice(&[0, 0, 0]);

        let mut cursor = Cursor::new(buf);
        let err = read_header(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn corrupted_payload_detected() {
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        write_record(&mut buf, b"important data").unwrap();

        // Flip one byte in the payload. Header is 8 bytes, length is 4,
        // CRC is 4, so payload starts at offset 16.
        buf[16] ^= 0x01;

        let mut cursor = Cursor::new(buf);
        read_header(&mut cursor).unwrap();
        let err = read_record(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn truncated_record_detected() {
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        write_record(&mut buf, b"some payload").unwrap();

        // Chop off the last few bytes of payload
        buf.truncate(buf.len() - 3);

        let mut cursor = Cursor::new(buf);
        read_header(&mut cursor).unwrap();
        let err = read_record(&mut cursor).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
```

Six tests covering: round-trip, bad magic, bad version, corrupted payload (CRC catches it), and truncation (detected as unexpected EOF during a record).

`std::io::Cursor` wraps an in-memory `Vec<u8>` as something that implements `Read` and `Write` — perfect for testing binary formats without touching the disk.

Run:

```sh
cargo test
```

Expected:

```
running 6 tests
test format::tests::roundtrip_empty ... ok
test format::tests::bad_magic_rejected ... ok
test format::tests::bad_version_rejected ... ok
test format::tests::roundtrip_records ... ok
test format::tests::corrupted_payload_detected ... ok
test format::tests::truncated_record_detected ... ok

test result: ok. 6 passed
```

All six tests passing means the format works for happy path *and* at least three classes of corruption. Confidence earned.

## Step 7 — Streaming many records

Single records are fine. Realistically, a KV store will write millions. Add a small benchmark harness in `main.rs`:

```rust
fn bench_write_many() -> std::io::Result<()> {
    use std::time::Instant;

    let path = "bench.rkvs";
    let start = Instant::now();

    {
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);
        write_header(&mut w)?;

        let payload = vec![0xABu8; 256];  // 256-byte record
        for _ in 0..100_000 {
            write_record(&mut w, &payload)?;
        }
        w.flush()?;
    }

    let elapsed = start.elapsed();
    let size = std::fs::metadata(path)?.len();
    println!(
        "wrote 100k records ({} bytes) in {:?} — {:.1} MB/s",
        size,
        elapsed,
        (size as f64 / 1_000_000.0) / elapsed.as_secs_f64()
    );
    Ok(())
}
```

Swap `main` to call it:

```rust
fn main() -> std::io::Result<()> {
    bench_write_many()
}
```

Run with release (debug is slow):

```sh
cargo run --release
```

Typical output on an SSD:

```
wrote 100k records (26400008 bytes) in 42.1ms — 626.7 MB/s
```

Numbers vary by machine, but should be hundreds of MB/s. If you dropped the `BufWriter` wrapper and wrote to the raw `File`, this would be tens of MB/s. The buffer matters.

## Common pitfalls

### `BufWriter` eats errors on drop

```rust
{
    let file = File::create("data.bin")?;
    let mut w = BufWriter::new(file);
    write_record(&mut w, b"important")?;
    // forgot to flush!
}  // BufWriter dropped here — attempts to flush but silently ignores errors
```

If the flush fails (disk full, permissions revoked), you won't know — `write_record` returned `Ok` but the bytes may never have hit the disk. Always:

```rust
w.flush()?;
```

Before the writer drops. In production code, also use `w.get_ref().sync_all()?` to force the OS to fsync to disk before considering the write durable. We'll come back to this on Day 20.

### `read_exact` vs `read`

`read` can return short. If you have:

```rust
let mut buf = [0u8; 4];
r.read(&mut buf)?;  // WRONG: may read 0..4 bytes
let n = u32::from_le_bytes(buf);
```

You just read uninitialized data (well, zeros) into `buf` if the underlying stream hiccuped. Always use `read_exact` for fixed-size reads:

```rust
r.read_exact(&mut buf)?;
```

### Endian mismatches

```rust
let n = 42u32;
let bytes = n.to_le_bytes();
// ... written to disk ...

// Later, on a different machine or after a typo:
let read_back = u32::from_be_bytes(bytes);  // WRONG
```

You'll get `0x2A000000` (704,643,072) instead of `42`. Pick one — little-endian — and use it everywhere. Never mix.

### Reading into a `Vec` with wrong length

```rust
let len = 1024;
let mut v: Vec<u8> = Vec::with_capacity(len);
r.read_exact(&mut v)?;  // FAILS: v.len() is 0, so read_exact reads nothing
```

`Vec::with_capacity(n)` reserves space but leaves `len = 0`. `read_exact` reads into the initialized slice, which is empty. You want:

```rust
let mut v = vec![0u8; len];  // initialized to zeros, len == 1024
r.read_exact(&mut v)?;
```

Or use `Vec::resize(len, 0)` if you already have the vec.

## What you learned

- **Binary file formats** are faster and denser than text, but require explicit design.
- **Endianness** matters. Pick little-endian.
- **`to_le_bytes`** / **`from_le_bytes`** pack integers into byte arrays with no ambiguity.
- **`BufReader`** / **`BufWriter`** batch syscalls. Always wrap raw files.
- **Length-prefixed records** are the standard way to pack variable-length data.
- **CRC32** detects accidental corruption cheaply via `crc32fast::hash`.
- **Magic bytes** + **version byte** make formats self-identifying and future-proof.
- **`read_exact`** is almost always what you want; plain `read` is a trap.
- **`BufWriter` swallows errors on drop** — always `flush()?` explicitly.
- **`io::Cursor`** lets you test binary formats with in-memory buffers.

## Exercises

1. **Timestamps.** Add a `u64` timestamp field to each record (nanoseconds since UNIX epoch). Update the CRC to cover both length and timestamp, not just payload. Bump the version to 2 and make `read_record` handle both.
2. **File listing.** Write a `fn list_records(path: &Path) -> io::Result<Vec<(usize, usize)>>` that returns `(offset, length)` pairs for every record without allocating payload buffers. Use `io::Seek::stream_position` to track offsets.
3. **Larger records.** Replace `u32` length with `u64`. What does the hex dump look like now? (Hint: eight bytes per length, sixteen bytes of record header.)
4. **BLAKE3 option.** Add a feature flag `--features strong-hash` that uses BLAKE3 (32 bytes) instead of CRC32 for checksums. You'll need to parameterize the record header. What breaks?
5. **Streaming payloads.** What if a payload is 2 GB and you don't want to hold it all in memory? Design a two-pass writer: first pass, compute the CRC and length; second pass, write bytes. You'll need `Seek` to rewind and patch the header.

## What's next

Day 16 introduces **serde**, Rust's universal serialization framework. Instead of writing payload bytes by hand, you'll derive serialization for strongly-typed structs and use the `bincode` crate to pack them into your record format. You'll also version your serialized structs — so old data keeps working after you add fields.

→ [Day 16 — Serde, Bincode, and Versioned Records](day-16.md)
