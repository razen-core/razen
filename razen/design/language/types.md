# Razen Language Specification: Type System

This document covers the complete Razen type system — primitive types, numeric hierarchy,
the `char` and `never` types, operator traits, the Iterator protocol, and type casting.

---

## 1. Primitive Types Overview

| Type     | Description                          | Default? |
|----------|--------------------------------------|----------|
| `bool`   | Logical value — `true` / `false`     | —        |
| `int`    | Signed 64-bit integer                | ✓ default signed |
| `uint`   | Unsigned 64-bit integer              | ✓ default unsigned |
| `float`  | 64-bit floating-point (`f64` alias)  | ✓ default float |
| `str`    | UTF-8 string                         | —        |
| `char`   | Single Unicode scalar value          | —        |
| `bytes`  | Raw byte buffer                      | —        |
| `void`   | No return value                      | —        |
| `never`  | A type that never has a value        | —        |
| `tensor` | N-dimensional float array (AI/ML)    | —        |

---

## 2. Numeric Type Hierarchy

`int` and `uint` are the **default** integer types. Use them unless you have a specific
reason to care about width — systems code, FFI, GPU, network protocols, file formats.

### 2.1 Signed Integers

| Type  | Width   | Range                                      | Suffix  |
|-------|---------|--------------------------------------------|---------|
| `i8`  | 8-bit   | -128 to 127                                | `i8`    |
| `i16` | 16-bit  | -32,768 to 32,767                          | `i16`   |
| `i32` | 32-bit  | -2,147,483,648 to 2,147,483,647            | `i32`   |
| `i64` | 64-bit  | -9,223,372,036,854,775,808 to …,807        | `i64`   |
| `int` | 64-bit  | alias for `i64` — the default signed type  | —       |

### 2.2 Unsigned Integers

| Type   | Width   | Range                                      | Suffix  |
|--------|---------|--------------------------------------------|---------|
| `u8`   | 8-bit   | 0 to 255                                   | `u8`    |
| `u16`  | 16-bit  | 0 to 65,535                                | `u16`   |
| `u32`  | 32-bit  | 0 to 4,294,967,295                         | `u32`   |
| `u64`  | 64-bit  | 0 to 18,446,744,073,709,551,615            | `u64`   |
| `uint` | 64-bit  | alias for `u64` — the default unsigned type | `u`    |

### 2.3 Floating-Point

| Type    | Width  | Precision     | Notes                              |
|---------|--------|---------------|------------------------------------|
| `f32`   | 32-bit | ~7 digits     | Required for GPU, tensors, ML ops  |
| `f64`   | 64-bit | ~15 digits    | General-purpose float              |
| `float` | 64-bit | ~15 digits    | alias for `f64` — the default float |

```razen
// Default types — use these 95% of the time
count:  int   := 42
size:   uint  := 1024u
ratio:  float := 0.75

// Sized types — when the width matters
pixel:  u8   := 255u8
port:   u16  := 8080u16
flags:  u32  := 0xFFFF_FFFFu32
node_id: u64 := 9_999_999_999u64

offset: i8   := -5i8
delta:  i16  := -1000i16
index:  i32  := 2_147_483_647i32
big:    i64  := -9_223_372_036_854_775_808i64

weight: f32  := 0.5f32    // 32-bit — for GPU / tensor ops
pi:     f64  := 3.14159   // 64-bit — same as float

// Digit separators — _ is ignored in numeric literals
million  := 1_000_000
hex_mask := 0xFF_FF_FF_FFu32
fp_val   := 3.141_592_653
```

### 2.4 Numeric Literal Suffixes

| Suffix   | Type   | Example            |
|----------|--------|--------------------|
| (none)   | `int`  | `42`               |
| `u`      | `uint` | `1024u`            |
| `i8`     | `i8`   | `-5i8`             |
| `i16`    | `i16`  | `1000i16`          |
| `i32`    | `i32`  | `100_000i32`       |
| `i64`    | `i64`  | `9999i64`          |
| `u8`     | `u8`   | `255u8`            |
| `u16`    | `u16`  | `8080u16`          |
| `u32`    | `u32`  | `0xFFu32`          |
| `u64`    | `u64`  | `0xFFFF_FFFFu64`   |
| `f32`    | `f32`  | `0.5f32`           |
| (float)  | `f64`  | `3.14` (inferred)  |

