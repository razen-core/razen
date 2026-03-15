# Razen Standard Library — `std`

The standard library provides modules for I/O, file system, networking, math, time,
concurrency, and more. All modules are opt-in — import what you need with `use`.

**Prelude types** (`option`, `result`, `vec`, `map`, `set`, `println`, etc.) are always
available without `use`. See `prelude.md` for the complete prelude reference.

---

## Module Index

| Module             | Purpose                                     |
|--------------------|---------------------------------------------|
| `std.io`           | Buffered I/O, stdin/stdout/stderr streams   |
| `std.fs`           | File system — read, write, path operations  |
| `std.path`         | Path manipulation and normalization         |
| `std.net`          | TCP, UDP, HTTP client and server            |
| `std.http`         | High-level HTTP client                      |
| `std.math`         | Mathematical constants and functions        |
| `std.time`         | Timestamps, durations, sleep                |
| `std.process`      | Process control, environment, arguments     |
| `std.env`          | Environment variables                       |
| `std.thread`       | OS threads and join handles                 |
| `std.sync`         | Mutex, RwLock, Channel, atomic types        |
| `std.rand`         | Random number generation                    |
| `std.json`         | JSON serialization and deserialization      |
| `std.regex`        | Regular expressions                         |
| `std.hash`         | Hashing algorithms (SHA256, MD5, etc.)      |
| `std.base64`       | Base64 encoding and decoding                |
| `std.collections`  | Extra collection algorithms                 |
| `std.string`       | Extra string utilities                      |
| `std.bytes`        | Byte buffer manipulation                    |
| `std.mem`          | Memory utilities (size, alignment)          |
| `std.ffi`          | Foreign function interface helpers          |

---

## std.io

Buffered I/O streams. For simple output, use the prelude `println` / `print` instead.

```razen
use std.io { BufReader, BufWriter, stdin, stdout, stderr }

// Buffered reading from stdin
mut reader := BufReader.new(stdin())

loop let some(line) = reader.read_line() {
    process(line)
}

// Buffered writing
mut writer := BufWriter.new(stdout())
writer.write("output line\n")
writer.flush()

// Read all stdin at once
data := stdin().read_to_string().unwrap()
```

### `BufReader`

```razen
BufReader.new(source)                   // wrap any readable source
reader.read_line()                       // option[str] — none at EOF
reader.read_all()                        // result[str, str]
reader.read_bytes(n: int)               // result[bytes, str]
reader.lines()                           // Iterator[str]
reader.bytes()                           // Iterator[u8]
```

### `BufWriter`

```razen
BufWriter.new(sink)                     // wrap any writable sink
writer.write(s: str)                    // result[void, str]
writer.write_bytes(b: bytes)            // result[void, str]
writer.write_line(s: str)              // result[void, str]
writer.flush()                           // result[void, str]
```

---

## std.fs

File system operations.

```razen
use std.fs { read, write, append, exists, remove, rename, copy,
             create_dir, remove_dir, read_dir, File, DirEntry }

// Simple read / write
content := std.fs.read("config.json").unwrap()
std.fs.write("output.txt", "hello world").unwrap()
std.fs.append("log.txt", "new log entry\n").unwrap()

// Check and delete
if std.fs.exists("temp.txt") {
    std.fs.remove("temp.txt").unwrap()
}

// Rename / copy
std.fs.rename("old.txt", "new.txt").unwrap()
std.fs.copy("source.txt", "backup.txt").unwrap()

// Directory operations
std.fs.create_dir("output").unwrap()
std.fs.create_dir_all("a/b/c").unwrap()    // creates intermediate dirs
std.fs.remove_dir("temp").unwrap()
std.fs.remove_dir_all("output").unwrap()   // recursive

// List directory
entries := std.fs.read_dir(".").unwrap()
loop entry in entries {
    println("{entry.name()} — {entry.size()} bytes")
}
```

### `File`

Low-level file handle for streaming reads and writes.

```razen
// Open modes
mut f := File.open("data.txt").unwrap()           // read-only
mut f := File.create("output.txt").unwrap()       // write, truncate
mut f := File.append("log.txt").unwrap()          // write, append
mut f := File.open_rw("data.bin").unwrap()        // read + write

// Always close with defer
defer f.close()

// Read
line := f.read_line()                  // option[str]
all  := f.read_to_string().unwrap()    // str
buf  := f.read_bytes(1024).unwrap()    // bytes

// Write
f.write("hello\n").unwrap()
f.write_bytes(buf).unwrap()
f.flush().unwrap()

// Seek
f.seek(0)                              // seek to position
f.seek_end(0)                          // seek to end
pos := f.position()                    // current position
size := f.size()                       // file size in bytes
```

