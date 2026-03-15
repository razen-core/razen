# Razen Prelude

The **prelude** is a small set of types, functions, and traits that are automatically
imported into every Razen file. You never write `use` for these — they are always in scope.

The prelude is intentionally minimal. Everything else comes from `std.*` modules.

---

## 1. Output

### `println`

Print a value followed by a newline. Accepts any type that implements `Display`.

```razen
println("Hello, Razen!")           // string
println(42)                        // int
println(3.14)                      // float
println(true)                      // bool
println(user.name)                 // any Display type
println("Score: {score}")          // interpolated string
println("")                        // empty line
```

### `print`

Print a value without a trailing newline.

```razen
print("Loading")
print(".")
print(".")
println(" done")    // "Loading... done"
```

### `eprintln`

Print to standard error with a newline.

```razen
eprintln("Error: {msg}")
eprintln("Warning: score is {score}")
```

### `eprint`

Print to standard error without a newline.

```razen
eprint("ERR: ")
eprintln(msg)
```

---

## 2. Assertions

### `assert`

Halt the program if a condition is false. Only active in debug and test builds.
Stripped in release builds unless `@keep_assert` is used.

```razen
assert 1 + 1 == 2
assert user.is_active
assert scores.len() > 0

// With a message
assert result.is_ok(), "expected ok result"
assert idx < arr.len(), "index {idx} out of bounds (len {arr.len()})"
```

### `assert_eq`

Assert two values are equal. On failure, prints both values.

```razen
assert_eq add(2, 3), 5
assert_eq user.name, "Alice"
assert_eq scores.len(), 4
```

### `assert_ne`

Assert two values are not equal.

```razen
assert_ne result, none
assert_ne error_code, 0
```

### `unreachable`

Mark a code path as impossible. Panics if reached.

```razen
match status {
    Status.Active   -> process(),
    Status.Inactive -> skip(),
    Status.Pending  -> queue(),
    // compiler enforces exhaustiveness, but if someone adds a variant
    // and forgets to update this match:
    _ -> unreachable("unhandled Status variant"),
}
```

---

## 3. Panic

### `panic`

Immediately halt the program with a message. Returns `never`.

```razen
panic("something went very wrong")
panic("expected user id > 0, got {id}")
```

`panic` is distinct from `assert` — it is never stripped. Use `assert` for
developer invariants during development, `panic` for conditions that truly
should never happen at runtime.

---

## 4. Debug Printing

### `dbg`

Print the value with its debug representation and return it unchanged.
Useful for inspecting values mid-expression without disrupting control flow.

```razen
// Prints: [dbg] score = 42  (file.rzn:14)
result := dbg(score) * 2

// Works in chains
filtered := values
    .filter(|x| dbg(x) > 5)
    .collect[vec[int]]()

// Multiple values
dbg(user.name, user.score, user.is_active)
```

`dbg` requires `T: Debug`. Stripped in release builds.

---

## 5. Option Constructors

```razen
// Constructors — always in scope
some(42)         // option[int] — has value
none             // option[T]  — no value (type inferred from context)

// Usage
act find(id: int) option[User] {
    if id == 1 { ret some(alice) }
    none
}

// Pattern match
match find(1) {
    some(user) -> println(user.name),
    none       -> println("not found"),
}

// Methods (on option[T])
opt.is_some()                   // bool
opt.is_none()                   // bool
opt.unwrap()                    // T — panics if none
opt.unwrap_or(default)          // T — returns default if none
opt.unwrap_or_else(|| compute())// T — calls closure if none
opt.expect("msg")               // T — panics with msg if none
opt.map(|x| f(x))               // option[U]
opt.flat_map(|x| opt_f(x))      // option[U]
opt.filter(|x| pred(x))         // option[T]
opt.or(other_opt)               // option[T]
opt.or_else(|| other)           // option[T]
opt.and(other_opt)              // option[U]
opt.and_then(|x| opt_f(x))      // option[U] — same as flat_map
opt.zip(other_opt)              // option[(T, U)]
opt.unzip()                     // (option[T], option[U]) — from option[(T,U)]
opt.ok_or(err_val)              // result[T, E]
opt.ok_or_else(|| err_val)      // result[T, E]
opt.inspect(|x| f(x))          // option[T] — side effect, returns self
opt.take(mut self)              // option[T] — returns value, sets self to none
opt.replace(mut self, val)      // option[T] — swaps value in place
```

---

## 6. Result Constructors