---

## 3. The `char` Type

A `char` is a single **Unicode scalar value** — a valid Unicode codepoint, represented
as a `u32` internally. `char` literals use single quotes.

```razen
// char literals
letter:  char := 'a'
digit:   char := '9'
symbol:  char := '!'
space:   char := ' '
newline: char := '\n'
emoji:   char := '😀'
unicode: char := '\u{1F600}'   // same as above

// char operations
is_alpha   := letter.is_alphabetic()    // true
is_digit   := digit.is_numeric()        // true
is_upper   := letter.is_uppercase()     // false
upper_c    := letter.to_uppercase()     // 'A'
lower_c    := 'Z'.to_lowercase()        // 'z'
codepoint  := letter.to_u32()           // 97
as_str     := letter.to_string()        // "a"

// char from u32
c := char.from_u32(65u32)              // some('A')

// char ranges
loop c in 'a'..='z' {
    print(c)    // a b c ... z
}

loop c in '0'..='9' {
    print(c)    // 0 1 2 ... 9
}

// String iteration yields chars
text := "Hello"
loop c in text.chars() {
    println(c)    // 'H', 'e', 'l', 'l', 'o'
}

// char in match
act classify(c: char) str {
    match c {
        'a'..='z' -> "lowercase",
        'A'..='Z' -> "uppercase",
        '0'..='9' -> "digit",
        ' '       -> "space",
        _         -> "other",
    }
}
```

---

## 4. The `never` Type

`never` is the type of an expression that **never produces a value** — it represents
computations that always diverge: panics, process exits, infinite loops.

`never` is a subtype of every type, so it can appear anywhere a value is expected.
The compiler uses this to verify exhaustive control flow.

```razen
// Functions that never return
act panic(msg: str) never {
    println("PANIC: {msg}")
    std.process.exit(1)
}

act unreachable() never {
    panic("reached unreachable code")
}

act server_loop(port: int) never {
    sock := net.bind(port).unwrap()
    loop {
        conn := sock.accept().unwrap()
        handle(conn)
    }
}

// never in match arms — compiler knows these arms are exhaustive
act divide(a: int, b: int) result[int, str] {
    if b == 0 { ret err("division by zero") }
    ok(a / b)
}

// never coerces to any type — useful in match
x: int := if condition { 42 } else { panic("bad state") }
//                                   ^^^^^^^^^^^^^^^^^^^^
//                         panic() returns never — coerces to int here

// loop { } without break also has type never
```

---

## 5. Numeric Type Casting with `as`

`as` performs explicit type conversion. Widening is always safe; narrowing truncates.

```razen
// Widening — always safe, no data loss
small: i32  := 100i32
big:   i64  := small as i64     // i32 → i64

byte:  u8   := 200u8
wide:  u32  := byte as u32      // u8 → u32

// Narrowing — truncates high bits
big_val: i64 := 300i64
trunced: i8  := big_val as i8   // 300 % 256 = 44

// int ↔ float
count: int   := 42
ratio: float := count as float  // int → float
back:  int   := ratio as int    // float → int (truncates toward zero)

// Numeric to char
ascii: u8   := 65u8
c:     char := ascii as char    // u8 → char ('A')

// char to numeric
code: u32 := 'A' as u32         // char → u32 (65)

// Sized integer conversions
n: i32  := 1000i32
m: u16  := n as u16             // i32 → u16

// f32 ↔ f64
hp: f32  := 0.5f32
dp: f64  := hp as f64           // f32 → f64 (widens)
bp: f32  := dp as f32           // f64 → f32 (may lose precision)
```

**Rules:**
- `as` never panics — it always produces a value by truncation or reinterpretation.
- For safe narrowing with bounds checking, use `.try_as()` which returns `option[T]`.

```razen
safe := big_val.try_as[i8]()    // option[i8] — none if out of range
```