### `DirEntry`

```razen
entry.name()        // str — file/dir name
entry.path()        // str — full path
entry.size()        // uint — size in bytes
entry.is_file()     // bool
entry.is_dir()      // bool
entry.modified()    // result[Timestamp, str]
```

---

## std.path

Path string manipulation.

```razen
use std.path { join, basename, dirname, extension, stem,
               abs_path, normalize, is_abs, Path }

// Joining paths — handles separators correctly on all platforms
full := std.path.join("home", "user", "docs", "file.txt")
// "home/user/docs/file.txt"  (Linux/macOS)
// "home\user\docs\file.txt"  (Windows)

// Decompose
base := std.path.basename("/home/user/file.txt")    // "file.txt"
dir  := std.path.dirname("/home/user/file.txt")     // "/home/user"
ext  := std.path.extension("archive.tar.gz")        // "gz"
stem := std.path.stem("archive.tar.gz")             // "archive.tar"

// Canonicalize
abs  := std.path.abs_path("../docs/file.txt").unwrap()
norm := std.path.normalize("a/./b/../c")            // "a/c"

// Check
is_a := std.path.is_abs("/home/user")               // true

// Path builder
p := Path.new("/home")
    .join("user")
    .join("docs")
    .with_extension("txt")
println(p.to_string())    // "/home/user/docs.txt"
```

---

## std.net

Low-level TCP and UDP networking.

```razen
use std.net { TcpListener, TcpStream, UdpSocket, SocketAddr }

// TCP server
listener := TcpListener.bind("0.0.0.0:8080").unwrap()
println("Listening on port 8080")

loop {
    match listener.accept() {
        ok((stream, addr)) -> {
            println("Connection from {addr}")
            handle_client(stream)
        },
        err(e) -> eprintln("Accept error: {e}"),
    }
}

// TCP client
mut stream := TcpStream.connect("api.razen.dev:443").unwrap()
defer stream.close()

stream.write("GET / HTTP/1.0\r\n\r\n").unwrap()
response := stream.read_to_string().unwrap()
println(response)

// UDP socket
mut sock := UdpSocket.bind("0.0.0.0:9000").unwrap()
sock.send_to("hello", "127.0.0.1:9001").unwrap()

(data, from_addr) := sock.recv_from().unwrap()
println("Got '{data}' from {from_addr}")
```

---

## std.http

High-level HTTP client. Uses async.

```razen
use std.http { get, post, put, delete, patch, Request, Response, Client }

// Simple GET
async act fetch_user(id: int) result[str, str] {
    res := std.http.get("https://api.razen.dev/users/{id}").await?
    ok(res.body)
}

// GET with headers
async act fetch_auth(url: str, token: str) result[str, str] {
    res := std.http.get(url)
        .header("Authorization", "Bearer {token}")
        .header("Accept", "application/json")
        .send()
        .await?
    ok(res.body)
}

// POST JSON
async act create_user(user: User) result[User, str] {
    body := std.json.to_json(user)?
    res  := std.http.post("https://api.razen.dev/users")
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?
    std.json.parse_json[User](res.body)
}

// Response
res.status           // int — HTTP status code
res.body             // str — response body
res.headers          // map[str, str]
res.ok()             // bool — status 200-299
res.text()           // str — alias for body
res.json[T]()        // result[T, str] — parse body as JSON
```

---

## std.math

Mathematical constants and functions.

