# The Razen Module System: `use` and `pub`

Razen's module system is designed to be predictable and scale from small scripts
to massive projects without complex configuration files.

**Core Philosophy:**
1. **Every file is a module.**
2. **Folders are namespaces.**
3. **Everything is private by default.** You must explicitly export what you want to share.

---

## 1. Visibility with `pub`

Razen has four visibility levels, from most to least accessible.

| Syntax        | Visible to                                     |
|---------------|------------------------------------------------|
| `pub`         | Any module anywhere — fully public             |
| `pub(pkg)`    | Any file within the same package (folder tree) |
| `pub(mod)`    | Sibling files within the same folder only      |
| *(no keyword)*| Current file only — fully private              |

```razen
// ─── models/user.rzn ───────────────────────────────────────

// Public — importable by any module
pub struct User {
    id:        int,
    name:      str,
    email:     str,
    is_active: bool,
}

pub enum Role {
    Admin,
    Member,
    Moderator,
}

pub trait Describable {
    act describe(self) str
    act print_info(self) void {
        println(self.describe())
    }
}

// Package-internal — visible within the project, not to external consumers
pub(pkg) struct UserCache {
    entries: map[int, User],
}

pub(pkg) act cache_user(cache: mut UserCache, user: User) void {
    cache.entries.insert(user.id, user)
}

// Module-internal — visible to sibling files in models/ folder only
pub(mod) act validate_email(email: str) bool {
    email.contains("@") && email.contains(".")
}

// Private — only this file can use it
act hash_password(password: str) str {
    // internal implementation
}
```

---

## 2. Importing with `use`

The `use` keyword brings code from other modules into scope.
File extensions are never typed — Razen handles `.rzn` automatically.

### 2.1 Basic Import — sibling files

```
/project
  ├── main.rzn
  ├── math.rzn
  └── models.rzn
```

**`main.rzn`**
```razen
use math
use models

act main() void {
    sum := math.add(10, 20)
    v   := math.Vector { x: 1.0, y: 2.0 }

    alice := models.User {
        id:        1,
        name:      "Alice",
        email:     "alice@razen.dev",
        is_active: true,
    }
    println(alice.describe())
}
```

### 2.2 Folder Import — dot syntax

Folders are namespaces. Replace `/` with `.`.

```
/project
  ├── main.rzn
  └── utils
      ├── network.rzn
      └── crypto.rzn
```

**`main.rzn`**
```razen
use utils.network
use utils.crypto

act main() void {
    conn  := utils.network.connect("https://api.razen.dev")
    token := utils.crypto.sign("payload", "secret")
}
```

### 2.3 Specific Items — `{ }`

Import named items directly into scope.

```razen
use math { add, Vector }
use models { User, Role, Describable }

act main() void {
    sum  := add(5, 5)
    v    := Vector { x: 0.0, y: 1.0 }

    alice := User {
        id:        1,
        name:      "Alice",
        email:     "alice@razen.dev",
        is_active: true,
    }
}
```

Trailing commas are fine:

```razen
use models { User, Role, Describable, }
use utils.network { connect, disconnect, Response, }
```

### 2.4 Renaming with `as`

```razen
use utils.network as net
use my_long_database_module as db

use models { User as AppUser, Role as UserRole }

act main() void {
    conn := net.connect("https://api.razen.dev")
    rows := db.query("SELECT * FROM users")

    alice: AppUser := AppUser { id: 1, name: "Alice", email: "", is_active: true }
}
```

---

## 3. Advanced Organization

### 3.1 Folder as Module — `mod.rzn`

Place `mod.rzn` inside a folder to define that folder's public API.
It re-exports items from sibling files so importers see one clean surface.

```
/project
  └── database
      ├── mod.rzn        ← folder's public API
      ├── connection.rzn
      ├── query.rzn
      └── migration.rzn
```

**`database/connection.rzn`**
```razen
pub struct Connection {
    host:   str,
    port:   int,
    active: bool,
}

pub act connect(host: str, port: int) result[Connection, str] {
    // ...
}

pub(pkg) act ping(conn: Connection) bool {
    // internal health check — package-visible
}
```

**`database/query.rzn`**
```razen
pub act run_query(conn: Connection, sql: str) result[vec[str], str] {
    // ...
}

pub(mod) act build_query(parts: vec[str]) str {
    parts.join(" ")    // only visible to database/ siblings
}
```

**`database/mod.rzn`**
```razen
pub use connection { connect, Connection }
pub use query { run_query }
// ping and build_query are not re-exported
```

**`main.rzn`**
```razen
use database

act main() void {
    conn := database.connect("localhost", 5432).unwrap()
    rows := database.run_query(conn, "SELECT * FROM users").unwrap()
    loop row in rows { println(row) }
}
```