---

## 6. Operator Overloading Traits

These standard traits let custom types work with built-in operators.
Implement them with `impl Trait for Type`.

### 6.1 Arithmetic

```razen
trait Add[Rhs, Output] {
    act add(self, other: Rhs) Output
}

trait Sub[Rhs, Output] {
    act sub(self, other: Rhs) Output
}

trait Mul[Rhs, Output] {
    act mul(self, other: Rhs) Output
}

trait Div[Rhs, Output] {
    act div(self, other: Rhs) Output
}

trait Rem[Rhs, Output] {
    act rem(self, other: Rhs) Output
}

trait Neg {
    act neg(self) Self    // unary minus -x
}
```

### 6.2 Equality & Ordering

```razen
trait Eq {
    act eq(self, other: Self) bool

    // Default implementation — override if needed
    act ne(self, other: Self) bool {
        not self.eq(other)
    }
}

enum Ordering {
    Less,
    Equal,
    Greater,
}

trait Ord: Eq {
    act cmp(self, other: Self) Ordering

    // Defaults derived from cmp — rarely need to override
    act lt(self, other: Self) bool { self.cmp(other) == Ordering.Less }
    act gt(self, other: Self) bool { self.cmp(other) == Ordering.Greater }
    act le(self, other: Self) bool { not self.gt(other) }
    act ge(self, other: Self) bool { not self.lt(other) }
    act min(self, other: Self) Self {
        if self.le(other) { self } else { other }
    }
    act max(self, other: Self) Self {
        if self.ge(other) { self } else { other }
    }
}
```

### 6.3 Display & Debug

```razen
trait Display {
    act to_string(self) str
}

trait Debug {
    act debug_str(self) str
}
```

### 6.4 Cloning

```razen
trait Clone {
    act clone(self) Self
}
```

### 6.5 Indexing

```razen
trait Index[Idx, Output] {
    act index(self, idx: Idx) Output
}

trait IndexMut[Idx, Output] {
    act index_mut(mut self, idx: Idx) Output
}
```

### 6.6 Implementation Example

```razen
struct Vector2 {
    x: float,
    y: float,
}

impl Add[Vector2, Vector2] for Vector2 {
    act add(self, other: Vector2) Vector2 {
        Vector2 { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Sub[Vector2, Vector2] for Vector2 {
    act sub(self, other: Vector2) Vector2 {
        Vector2 { x: self.x - other.x, y: self.y - other.y }
    }
}

impl Mul[float, Vector2] for Vector2 {
    act mul(self, scalar: float) Vector2 {
        Vector2 { x: self.x * scalar, y: self.y * scalar }
    }
}

impl Neg for Vector2 {
    act neg(self) Vector2 {
        Vector2 { x: -self.x, y: -self.y }
    }
}

impl Eq for Vector2 {
    act eq(self, other: Vector2) bool {
        self.x == other.x && self.y == other.y
    }
}

impl Display for Vector2 {
    act to_string(self) str {
        "({self.x}, {self.y})"
    }
}

impl Clone for Vector2 {
    act clone(self) Vector2 {
        Vector2 { x: self.x, y: self.y }
    }
}

// Using operator overloads
a := Vector2 { x: 1.0, y: 2.0 }
b := Vector2 { x: 3.0, y: 4.0 }

c := a + b            // Vector2 { x: 4.0, y: 6.0 }
d := b - a            // Vector2 { x: 2.0, y: 2.0 }
e := a * 3.0          // Vector2 { x: 3.0, y: 6.0 }
f := -a               // Vector2 { x: -1.0, y: -2.0 }
g := a.clone()        // new Vector2 { x: 1.0, y: 2.0 }

if a == b { println("equal") }
println(a.to_string())    // "(1, 2)"
```

---

## 7. The `@derive` Attribute for Auto-Implementation

Common traits can be automatically derived by the compiler using the `@derive` attribute.

```razen
@derive[Debug, Clone, Eq]
struct Point {
    x: float,
    y: float,
}

// Compiler generates Debug, Clone, and Eq implementations automatically
p := Point { x: 1.0, y: 2.0 }
q := p.clone()
if p == q { println("equal") }
println(p.debug_str())    // "Point { x: 1.0, y: 2.0 }"
```