```razen
use std.math

// Constants
std.math.PI        // 3.14159265358979323846
std.math.E         // 2.71828182845904523536
std.math.TAU       // 2 * PI = 6.28318...
std.math.PHI       // Golden ratio = 1.61803...
std.math.SQRT2     // √2 = 1.41421...
std.math.LN2       // ln(2) = 0.69314...
std.math.LN10      // ln(10) = 2.30258...
std.math.INF       // +∞
std.math.NEG_INF   // -∞
std.math.NAN       // Not-a-Number

// Basic
std.math.abs(x)             // |x|          — works on int, float, f32
std.math.min(a, b)          // smaller of a, b
std.math.max(a, b)          // larger of a, b
std.math.clamp(x, lo, hi)   // lo if x<lo, hi if x>hi, else x
std.math.sign(x)            // -1, 0, or 1

// Rounding
std.math.floor(x)           // round down
std.math.ceil(x)            // round up
std.math.round(x)           // round to nearest
std.math.trunc(x)           // truncate toward zero
std.math.frac(x)            // fractional part

// Powers and roots
std.math.sqrt(x)            // √x
std.math.cbrt(x)            // ∛x
std.math.pow(base, exp)     // base^exp (float version)
std.math.exp(x)             // e^x
std.math.exp2(x)            // 2^x

// Logarithms
std.math.ln(x)              // natural log
std.math.log2(x)            // log base 2
std.math.log10(x)           // log base 10
std.math.log(x, base)       // log with custom base

// Trigonometry (radians)
std.math.sin(x)
std.math.cos(x)
std.math.tan(x)
std.math.asin(x)
std.math.acos(x)
std.math.atan(x)
std.math.atan2(y, x)        // angle from origin to (x, y)
std.math.sinh(x)
std.math.cosh(x)
std.math.tanh(x)

// Degree helpers
std.math.to_radians(deg)    // degrees → radians
std.math.to_degrees(rad)    // radians → degrees

// Float checks
std.math.is_nan(x)          // bool
std.math.is_inf(x)          // bool
std.math.is_finite(x)       // bool
std.math.is_normal(x)       // bool — not NaN, Inf, zero, or subnormal

// Integer math
std.math.gcd(a, b)          // greatest common divisor
std.math.lcm(a, b)          // least common multiple
std.math.factorial(n)       // n!
std.math.is_prime(n)        // bool
std.math.next_power_of_2(n) // next power of 2 >= n

// Short-form import
use std.math { PI, sqrt, sin, cos, atan2 }
area := PI * r * r
```

---

## std.time

Time, timestamps, and durations.

```razen
use std.time { now, sleep, Duration, Timestamp, Instant }

// Current time
ts  := std.time.now()           // Timestamp (wall clock)
ins := std.time.instant()       // Instant (monotonic — for timing)

// Sleep
std.time.sleep(Duration.millis(100))    // sleep 100ms
std.time.sleep(Duration.secs(2))        // sleep 2s
std.time.sleep(Duration.micros(500))    // sleep 500μs

// Duration constructors
Duration.nanos(n)
Duration.micros(n)
Duration.millis(n)
Duration.secs(n)
Duration.mins(n)
Duration.hours(n)
Duration.days(n)

// Duration operations
d1 := Duration.secs(5)
d2 := Duration.millis(500)
d3 := d1 + d2             // 5.5 seconds
d4 := d1 - d2             // 4.5 seconds
ms := d1.as_millis()      // 5000
ns := d1.as_nanos()       // 5_000_000_000

// Timing code
start := std.time.instant()
heavy_computation()
elapsed := start.elapsed()
println("Took {elapsed.as_millis()}ms")

// Timestamp
ts.unix_secs()            // int — seconds since Unix epoch
ts.unix_millis()          // int — milliseconds since epoch
ts.unix_nanos()           // int — nanoseconds since epoch
ts.to_string()            // "2025-06-15T10:30:00Z"
ts.format("%Y-%m-%d")     // "2025-06-15"

// Timestamp arithmetic
tomorrow := ts + Duration.days(1)
diff      := ts2 - ts1    // Duration
```

---

## std.process

Process lifecycle, arguments, and environment.

```razen
use std.process { exit, args, env, Command }

// Exit with code
std.process.exit(0)     // success
std.process.exit(1)     // general error — returns never

// Command line arguments (vec[str])
all_args := std.process.args()
// all_args[0] is the program name
// all_args[1..] are the user arguments

if all_args.len() < 2 {
    eprintln("Usage: {all_args[0]} <input_file>")
    std.process.exit(1)
}
input_path := all_args[1]

// Environment variables
use std.env { get, set, remove, all }

home     := std.env.get("HOME").unwrap_or("/tmp")
path     := std.env.get("PATH").unwrap_or("")
all_env  := std.env.all()    // map[str, str]

std.env.set("MY_VAR", "hello")
std.env.remove("TEMP")

// Spawn a subprocess
use std.process { Command }

output := Command.new("git")
    .arg("log")
    .arg("--oneline")
    .arg("-10")
    .run()
    .unwrap()

println(output.stdout)
println(output.stderr)
exit_code := output.status    // int
```

