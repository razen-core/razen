# DASO.md

## Deterministic Automatic Scoped Ownership

### The Memory Management Model of the Razen Programming Language

---

## 1. Overview

**DASO (Deterministic Automatic Scoped Ownership)** is Razen's native memory management model.

It is designed to achieve:

* **Performance near C/C++**
* **Deterministic memory behavior — no GC pauses**
* **Memory safety by default**
* **Minimal syntax and low cognitive load**
* **Less verbosity than Rust**
* **No runtime garbage collector**
* **95–100% of the time developers do not think about memory**

DASO is a **hybrid compile-time + deterministic runtime model** that combines the **best proven parts** of:

* Manual memory (C/C++)
* Ownership systems (Rust)
* Automatic management (GC languages)
* Region-based allocation (systems research)

Without inheriting their weaknesses.

---

## 2. Design Philosophy

Razen follows three core principles:

1. **Determinism over heuristics**
   Memory must be freed at known points in time.

2. **Safety without ceremony**
   The compiler enforces rules; developers write simple code.

3. **Pay only for what you use**
   No global GC, no hidden runtime costs.

---

## 3. Core Concept: Implicit Scoped Regions

### 3.1 Function-Level Memory Regions

Every `act` in Razen implicitly creates a **memory region**.

* All allocations inside the scope go into that region
* The region is freed **entirely and instantly** when the scope ends
* Deallocation is **O(1)**

```razen
act compute() void {
    a: vec[int]      := vec[1, 2, 3]
    b: map[str, int] := map["x": 10, "y": 20]
    c := User { id: 1, name: "Razen" }
}
// entire region freed here — one pointer reset
```

### 3.2 Why Regions Matter

| Model              | Deallocation Cost        |
|--------------------|--------------------------|
| C                  | Manual, error-prone      |
| Rust               | Drop per value (O(n))    |
| GC                 | Deferred, unpredictable  |
| **Razen (DASO)**   | One pointer reset (O(1)) |

---

## 4. Escape Analysis (Zero-Syntax Automation)

If a value **escapes its defining scope**, the compiler automatically promotes it.

```razen
act make_user() User {
    u := User { id: 1, name: "Alice" }
    ret u    // u escapes — compiler promotes to heap automatically
}
```

**Compiler behavior:**

* Detects `u` escapes the region
* Moves it to heap-backed storage
* Assigns ownership

**No annotations. No keywords. No developer action required.**

---

## 5. Ownership Model (Lite, Implicit, Inferred)

### 5.1 Ownership Rules

* Single owner by default
* Moves are implicit on assignment
* Borrows are inferred at call sites
* No lifetime syntax in the language

```razen
u := User { id: 1, name: "Alice" }
v := u       // move — u is now invalid
```

```razen
act print_user(user: User) void {
    println(user.name)
}

u := User { id: 1, name: "Alice" }
print_user(u)    // compiler infers borrow — u still valid here
```

---

## 6. Shared Memory (Explicit and Controlled)

Shared ownership is **opt-in** using the `shared` keyword.

```razen
shared cache: map[str, str] = map[]
cache.insert("key", "value")
```

### Properties

* Uses Automatic Reference Counting (ARC)
* Non-atomic by default
* **Automatically atomic when crossing `async` / `fork` boundaries** — compiler handles this
* Compiler warns on excessive sharing patterns

```razen
shared node := Node { id: 1, data: "shared" }
a := node    // ref count: 2
b := node    // ref count: 3
// freed when a, b, node all go out of scope
```

**Shared in data structures:**

```razen
struct ListNode[T] {
    value: T,
    next:  option[shared ListNode[T]],
}

node_c := shared ListNode[int] { value: 3, next: none }
node_b := shared ListNode[int] { value: 2, next: some(node_c) }
node_a := shared ListNode[int] { value: 1, next: some(node_b) }
```

---

## 7. Deterministic Resource Cleanup — `defer`

`defer` schedules code to run at scope exit, in reverse declaration order.

```razen
act read_file(path: str) result[str, str] {
    f := File.open(path)?
    defer f.close()           // runs when scope exits — always
    data := f.read()?
    ok(data)
}

act with_lock(key: str) void {
    lock := mutex.acquire(key)
    defer lock.release()      // even on early ret
    // critical section
}
```

Memory itself does **not** require `defer` — DASO regions handle that automatically.

---

## 8. Concurrency and DASO — `async` and `fork`

### 8.1 `async` / `await`

Async functions run cooperatively on a shared executor.
Values inside an `async act` follow the same region rules, with one addition:
values that cross `.await` points must either be owned or `shared`.

```razen
async act fetch(url: str) result[str, str] {
    res := http.get(url).await?
    ok(res.body)
}

async act load_user(id: int) result[User, str] {
    data := fetch("https://api.razen.dev/users/{id}").await?
    user := parse_json[User](data)?
    ok(user)
}
```

### 8.2 `fork` — Structured Concurrency

`fork` spawns multiple tasks that run concurrently and **waits for all of them**
before the enclosing scope can proceed. This is structured concurrency — no task
outlives the `fork` block.

**DASO interactions with `fork`:**
* Values passed into a `fork` task must be owned (moved in) or `shared`
* `shared` values crossing a `fork` boundary automatically upgrade to atomic ARC
* When the `fork` block exits, all task regions are freed deterministically