**Derivable traits:** `Debug`, `Clone`, `Eq`, `Ord`, `Display`, `Hash`

`Ord` requires `Eq` to also be derived or implemented.
`Hash` is used for map keys — automatically derived when needed.

---

## 8. The Iterator Protocol

Any type can become iterable by implementing `Iterator[T]`.
The `loop item in collection` syntax desugars to calling `.next()` in a loop.

```razen
trait Iterator[T] {
    act next(mut self) option[T]
}
```

**The loop desugar:**

```razen
// This:
loop item in collection {
    do_something(item)
}

// Becomes:
mut iter := collection.iter()
loop {
    match iter.next() {
        some(item) -> do_something(item),
        none       -> break,
    }
}
```

**Making a custom type iterable:**

```razen
struct Counter {
    current: int,
    max:     int,
}

impl Iterator[int] for Counter {
    act next(mut self) option[int] {
        if self.current >= self.max { ret none }
        val := self.current
        self.current = self.current + 1
        some(val)
    }
}

// Now Counter works with loop
mut counter := Counter { current: 0, max: 5 }
loop n in counter {
    println(n)    // 0, 1, 2, 3, 4
}

// Also works with all collection methods
result := counter.map(|n| n * n).filter(|n| n > 4).collect[vec[int]]()
```

**Iterable types (implement `.iter()` automatically):**

```razen
// All built-in collections
loop item in my_vec    { }
loop item in my_array  { }
loop (k, v) in my_map  { }
loop item in my_set    { }
loop c in my_str.chars() { }
loop b in my_bytes     { }
loop i in 0..10        { }

// Chaining iterator adapters
nums := vec[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

result := nums
    .filter(|x| x % 2 == 0)         // [2, 4, 6, 8, 10]
    .map(|x| x * x)                  // [4, 16, 36, 64, 100]
    .take(3)                          // [4, 16, 36]
    .collect[vec[int]]()

// Common adapters
.map(|x| f(x))                        // transform each item
.filter(|x| pred(x))                  // keep items matching pred
.reduce(init, |acc, x| f(acc, x))     // fold to single value
.fold(init, |acc, x| f(acc, x))       // same as reduce
.any(|x| pred(x))                     // true if any item matches
.all(|x| pred(x))                     // true if all items match
.find(|x| pred(x))                    // option[T] — first match
.position(|x| pred(x))               // option[int] — first index
.enumerate()                           // (int, T) pairs
.zip(other)                            // (T, U) pairs
.take(n)                               // first n items
.skip(n)                               // skip first n items
.chain(other)                          // concatenate two iterables
.flat_map(|x| iter_of(x))             // map then flatten
.collect[vec[T]]()                    // materialize into collection
.count()                               // number of items
.sum()                                 // sum (requires Add)
.product()                             // product (requires Mul)
.min()                                 // option[T] (requires Ord)
.max()                                 // option[T] (requires Ord)
.sort()                                // sorted vec (requires Ord)
.sort_by(|a, b| a.cmp(b))            // sorted with custom comparator
```

---

## 9. Type Aliases & Newtype Pattern

```razen
// Simple alias — transparent, no type safety
alias UserId    = int
alias Score     = float
alias Matrix4x4 = [[float; 4]; 4]

// Generic alias
alias ApiResult[T] = result[T, str]
alias MaybeUser    = option[User]

// Newtype pattern — wrapping creates a distinct type
struct UserId(int)       // newtype over int
struct Email(str)        // newtype over str
struct Meters(float)     // newtype over float

impl UserId {
    act new(id: int) result[UserId, str] {
        if id <= 0 { ret err("UserId must be positive") }
        ok(UserId(id))
    }
    act value(self) int { self.0 }
}

impl Email {
    act new(addr: str) result[Email, str] {
        if not addr.contains("@") { ret err("Invalid email") }
        ok(Email(addr))
    }
    act value(self) str { self.0 }
}

// Now int and UserId are different types — can't mix them
user_id := UserId.new(42).unwrap()
raw_id  := user_id.value()           // 42

// This would be a compile error:
// act process(id: UserId) void { }
// process(42)    ← ERROR: expected UserId, got int
```