### 3.2 Root Imports — `~`

Use `~` to import from the project root. Avoids `../../..` in deep nesting.

```
/project
  ├── config.rzn
  ├── models.rzn
  └── app
      └── http
          └── controllers
              └── auth.rzn
```

**`app/http/controllers/auth.rzn`**
```razen
use ~.config
use ~.models { User, Role }

act handle_login(email: str) result[User, str] {
    api_key := config.get_api_key()
    ok(User { id: 1, name: email, email: email, is_active: true })
}
```

---

## 4. Standard Library Imports

```razen
use std.math
use std.io
use std.fs
use std.net
use std.time
use std.process
use std.collections

act main() void {
    area  := std.math.PI * 5.0 * 5.0
    root  := std.math.sqrt(144.0)

    std.io.println("Hello, Razen!")
    line  := std.io.read_line()

    data  := std.fs.read("config.json").unwrap()
    std.fs.write("output.txt", data)

    sorted := std.collections.sort(vec[3, 1, 4, 1, 5, 9])

    now   := std.time.now()
    ms    := std.time.millis()
}
```

Specific imports:

```razen
use std.math { PI, sqrt, pow, floor, ceil, abs }
use std.fs   { read, write, exists, remove }
```

---

## 5. Re-export — `pub use`

Re-export items from other modules to create a curated public API.

```razen
// In mod.rzn or any module file
pub use connection { connect, Connection }        // fully public
pub(pkg) use internal { helper_fn }               // package-visible re-export
```

---

## 6. Complete Project Example

```
/blog_api
  ├── main.rzn
  ├── config.rzn
  ├── models
  │   ├── mod.rzn
  │   ├── user.rzn
  │   └── post.rzn
  └── handlers
      ├── mod.rzn
      ├── auth.rzn
      └── posts.rzn
```

**`config.rzn`**
```razen
pub struct AppConfig {
    host:    str,
    port:    int,
    db_url:  str,
}

pub act load() AppConfig {
    AppConfig { host: "0.0.0.0", port: 8080, db_url: "postgres://localhost/blog" }
}
```

**`models/user.rzn`**
```razen
@derive[Debug, Clone, Eq]
pub struct User {
    id:        int,
    name:      str,
    email:     str,
    is_active: bool,
}

pub enum Role { Admin, Author, Reader }
```

**`models/post.rzn`**
```razen
@derive[Debug, Clone]
pub struct Post {
    id:        int,
    title:     str,
    body:      str,
    author_id: int,
    published: bool,
}
```

**`models/mod.rzn`**
```razen
pub use user { User, Role }
pub use post { Post }
```

**`handlers/auth.rzn`**
```razen
use ~.models { User }
use ~.config { AppConfig }

pub act login(cfg: AppConfig, email: str) result[User, str] {
    guard email != "" else { ret err("Email required") }
    ok(User { id: 1, name: "Alice", email: email, is_active: true })
}
```

**`handlers/posts.rzn`**
```razen
use ~.models { Post, User }

pub act list_posts(user: User) vec[Post] {
    vec[
        Post { id: 1, title: "Hello Razen", body: "...", author_id: user.id, published: true },
        Post { id: 2, title: "DASO Model",  body: "...", author_id: user.id, published: false },
    ]
}
```

**`handlers/mod.rzn`**
```razen
pub use auth { login }
pub use posts { list_posts }
```

**`main.rzn`**
```razen
use config
use models { User, Post }
use handlers

act main() void {
    cfg := config.load()

    match handlers.login(cfg, "alice@razen.dev") {
        ok(user) -> {
            println("Logged in: {user.name}")
            posts := handlers.list_posts(user)
            loop post in posts {
                status := if post.published { "✓" } else { "draft" }
                println("[{status}] {post.title}")
            }
        },
        err(msg) -> println("Login failed: {msg}"),
    }
}
```

---

## Import Style Summary

| Syntax                        | Description                              | When to use                          |
|-------------------------------|------------------------------------------|--------------------------------------|
| `use math`                    | Import entire module                     | Use many items from it               |
| `use math { add, Vector }`    | Import specific items                    | Use a few items frequently           |
| `use math as m`               | Import with alias                        | Name conflict or brevity             |
| `use utils.network`           | Import from subfolder                    | Module is in a subdirectory          |
| `use ~.config`                | Import from project root                 | Avoid `../../` in deep nesting       |
| `pub use other { Item }`      | Re-export from `mod.rzn`                 | Build a folder's public API          |

---

**See also:**
- `keywords.md` — `pub`, `use`, visibility rules
- `symbols.md` — operators and syntax