---

## std.thread

OS threads for CPU-bound parallelism.
For I/O-bound work, prefer `async`/`fork`.

```razen
use std.thread { spawn, sleep, current, JoinHandle }

// Spawn a thread
handle: JoinHandle[int] := std.thread.spawn(|| {
    heavy_computation()    // runs on a new OS thread
    42                     // return value
})

// Do other work while thread runs
other_work()

// Wait for thread and get result
result := handle.join().unwrap()    // 42

// Spawn multiple threads
handles := (0..8).map(|i| {
    std.thread.spawn(|| process_chunk(i))
}).collect[vec[JoinHandle[void]]]()

loop h in handles {
    h.join().unwrap()
}

// Thread sleep (use std.time.sleep in most cases)
std.thread.sleep(Duration.millis(50))

// Get current thread info
id := std.thread.current().id()
```

---

## std.sync

Synchronization primitives for shared mutable state across threads.
For `async`/`fork`, prefer `shared` values which use automatic atomic ARC.

```razen
use std.sync { Mutex, RwLock, Channel, channel, AtomicInt, AtomicBool }

// Mutex — exclusive access
shared counter: Mutex[int] = Mutex.new(0)

// Lock, modify, auto-unlock
{
    mut guard := counter.lock().unwrap()
    guard.value += 1
}    // lock released here

// RwLock — multiple readers, one writer
shared data: RwLock[vec[str]] = RwLock.new(vec[])

// Multiple concurrent readers
reader := data.read().unwrap()
println(reader.value.len())    // read-only access

// Exclusive write
{
    mut writer := data.write().unwrap()
    writer.value.push("new item")
}

// Channel — message passing between threads
(sender, receiver) := std.sync.channel[str]()

// Send from one thread
std.thread.spawn(|| {
    sender.send("hello").unwrap()
    sender.send("world").unwrap()
    sender.close()
})

// Receive in this thread
loop let some(msg) = receiver.recv() {
    println(msg)
}

// Atomic types — lock-free primitives
shared count: AtomicInt = AtomicInt.new(0)
count.fetch_add(1)
count.fetch_sub(1)
val := count.load()

shared flag: AtomicBool = AtomicBool.new(false)
flag.store(true)
is_set := flag.load()
```

---

## std.json

JSON parsing and serialization.

```razen
use std.json { parse_json, to_json, to_json_pretty }

// Parse JSON string → Razen type
// Requires @derive[Debug, Clone, Eq] on the struct (or manual impl)
@derive[Debug, Clone]
struct User {
    id:        int,
    name:      str,
    email:     str,
    is_active: bool,
}

json_str := "{\"id\": 1, \"name\": \"Alice\", \"email\": \"alice@razen.dev\", \"is_active\": true}"
user := std.json.parse_json[User](json_str).unwrap()
println(user.name)    // "Alice"

// Parse into generic map for unknown shapes
raw := std.json.parse_json[map[str, str]](json_str).unwrap()
println(raw["name"])    // "Alice"

// Serialize Razen type → JSON string
json_out := std.json.to_json(user).unwrap()
println(json_out)
// {"id":1,"name":"Alice","email":"alice@razen.dev","is_active":true}

// Pretty-print JSON
pretty := std.json.to_json_pretty(user).unwrap()
println(pretty)
// {
//   "id": 1,
//   "name": "Alice",
//   ...
// }

// Rename fields with @json attribute
@json[rename_all: "camelCase"]
@derive[Debug, Clone]
struct ApiUser {
    user_id:   int,      // serializes as "userId"
    full_name: str,      // serializes as "fullName"
    is_active: bool,     // serializes as "isActive"
}
```

---

## std.regex

Regular expressions.

```razen
use std.regex { Regex, Match }

// Compile a pattern
pat := Regex.new(r"\d{3}-\d{4}").unwrap()

// Test
is_phone := pat.is_match("555-1234")    // true

// Find first match
match pat.find("Call 555-1234 now") {
    some(m) -> println("Found: {m.text()} at {m.start()}..{m.end()}"),
    none    -> println("No match"),
}

// Find all matches
matches := pat.find_all("555-1234 or 444-5678")
loop m in matches {
    println(m.text())    // "555-1234", "444-5678"
}

// Capture groups
email_pat := Regex.new(r"(\w+)@(\w+)\.(\w+)").unwrap()
if let some(m) = email_pat.captures("alice@razen.dev") {
    println(m.group(1))    // "alice"
    println(m.group(2))    // "razen"
    println(m.group(3))    // "dev"
}

// Replace
result := pat.replace("Call 555-1234 now", "XXX-XXXX")
// "Call XXX-XXXX now"

result_all := pat.replace_all("555-1234 or 444-5678", "XXX-XXXX")
// "XXX-XXXX or XXX-XXXX"

// Split
parts := Regex.new(r"\s+").unwrap().split("hello   world  foo")
// ["hello", "world", "foo"]
```