---

## 10. `Self` Type

Inside `impl` and `trait` blocks, `Self` refers to the type being implemented.
Use it to write methods that return or accept the same type without naming it explicitly.

```razen
trait Builder {
    act set_name(mut self, name: str) Self
    act set_active(mut self, active: bool) Self
    act build(self) Self
}

struct UserBuilder {
    name:      str,
    email:     str,
    is_active: bool,
}

impl UserBuilder {
    act new() Self {
        UserBuilder { name: "", email: "", is_active: false }
    }

    act name(mut self, name: str) Self {
        self.name = name
        self
    }

    act email(mut self, email: str) Self {
        self.email = email
        self
    }

    act active(mut self) Self {
        self.is_active = true
        self
    }

    act build(self) result[User, str] {
        if self.name == "" { ret err("Name required") }
        ok(User {
            id:        0,
            name:      self.name,
            email:     self.email,
            is_active: self.is_active,
        })
    }
}

// Builder pattern
user := UserBuilder.new()
    .name("Alice")
    .email("alice@razen.dev")
    .active()
    .build()
    .unwrap()
```

---

## 11. String Escape Sequences

| Sequence       | Meaning                                    |
|----------------|--------------------------------------------|
| `\n`           | Newline (LF)                               |
| `\r`           | Carriage return (CR)                       |
| `\t`           | Horizontal tab                             |
| `\\`           | Literal backslash `\`                      |
| `\"`           | Literal double quote `"`                   |
| `\'`           | Literal single quote `'` (in char literal) |
| `\0`           | Null byte                                  |
| `\x41`         | Hex byte — `A` (must be valid ASCII)       |
| `\u{1F600}`    | Unicode codepoint (1–6 hex digits)         |

```razen
tab_sep   := "col1\tcol2\tcol3"
two_lines := "line one\nline two"
path      := "C:\\Users\\razen\\file.txt"
quoted    := "She said \"hello\""
null_byte := "\0"
smiley    := "\u{1F600}"    // "😀"
hex_A     := "\x41"         // "A"

// Raw strings use r"..." — no escape processing
raw_path  := r"C:\Users\razen\file.txt"
raw_regex := r"\d+\.\d+"
```

---

## 12. Platform-Sized Integers — `isize` and `usize`

`isize` and `usize` are **pointer-sized** integers. Their width matches the target
platform: 32-bit on 32-bit systems, 64-bit on 64-bit systems.

Use them for array indices, pointer arithmetic, memory sizes, and any interop
with C's `ssize_t` and `size_t`.

```razen
// usize — unsigned pointer-sized integer
arr_len:   usize := arr.len()        // len() always returns usize
byte_count: usize := std.mem.size_of[User]()

// Index with usize
idx: usize := 3u
val := arr[idx]

// isize — signed pointer-sized integer (pointer arithmetic, offsets)
offset: isize := -4i
ptr_diff := (ptr_b as isize) - (ptr_a as isize)

// usize in loop
loop i in 0u..arr.len() {
    println(arr[i])
}

// int ↔ usize conversion
n:   int   := 42
u:   usize := n as usize
back: int  := u as int

// Why not just use int/uint?
// int  (i64) and usize may differ on 32-bit platforms
// usize is guaranteed to hold any valid index on the target machine
// Required for FFI with size_t / ssize_t
```

| Type    | Width   | Signed? | Use when                               |
|---------|---------|---------|----------------------------------------|
| `usize` | pointer | No      | Array indices, memory sizes, C size_t  |
| `isize` | pointer | Yes     | Pointer offsets, differences, C ssize_t |

---

## 13. Tensor Type System

`tensor` is a first-class built-in type designed for AI/ML workloads.
Tensors carry their **element dtype** as a type parameter.

```razen
// Typed tensor declarations
weights:  tensor[f32]   := tensor[f32][0.1, 0.4, 0.2, 0.3]
labels:   tensor[int]   := tensor[int][0, 1, 0, 1, 1]
image:    tensor[u8]    := tensor[u8][255, 128, 0, 64]
logits:   tensor[f64]   := tensor[f64][1.2, -0.5, 3.1]