```razen
// Fork two tasks — results available after block
(user_res, posts_res) := fork {
    load_user(1).await,
    load_posts(1).await,
}

// Named results
fork {
    user_data   <- load_user(1).await,
    post_data   <- load_posts(1).await,
    cfg_data    <- load_config().await,
}
// user_data, post_data, cfg_data are bound and in scope here

// Fork N tasks over a collection
results := fork loop id in user_ids {
    load_user(id).await
}
// results: vec[result[User, str]]

// Shared state across forked tasks — atomic ARC kicks in automatically
shared counter: map[str, int] = map[]
fork {
    increment_counter(counter).await,
    read_counter(counter).await,
}

// Passing owned values into tasks — moved, not shared
data := fetch_raw_data()
processed := fork {
    heavy_transform(data).await    // data is moved into this task
}
```

**Why `fork` is safe under DASO:**

| Concern            | How DASO handles it                              |
|--------------------|--------------------------------------------------|
| Data races         | Non-shared values can't cross fork; shared = atomic ARC |
| Memory leaks       | Fork block owns all task regions; freed at exit  |
| Dangling references| Escape analysis prevents references from living past fork |
| Unpredictable lifetime | All tasks complete before fork exits — deterministic |

---

## 9. Unsafe Manual Memory

```razen
unsafe {
    buf  := alloc(1024)
    ptr  := &buf
    data := *ptr
    free(buf)
}
```

* Zero runtime overhead inside `unsafe`
* No DASO region applies — fully manual
* Isolated from safe code — undefined behavior cannot leak out

---

## 10. Comparison with Other Languages

### 10.1 Razen vs C/C++

| Aspect        | C/C++              | Razen                    |
|---------------|--------------------|--------------------------|
| Memory safety | Manual             | Compiler-enforced        |
| Leaks         | Common             | Impossible (safe code)   |
| Performance   | Excellent          | Excellent                |
| Determinism   | Yes                | Yes                      |

### 10.2 Razen vs Rust

| Aspect         | Rust                    | Razen                     |
|----------------|-------------------------|---------------------------|
| Lifetimes      | Explicit                | None (compiler-internal)  |
| Borrow checker | User-visible            | Hidden                    |
| Verbosity      | High                    | Low                       |
| Safety         | Excellent               | Excellent                 |
| Region frees   | O(n) drop chain         | O(1) pointer reset        |
| Shared state   | `Rc<RefCell<T>>`        | `shared` keyword          |

### 10.3 Razen vs GC Languages

| Aspect          | GC Languages   | Razen        |
|-----------------|----------------|--------------|
| GC pauses       | Yes            | Never        |
| Memory overhead | High           | Low          |
| Predictability  | Medium         | Exact        |
| Runtime cost    | Continuous     | Near-zero    |

---

## 11. Performance Characteristics

* Region allocation: bump-pointer (stack-like speed)
* Region deallocation: O(1) — one pointer reset
* Ownership moves: zero cost
* Shared memory: localized ARC only where `shared` is used
* Fork: all task regions freed at fork exit — deterministic
* No background GC threads
* No global runtime

---

## 12. Developer Mental Model

> "If it doesn't escape, I don't think about memory."
> "If it escapes, Razen handles it automatically."
> "If I need shared ownership, I write `shared`."
> "If I fork, shared values become atomic automatically."
> "If I need raw control, I write `unsafe`."

**Decision tree:**

```
Do I need to share a value across multiple owners?
├── No  → just use it — DASO handles cleanup
└── Yes → add `shared` — ARC kicks in

Does the value outlive its defining function?
├── No  → region allocation — O(1) cleanup at scope exit
└── Yes → compiler promotes automatically — no syntax needed

Does the value cross a fork or async boundary?
├── No  → non-atomic ARC (cheap)
└── Yes → compiler upgrades to atomic ARC automatically

Do I need OS resources (files, locks, sockets)?
└── Yes → use `defer` for cleanup

Do I need raw pointer access or manual allocation?
└── Yes → wrap in `unsafe { }`
```

---

## 13. Full Example — DASO and fork Together

```razen
struct Config {
    host: str,
    port: int,
}

struct UserService {
    config: shared Config,
    cache:  shared map[int, User],
}

impl UserService {
    act new(cfg: Config) UserService {
        UserService {
            config: shared cfg,
            cache:  shared map[],
        }
    }

    async act load_users(self, ids: vec[int]) result[vec[User], str] {
        // Fetch all users concurrently — fork spawns one task per id
        results := fork loop id in ids {
            self.load_one(id).await
        }

        // Collect results — stop on first error
        mut users: vec[User] = vec[]
        loop r in results {
            match r {
                ok(user)  -> users.push(user),
                err(msg)  -> ret err("Failed loading users: {msg}"),
            }
        }
        ok(users)
    }

    async act load_one(self, id: int) result[User, str] {
        // Check cache first — shared, atomic across fork
        if let some(user) = self.cache.get(id) {
            ret ok(user)
        }

        url  := "{self.config.host}:{self.config.port}/users/{id}"
        data := http.get(url).await?
        user := parse_json[User](data)?

        self.cache.insert(id, user)    // atomic write — safe across fork
        ok(user)
    }
}

async act main() void {
    cfg     := Config { host: "https://api.razen.dev", port: 443 }
    service := UserService.new(cfg)

    match service.load_users(vec[1, 2, 3, 4, 5]).await {
        ok(users) -> {
            loop user in users {
                println("{user.id}: {user.name}")
            }
        },
        err(msg) -> println("Error: {msg}"),
    }
}
// All regions freed deterministically when main exits
```

---

## 14. Summary

**DASO makes Razen:**

* **Faster than GC languages** — no pauses, O(1) region deallocation
* **Safer than C/C++** — compiler-enforced, no use-after-free, no leaks
* **Less verbose than Rust** — no lifetimes, no borrow syntax
* **Concurrency-safe** — `fork` + automatic atomic ARC
* **Predictable** — memory freed at exact known points
* **Scalable** — from scripts to operating system kernels

---

**Razen Memory Model:**
**Deterministic. Automatic. Safe. Fast.**