```razen
// Constructors — always in scope
ok(42)           // result[int, E]  — success
err("failed")    // result[T, str]  — failure

// Usage
act parse(s: str) result[int, str] {
    if s.is_empty() { ret err("empty input") }
    ok(s.parse_int()?)
}

// Pattern match
match parse("42") {
    ok(n)    -> println("got {n}"),
    err(msg) -> println("error: {msg}"),
}

// Methods (on result[T, E])
res.is_ok()                     // bool
res.is_err()                    // bool
res.unwrap()                    // T — panics if err
res.unwrap_err()                // E — panics if ok
res.unwrap_or(default)          // T
res.unwrap_or_else(|e| f(e))    // T
res.expect("msg")               // T — panics with msg if err
res.expect_err("msg")           // E — panics with msg if ok
res.ok()                        // option[T] — discards error
res.err()                       // option[E] — discards value
res.map(|x| f(x))               // result[U, E]
res.map_err(|e| f(e))           // result[T, F]
res.flat_map(|x| res_f(x))      // result[U, E]
res.and(other_res)              // result[U, E]
res.and_then(|x| res_f(x))      // result[U, E] — same as flat_map
res.or(other_res)               // result[T, F]
res.or_else(|e| res_f(e))       // result[T, F]
res.inspect(|x| f(x))          // result[T,E] — side effect on ok, returns self
res.inspect_err(|e| f(e))      // result[T,E] — side effect on err, returns self
```

---

## 7. String Builder

### `string_builder`

Creates a mutable buffer for building strings piece by piece.
More efficient than repeated `+` concatenation.

```razen
mut b := string_builder()

b.append("Hello")
b.append(", ")
b.append("Razen!")
b.append_line(" — v1.0")      // appends value + newline
b.append_char('!')
b.append_repeat("=", 40)      // append "=" forty times

result := b.to_string()        // "Hello, Razen! — v1.0\n!"

// Pre-allocate capacity
mut b2 := string_builder_with_capacity(256)

// Builder with separator
mut b3 := string_builder()
items := vec["a", "b", "c", "d"]
loop (i, item) in items.enumerate() {
    if i > 0 { b3.append(", ") }
    b3.append(item)
}
result2 := b3.to_string()    // "a, b, c, d"
```

---

## 8. Formatting — `format`

Build a formatted string without printing it. Same interpolation as string literals.

```razen
msg := format("Hello, {name}! You are {age} years old.")
path := format("{base_dir}/{filename}.{ext}")
padded := format("{value:>10}")      // right-align in 10 chars
hex    := format("{n:#x}")           // hex: 0xff
debug  := format("{user:?}")         // debug repr
```

### Format specifiers

| Specifier    | Meaning                            | Example           |
|--------------|------------------------------------|-------------------|
| `{val}`      | Default display                    | `"42"`            |
| `{val:?}`    | Debug representation               | `"Point { x: 1 }"`|
| `{val:b}`    | Binary                             | `"101010"`        |
| `{val:#b}`   | Binary with prefix                 | `"0b101010"`      |
| `{val:o}`    | Octal                              | `"52"`            |
| `{val:#o}`   | Octal with prefix                  | `"0o52"`          |
| `{val:x}`    | Hex lowercase                      | `"2a"`            |
| `{val:#x}`   | Hex lowercase with prefix          | `"0x2a"`          |
| `{val:X}`    | Hex uppercase                      | `"2A"`            |
| `{val:#X}`   | Hex uppercase with prefix          | `"0X2A"`          |
| `{val:e}`    | Scientific notation                | `"4.2e1"`         |
| `{val:>N}`   | Right-align in N chars             | `"      42"`      |
| `{val:<N}`   | Left-align in N chars              | `"42      "`      |
| `{val:^N}`   | Center in N chars                  | `"    42    "`    |
| `{val:0>N}`  | Right-align, zero-fill             | `"000042"`        |
| `{val:.N}`   | N decimal places                   | `"3.14"`          |
| `{val:.Nf}`  | N decimal places, always show dot  | `"3.14"`          |

---

## 9. Input — `read_line`

Read a line from standard input. Returns `result[str, str]`.

```razen
// Basic
line := read_line().unwrap_or("")
println("You typed: {line}")

// With prompt
print("Enter your name: ")
name := read_line().unwrap().trim()
println("Hello, {name}!")

// In a loop
loop {
    print("> ")
    line := read_line().unwrap_or("")
    if line == "quit" { break }
    process_command(line)
}
```

---

## 10. Type Conversion

### `to_string`

Convert any `Display` value to a `str`.

```razen
s := 42.to_string()         // "42"
s := 3.14.to_string()       // "3.14"
s := true.to_string()       // "true"
s := 'a'.to_string()        // "a"
s := user.to_string()       // calls Display impl
```

### Parse methods (on `str`)

```razen
"42".parse_int()            // result[int, str]
"3.14".parse_float()        // result[float, str]
"true".parse_bool()         // result[bool, str]
"a".parse_char()            // result[char, str]
"42".parse[int]()           // result[int, str]   — generic form
"3.14".parse[f32]()         // result[f32, str]
```