// Default inferred dtype — float (f64) for floating literals
input     := tensor[1.0, 2.0, 3.0]       // inferred: tensor[float]
counts    := tensor[1, 2, 3, 4]           // inferred: tensor[int]

// 2D tensors — dtype carried on outer tensor
matrix:   tensor[f32]   := tensor[f32][
    [0.1, 0.2, 0.3],
    [0.4, 0.5, 0.6],
]

// 3D tensor
cube:     tensor[f32]   := tensor[f32][
    [[1.0, 2.0], [3.0, 4.0]],
    [[5.0, 6.0], [7.0, 8.0]],
]

// Mutable
mut w: tensor[f32] = tensor[f32][0.1, 0.2, 0.3, 0.4]
w = w * 0.5f32

// dtype query
d := weights.dtype()    // "f32"
shape := matrix.shape() // (2, 3)
size  := matrix.size()  // 6

// Convert dtype
f64_w := weights.to_dtype[f64]()    // tensor[f32] → tensor[f64]
f32_w := logits.to_dtype[f32]()     // tensor[f64] → tensor[f32]

// GPU / CPU transfer
gpu_w := weights.to_gpu()           // tensor[f32] — on GPU memory
cpu_w := gpu_w.to_cpu()             // tensor[f32] — back on CPU

// Supported dtypes for tensor[T]
// T must be one of: f32, f64, i8, i16, i32, i64, u8, u16, u32, u64, int, float
// Most ML operations require f32 or f64
```

**Tensor operations by dtype:**

```razen
a: tensor[f32] := tensor[f32][1.0, 2.0, 3.0]
b: tensor[f32] := tensor[f32][4.0, 5.0, 6.0]

// Element-wise — dtypes must match
c := a + b         // tensor[f32][5.0, 7.0, 9.0]
d := a * b         // tensor[f32][4.0, 10.0, 18.0]
e := a / b         // tensor[f32][0.25, 0.4, 0.5]

// Reductions
dot_p   := a.dot(b)             // f32: 32.0
total   := a.sum()              // f32: 6.0
mean    := a.mean()             // f32: 2.0
maximum := a.max()              // f32: 3.0
minimum := a.min()              // f32: 1.0

// Scalar ops — scalar must match dtype
scaled  := a * 2.0f32          // tensor[f32]
shifted := a + 1.0f32          // tensor[f32]

// Activation functions (on tensor[f32] or tensor[f64])
relu    := a.relu()
sigmoid := a.sigmoid()
softmax := a.softmax()
tanh    := a.tanh()

// Gradient computation
a.requires_grad()               // enable gradient tracking
loss := compute_loss(a)
loss.backward()                 // compute gradients
grad := a.grad()                // tensor[f32] — the gradient
```

---

## 14. `const act` — Compile-Time Functions

`const act` declares a function that the compiler **evaluates at compile time**
when called in a constant context. The result is inlined as a literal.

```razen
// Simple compile-time function
const act max_of(a: int, b: int) int -> if a > b { a } else { b }
const act min_of(a: int, b: int) int -> if a < b { a } else { b }
const act array_len[T, N](_: [T; N]) int -> N

// Use in const declarations — evaluated at compile time
const BUFFER_SIZE: int  = max_of(1024, 2048)   // 2048
const MIN_PORT:    int  = min_of(8080, 9090)    // 8080

// Evaluate array size from an actual array
my_array := [1, 2, 3, 4, 5]
const LEN: int = array_len(my_array)             // 5

// More complex compile-time computation
const act fibonacci(n: int) int {
    if n <= 1 { ret n }
    fibonacci(n - 1) + fibonacci(n - 2)
}

const FIB_10: int = fibonacci(10)    // 55 — computed at compile time

// Compile-time string operations
const act make_header(title: str) str {
    "═══ {title} ═══"
}