---

## std.rand

Random number generation.

```razen
use std.rand { random, random_range, random_bool, shuffle, choose, Rng }

// Random values
n:  int   := std.rand.random[int]()        // any int
u:  uint  := std.rand.random[uint]()       // any uint
f:  float := std.rand.random[float]()      // 0.0 to 1.0
f2: f32   := std.rand.random[f32]()        // 0.0 to 1.0 (32-bit)

// Bounded random
n  := std.rand.random_range(1, 100)        // 1 to 99 (exclusive end)
n  := std.rand.random_range(1..=100)       // 1 to 100 (inclusive)
b  := std.rand.random_bool()               // true or false
b  := std.rand.random_bool_with_prob(0.3)  // true 30% of the time

// Shuffle in place
mut items: vec[int] = vec[1, 2, 3, 4, 5]
std.rand.shuffle(items)

// Random element
item := std.rand.choose(items)             // option[T]

// Seeded RNG for reproducibility
mut rng := Rng.seeded(42)
n := rng.next_int_range(0, 100)
f := rng.next_float()
rng.shuffle(items)
```

---

## std.hash

Hashing algorithms.

```razen
use std.hash { sha256, sha512, md5, fnv1a, blake3, Hasher }

// One-shot hashing
digest := std.hash.sha256("hello world")          // str → hex str
digest := std.hash.sha256_bytes(data)             // bytes → bytes
digest := std.hash.sha512("input")
digest := std.hash.md5("legacy use")
fast   := std.hash.fnv1a("quick hash")
secure := std.hash.blake3("modern secure hash")

// HMAC
mac := std.hash.hmac_sha256(key: "secret", data: "message")

// Streaming hasher
mut h := Hasher.sha256()
h.update("part 1")
h.update("part 2")
digest := h.finalize()    // hex string
```

---

## std.base64

Base64 encoding and decoding.

```razen
use std.base64 { encode, decode, encode_url, decode_url }

encoded := std.base64.encode("Hello, Razen!")
// "SGVsbG8sIFJhemVuIQ=="

decoded := std.base64.decode(encoded).unwrap()
// "Hello, Razen!"

// URL-safe base64 (no +, /, = characters)
url_enc := std.base64.encode_url(bytes_data)
url_dec := std.base64.decode_url(url_enc).unwrap()

// Encode raw bytes
data: bytes := bytes[0x48, 0x65, 0x6C, 0x6C, 0x6F]
encoded     := std.base64.encode_bytes(data)
```

---

## std.collections

Extra collection algorithms beyond the built-in methods.

```razen
use std.collections { sort, sort_by, sort_by_key, stable_sort,
                      dedup, dedup_by, group_by, partition,
                      flatten, zip, unzip, transpose,
                      binary_search, merge, intersect, diff }

// Sorting
sorted    := std.collections.sort(vec[3, 1, 4, 1, 5, 9])
// [1, 1, 3, 4, 5, 9]

sorted_r  := std.collections.sort_by(items, |a, b| b.cmp(a))
// descending

by_name   := std.collections.sort_by_key(users, |u| u.name)
// sorted by name

// Dedup — remove consecutive duplicates (sort first for all duplicates)
deduped   := std.collections.dedup(vec[1, 1, 2, 2, 3])
// [1, 2, 3]

// Group by
grouped   := std.collections.group_by(users, |u| u.role)
// map[Role, vec[User]]

// Partition — split into (matches, non-matches)
(evens, odds) := std.collections.partition(nums, |n| n % 2 == 0)

// Flatten
nested    := vec[vec[1, 2], vec[3, 4], vec[5]]
flat      := std.collections.flatten(nested)    // [1, 2, 3, 4, 5]

// Binary search (requires sorted input)
idx := std.collections.binary_search(sorted_nums, 42)
// option[int] — index if found, none if not

// Set-like operations on vecs
merged    := std.collections.merge(a, b)          // union, sorted
common    := std.collections.intersect(a, b)      // intersection
only_a    := std.collections.diff(a, b)           // a minus b
```