---

## 11. Cloning

### `clone`

Deep-copy a value. Requires `T: Clone`.

```razen
original := vec[1, 2, 3]
copy     := original.clone()

original_user := alice
backup_user   := alice.clone()    // requires @derive[Clone] on User

// Built-in types are always Clone
n := 42
m := n.clone()    // same as n for primitive types
```

---

## 12. Prelude Type Summary

The following types, traits, and their methods are always in scope
without any `use` statement:

| Item                | Kind          | Notes                                    |
|---------------------|---------------|------------------------------------------|
| `bool`              | Type          | `true`, `false`                          |
| `int`               | Type          | Default signed 64-bit integer            |
| `uint`              | Type          | Default unsigned 64-bit integer          |
| `float`             | Type          | Default 64-bit float                     |
| `i8..i64`           | Types         | Sized signed integers                    |
| `u8..u64`           | Types         | Sized unsigned integers                  |
| `isize`, `usize`    | Types         | Pointer-sized integers                   |
| `f32`, `f64`        | Types         | Sized floats                             |
| `char`              | Type          | Unicode scalar value                     |
| `str`               | Type          | UTF-8 string                             |
| `bytes`             | Type          | Raw byte buffer                          |
| `void`              | Type          | No return value                          |
| `never`             | Type          | Diverging computation                    |
| `tensor`            | Type          | N-dimensional array (AI/ML)              |
| `option[T]`         | Type          | `some(T)` or `none`                      |
| `result[T, E]`      | Type          | `ok(T)` or `err(E)`                      |
| `vec[T]`            | Type          | Dynamic array                            |
| `map[K, V]`         | Type          | Hash map                                 |
| `set[T]`            | Type          | Hash set                                 |
| `some(val)`         | Constructor   | Create `option[T]`                       |
| `none`              | Value         | Empty `option[T]`                        |
| `ok(val)`           | Constructor   | Create `result[T, E]`                    |
| `err(val)`          | Constructor   | Create `result[T, E]`                    |
| `println`           | Function      | Print with newline                       |
| `print`             | Function      | Print without newline                    |
| `eprintln`          | Function      | Print to stderr with newline             |
| `eprint`            | Function      | Print to stderr without newline          |
| `assert`            | Macro         | Debug assertion                          |
| `assert_eq`         | Macro         | Equality assertion                       |
| `assert_ne`         | Macro         | Inequality assertion                     |
| `unreachable`       | Function      | Mark unreachable code path               |
| `panic`             | Function      | Halt with message → `never`              |
| `dbg`               | Macro         | Debug-print and return value             |
| `format`            | Function      | Build formatted string                   |
| `read_line`         | Function      | Read line from stdin                     |
| `string_builder`    | Function      | Create a `StringBuilder`                 |
| `Display`           | Trait         | `.to_string()` — human-readable          |
| `Debug`             | Trait         | `.debug_str()` — developer repr          |
| `Clone`             | Trait         | `.clone()` — deep copy                   |
| `Eq`                | Trait         | `==`, `!=`                               |
| `Ord`               | Trait         | `<`, `>`, `<=`, `>=`, `.cmp()`           |
| `Hash`              | Trait         | `.hash()` — for map/set keys             |
| `Add`               | Trait         | `+` operator                             |
| `Sub`               | Trait         | `-` operator                             |
| `Mul`               | Trait         | `*` operator                             |
| `Div`               | Trait         | `/` operator                             |
| `Rem`               | Trait         | `%` operator                             |
| `Neg`               | Trait         | Unary `-`                                |
| `Iterator[T]`       | Trait         | `.next()` — makes types loopable         |
| `From[T]`           | Trait         | `.from()` — type conversion              |
| `Into[T]`           | Trait         | `.into()` — type conversion              |
| `Error`             | Trait         | Standard error type                      |
| `Ordering`          | Enum          | `Less`, `Equal`, `Greater`               |

---

## 13. What Requires `use`

Anything not in this list requires an explicit `use` statement:

```razen
use std.fs       { read, write, exists, File }
use std.net      { TcpListener, TcpStream, http }
use std.time     { now, sleep, Duration }
use std.math     { PI, E, sqrt, sin, cos, pow }
use std.process  { exit, env, args }
use std.io       { BufReader, BufWriter, stdin, stdout }
use std.json     { parse_json, to_json }
use std.regex    { Regex }
use std.path     { Path, join_path, abs_path }
use std.thread   { spawn, JoinHandle }
use std.sync     { Mutex, RwLock, Channel }
use std.rand     { random, random_range, shuffle }
use std.hash     { sha256, md5 }
use std.base64   { encode, decode }
use std.collections { sort, dedup, group_by, partition }
```

See `std.md` for the complete standard library reference.