const APP_HEADER: str = make_header("Razen v1.0")

// Compile-time size calculations
const act padding_for(size: int, alignment: int) int {
    rem := size % alignment
    if rem == 0 { 0 } else { alignment - rem }
}

const STRUCT_PADDING: int = padding_for(13, 8)    // 3

// Rules:
// - const act body may only use: arithmetic, comparisons, const bindings,
//   other const act calls, and literals
// - No I/O, no heap allocation, no loops that depend on runtime values
// - Recursion is allowed (with a compile-time depth limit)
// - Works with generic type parameters that have compile-time-known sizes
```

---

## 15. Tuple Structs (Newtype Pattern)

A **tuple struct** wraps one or more types as positional fields.
The constructor is the struct name itself. Fields are accessed with `.0`, `.1`, etc.

```razen
// Single-field newtype — adds type safety over the underlying type
struct UserId(int)
struct Email(str)
struct Meters(float)
struct Millis(int)
struct PixelPos(int, int)    // two-field tuple struct

// Construction — use struct name as a function
uid     := UserId(42)
email   := Email("alice@razen.dev")
dist    := Meters(100.0)
delay   := Millis(500)
pos     := PixelPos(1280, 720)

// Access positional fields
raw_id  := uid.0        // 42
addr    := email.0      // "alice@razen.dev"
x_pos   := pos.0        // 1280
y_pos   := pos.1        // 720

// Destructure in let
PixelPos(x, y) := pos
UserId(id)     := uid

// In match
match maybe_id {
    some(UserId(n)) if n > 0 -> println("valid id: {n}"),
    some(UserId(_))           -> println("invalid id"),
    none                      -> println("no id"),
}

// Methods on tuple structs
impl UserId {
    act new(id: int) result[UserId, str] {
        if id <= 0 { ret err("UserId must be positive") }
        ok(UserId(id))
    }

    act value(self) int { self.0 }

    act is_valid(self) bool { self.0 > 0 }
}

impl Email {
    act new(addr: str) result[Email, str] {
        if not addr.contains("@") { ret err("Invalid email: {addr}") }
        ok(Email(addr.to_lower_case()))
    }

    act value(self) str { self.0 }

    act domain(self) str {
        parts := self.0.split("@")
        parts[1]
    }
}

impl Display for UserId {
    act to_string(self) str { "UserId({self.0})" }
}

impl Display for Meters {
    act to_string(self) str { "{self.0}m" }
}

// Type safety in practice
act process_user(id: UserId) void { ... }

// process_user(42)           ← COMPILE ERROR: expected UserId, got int
// process_user(UserId(42))   ← OK

act add_distances(a: Meters, b: Meters) Meters {
    Meters(a.0 + b.0)
}

// add_distances(1.0, 2.0)               ← COMPILE ERROR
// add_distances(Meters(1.0), 2.0)       ← COMPILE ERROR
// add_distances(Meters(1.0), Meters(2.0)) ← OK: Meters(3.0)
```

---

## 16. Error Trait and Custom Errors

The `Error` trait is the standard interface for all error types in Razen.
It enables error chaining, conversion via `From`, and the `?` operator across error types.

```razen
// The Error trait (in prelude)
trait Error: Display {
    act message(self) str
    act source(self) option[shared Error] {
        none    // default: no chained cause
    }
}
```

### Custom Error Types

```razen
@derive[Debug, Clone, Eq]
enum AppError {
    Io       { msg: str },
    Parse    { line: int, col: int, msg: str },
    Network  { code: int, reason: str },
    Auth     { reason: str },
    NotFound { resource: str, id: int },
}

impl Display for AppError {
    act to_string(self) str {
        self.message()
    }
}

impl Error for AppError {
    act message(self) str {
        match self {
            AppError.Io { msg }                 -> "IO error: {msg}",
            AppError.Parse { line, col, msg }   -> "Parse error at {line}:{col}: {msg}",
            AppError.Network { code, reason }   -> "Network {code}: {reason}",
            AppError.Auth { reason }             -> "Auth failed: {reason}",
            AppError.NotFound { resource, id }  -> "{resource} with id {id} not found",
        }
    }
}
```

---

## 17. `From` and `Into` — Type Conversion Traits

`From` and `Into` are the standard traits for infallible type conversion.
The `?` operator uses `From` to automatically convert between compatible error types.

```razen
trait From[T] {
    act from(value: T) Self
}