---

## std.string

Extra string utilities beyond built-in `str` methods.

```razen
use std.string { pad_left, pad_right, pad_center, repeat,
                 word_wrap, truncate, indent, dedent,
                 similarity, levenshtein }

// Padding
right    := std.string.pad_left("42", 8)           // "      42"
left     := std.string.pad_right("42", 8)          // "42      "
center   := std.string.pad_center("42", 8)         // "   42   "
zeroed   := std.string.pad_left("42", 6, '0')      // "000042"

// Repeat
line     := std.string.repeat("─", 40)             // "────...─" (40 chars)

// Word wrap
wrapped  := std.string.word_wrap("long text here", max_width: 20)
// vec[str] — lines of at most 20 chars

// Truncate
short    := std.string.truncate("hello world", 8)         // "hello..."
short2   := std.string.truncate("hello world", 8, "…")   // "hello w…"

// Indent / dedent
indented := std.string.indent(code, "    ")    // 4-space indent each line
stripped := std.string.dedent(indented)         // remove common indent

// Similarity
sim := std.string.similarity("razen", "razor")         // 0.0 to 1.0
lev := std.string.levenshtein("razen", "razor")        // edit distance: int
```

---

## std.bytes

Byte buffer manipulation.

```razen
use std.bytes { BytesBuf, from_str, from_hex, to_hex }

// Create
buf := std.bytes.BytesBuf.new()
buf := std.bytes.BytesBuf.with_capacity(1024)

// Write
buf.write_u8(0xFF)
buf.write_u16_le(1024u16)    // little-endian
buf.write_u16_be(1024u16)    // big-endian
buf.write_u32_le(0xDEAD_BEEFu32)
buf.write_i32_le(-42i32)
buf.write_f32_le(3.14f32)
buf.write_bytes(data)
buf.write_str("hello")

// Read (with position)
mut cursor := std.bytes.Cursor.new(raw_bytes)
n:  u8   := cursor.read_u8()
n2: u16  := cursor.read_u16_le()
s:  str  := cursor.read_str(5)     // read 5 bytes as utf-8

// Convert
hex_str := std.bytes.to_hex(bytes[0xDE, 0xAD])    // "dead"
raw     := std.bytes.from_hex("deadbeef").unwrap()
utf8    := std.bytes.from_str("hello")
```

---

## std.mem

Memory layout and size utilities.

```razen
use std.mem { size_of, align_of, zeroed }

// Sizes at compile time
int_size   := std.mem.size_of[int]()     // 8 (bytes)
u8_size    := std.mem.size_of[u8]()      // 1
f32_size   := std.mem.size_of[f32]()     // 4
user_size  := std.mem.size_of[User]()    // sum of field sizes + padding

// Alignment
int_align  := std.mem.align_of[int]()    // 8
u8_align   := std.mem.align_of[u8]()     // 1

// Zero-initialized value
zeroed_val := std.mem.zeroed[User]()     // all fields set to zero/false/""
```

---

## std.ffi

Foreign function interface helpers for calling C code.

```razen
use std.ffi { CStr, CString, ptr_cast }

// Convert between Razen str and C strings
c_str := std.ffi.CString.from_str("hello").unwrap()
c_ptr := c_str.as_ptr()    // *const u8 — pass to C functions

// Convert C string pointer back to Razen str
razen_str := std.ffi.CStr.from_ptr(c_ptr).to_str().unwrap()

// Low-level FFI with unsafe
@extern["C"]
act c_strlen(s: bytes) uint

@extern["C"]
act c_memcpy(dst: bytes, src: bytes, n: uint) bytes

unsafe {
    len := c_strlen(c_ptr)
    println("C string length: {len}")
}
```

---

## Quick Import Reference

```razen
// Most common imports
use std.fs   { read, write, File }
use std.http { get, post }
use std.json { parse_json, to_json }
use std.math { PI, sqrt, sin, cos }
use std.time { now, sleep, Duration }
use std.rand { random, random_range }
use std.process { exit }
use std.env  { get }

// One-liner for multiple modules
use std.{ fs, math, time, json }
// Then: std.fs.read(...), std.math.PI, etc.
```