trait Into[T] {
    // Default: if From[U] for T is implemented, Into[T] for U comes for free
    act into(self) T
}
```

### Using `From` for error conversion

```razen
// Define a wrapper error type
@derive[Debug]
enum ServiceError {
    Io(AppError),
    Logic { msg: str },
}

impl Display for ServiceError {
    act to_string(self) str {
        match self {
            ServiceError.Io(e)      -> "Service IO error: {e.message()}",
            ServiceError.Logic { msg } -> "Logic error: {msg}",
        }
    }
}

impl Error for ServiceError { ... }

// Implement From so ? can convert AppError → ServiceError
impl From[AppError] for ServiceError {
    act from(e: AppError) ServiceError {
        ServiceError.Io(e)
    }
}

// Now ? converts automatically
act run_service(path: str) result[str, ServiceError] {
    // fs.read returns result[str, AppError]
    // ? converts AppError → ServiceError via From[AppError]
    data := std.fs.read(path)?

    if data.is_empty() {
        ret err(ServiceError.Logic { msg: "empty file" })
    }
    ok(data)
}
```

### Numeric conversions with `From`

```razen
// Built-in From implementations
wide: i64 := i64.from(100i32)    // i32 → i64 (widening, always safe)
wide: f64 := f64.from(0.5f32)    // f32 → f64 (widening)

// Using .into() — requires type annotation to resolve the target
wide: i64 := (100i32).into()
```

### Custom `From` / `Into`

```razen
struct Celsius(float)
struct Fahrenheit(float)

impl From[Celsius] for Fahrenheit {
    act from(c: Celsius) Fahrenheit {
        Fahrenheit(c.0 * 9.0 / 5.0 + 32.0)
    }
}

impl From[Fahrenheit] for Celsius {
    act from(f: Fahrenheit) Celsius {
        Celsius((f.0 - 32.0) * 5.0 / 9.0)
    }
}

body_temp   := Celsius(37.0)
in_f        := Fahrenheit.from(body_temp)    // Fahrenheit(98.6)
back_to_c   := Celsius.from(in_f)            // Celsius(37.0)

// Using .into()
in_f2: Fahrenheit := body_temp.into()
```

---

## Summary

| Type     | Default? | Width   | Use when                               |
|----------|----------|---------|----------------------------------------|
| `int`    | ✓        | 64-bit  | General-purpose signed integer         |
| `uint`   | ✓        | 64-bit  | General-purpose unsigned integer       |
| `float`  | ✓        | 64-bit  | General-purpose floating-point         |
| `i8`     | —        | 8-bit   | Tiny signed values, C interop          |
| `i16`    | —        | 16-bit  | Small signed values                    |
| `i32`    | —        | 32-bit  | 32-bit signed, common in APIs          |
| `i64`    | —        | 64-bit  | Explicit alias for `int`               |
| `u8`     | —        | 8-bit   | Bytes, pixel values, ASCII             |
| `u16`    | —        | 16-bit  | Ports, small counters                  |
| `u32`    | —        | 32-bit  | File sizes, common in C APIs           |
| `u64`    | —        | 64-bit  | Explicit alias for `uint`              |
| `isize`  | —        | pointer | Array indices, C ssize_t               |
| `usize`  | —        | pointer | Memory sizes, C size_t                 |
| `f32`    | —        | 32-bit  | GPU buffers, tensor weights, ML ops    |
| `f64`    | —        | 64-bit  | Explicit alias for `float`             |
| `char`   | —        | 32-bit  | Single Unicode scalar value            |
| `never`  | —        | —       | Functions that never return            |

**See also:**
- `keywords.md` — language keywords
- `patterns.md` — pattern matching and destructuring
- `symbols.md` — operators and syntax
- `prelude.md` — auto-imported types and functions
- `std.md` — standard library reference