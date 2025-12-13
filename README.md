# libvirt-rs

Pure Rust implementation of the libvirt client library. This project provides a native Rust client for communicating with libvirt daemons using the libvirt RPC protocol, without requiring the C libvirt library.

## Features

- **Pure Rust**: No C dependencies, fully native Rust implementation
- **Auto-generated API**: All 453+ libvirt RPC methods are automatically generated from `.x` protocol definition files
- **Async/Await**: Built on Tokio for async I/O
- **Type-safe**: Strong typing with serde-based XDR serialization

## Architecture

```plaintext
┌─────────────────────────────────────────────────────────────────┐
│  .x Protocol Files (proto/remote_protocol.x)                    │
│       │                                                         │
│       ▼ (build.rs)                                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  libvirt-codegen                                        │   │
│  │  • parser.rs: Parse .x files using nom                  │   │
│  │  • generator.rs: Generate Rust code using quote         │   │
│  └─────────────────────────────────────────────────────────┘   │
│       │                                                         │
│       ▼ (OUT_DIR/generated.rs)                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Generated Code                                         │   │
│  │  • Structs, Enums, Unions, Typedefs                    │   │
│  │  • Constants (REMOTE_PROGRAM, REMOTE_PROTOCOL_VERSION) │   │
│  │  • LibvirtRpc trait + GeneratedClient<T>               │   │
│  │  • 453+ async RPC methods                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  libvirt crate                                          │   │
│  │  • Connection: Unix socket transport + LibvirtRpc impl │   │
│  │  • Client: High-level API wrapper                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```plaintext
libvirt-rs/
├── Cargo.toml                 # Workspace configuration
├── proto/                     # libvirt protocol definition files
│   ├── remote_protocol.x      # Main remote protocol (source of truth)
│   └── virnetprotocol.x       # Network protocol definitions
│
├── crates/
│   ├── libvirt-xdr/           # XDR serialization (serde-based)
│   │   └── src/
│   │       ├── ser.rs         # XDR Serializer
│   │       ├── de.rs          # XDR Deserializer
│   │       └── opaque.rs      # Fixed-length opaque (UUID) handling
│   │
│   ├── libvirt-codegen/       # Code generator
│   │   └── src/
│   │       ├── parser.rs      # .x file parser (nom)
│   │       ├── ast.rs         # AST definitions
│   │       └── generator.rs   # Rust code generator (quote)
│   │
│   └── libvirt/               # Main library
│       ├── build.rs           # Invokes codegen at build time
│       └── src/
│           ├── lib.rs         # Public API (Client, types)
│           ├── connection.rs  # RPC connection management
│           ├── packet.rs      # RPC packet encoding/decoding
│           └── transport/     # Transport implementations
│
└── examples/
    └── test_parse.rs          # Parser test example
```

## How Code Generation Works

The libvirt RPC protocol is defined in XDR (External Data Representation) format in `.x` files. This project automatically generates Rust code from these definitions at build time.

### 1. Protocol Definition (.x files)

The `proto/remote_protocol.x` file contains:

```c
/* Type definitions */
struct remote_nonnull_domain {
    remote_nonnull_string name;
    remote_uuid uuid;
    int id;
};

/* Procedure arguments and returns */
struct remote_connect_list_all_domains_args {
    int need_results;
    unsigned int flags;
};

struct remote_connect_list_all_domains_ret {
    remote_nonnull_domain domains<REMOTE_DOMAIN_LIST_MAX>;
    unsigned int ret;
};

/* Procedure definitions */
enum remote_procedure {
    REMOTE_PROC_CONNECT_OPEN = 1,
    REMOTE_PROC_CONNECT_CLOSE = 2,
    REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS = 273,
    /* ... 453+ procedures */
};
```

### 2. Parser (libvirt-codegen/src/parser.rs)

The parser uses [nom](https://github.com/rust-bakery/nom) to parse `.x` files into an AST:

```rust
// Parses: struct remote_domain { ... }
fn parse_struct(input: &str) -> IResult<&str, TypeDef>

// Parses: enum remote_procedure { ... }
fn parse_enum(input: &str) -> IResult<&str, TypeDef>

// Parses: typedef opaque remote_uuid[16];
fn parse_typedef(input: &str) -> IResult<&str, TypeDef>
```

### 3. Code Generator (libvirt-codegen/src/generator.rs)

The generator uses [quote](https://github.com/dtolnay/quote) to produce Rust code:

**Type Generation:**
```rust
// Input: struct remote_nonnull_domain { name; uuid; id; }
// Output:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonnullDomain {
    pub name: String,
    pub uuid: FixedOpaque16,
    pub id: i32,
}
```

**RPC Method Generation:**
```rust
// Input: REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS = 273
//        args: remote_connect_list_all_domains_args
//        ret:  remote_connect_list_all_domains_ret
// Output:
pub async fn connect_list_all_domains(
    &self,
    args: ConnectListAllDomainsArgs
) -> Result<ConnectListAllDomainsRet, RpcError> {
    let payload = libvirt_xdr::to_bytes(&args)?;
    let response = self.inner.rpc_call(
        Procedure::ProcConnectListAllDomains as u32,
        payload
    ).await?;
    libvirt_xdr::from_bytes(&response)
}
```

### 4. Build Integration (libvirt/build.rs)

```rust
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Parse .x protocol file
    let protocol = libvirt_codegen::parse_file("proto/remote_protocol.x")
        .expect("failed to parse protocol");

    // Generate Rust code
    let code = libvirt_codegen::generate(&protocol);

    // Write to OUT_DIR
    let dest = Path::new(&out_dir).join("generated.rs");
    fs::write(&dest, code).unwrap();

    println!("cargo:rerun-if-changed=proto/remote_protocol.x");
}
```

### 5. Include Generated Code (libvirt/src/lib.rs)

```rust
#[allow(dead_code)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

pub use generated::*;
```

## XDR Type Mapping

| XDR Type | Rust Type | Notes |
|----------|-----------|-------|
| `int` | `i32` | 4 bytes, big-endian |
| `unsigned int` | `u32` | 4 bytes, big-endian |
| `hyper` | `i64` | 8 bytes, big-endian |
| `unsigned hyper` | `u64` | 8 bytes, big-endian |
| `bool` | `bool` | 4 bytes (0 or 1) |
| `string<N>` | `String` | Length prefix + data + padding |
| `opaque<N>` | `Vec<u8>` | Variable length with prefix |
| `opaque[N]` | `FixedOpaque16` | Fixed length, no prefix (for N=16) |
| `T<N>` | `Vec<T>` | Variable length array |
| `T[N]` | `[T; N]` | Fixed length array |
| `T *` | `Option<T>` | Optional (discriminant + value) |
| `struct` | `struct` | Fields in order |
| `enum` | `enum` | `#[repr(i32)]` |
| `union` | `enum` | Tagged union |

## Usage

### Basic Example

```rust
use libvirt::{Client, ConnectListAllDomainsArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to libvirt daemon
    let client = Client::connect("qemu:///system").await?;

    // Use auto-generated API
    let version = client.rpc().connect_get_version().await?;
    println!("Hypervisor version: {}", version.hv_ver);

    // List all domains
    let args = ConnectListAllDomainsArgs {
        need_results: 1,
        flags: 0,
    };
    let ret = client.rpc().connect_list_all_domains(args).await?;

    for dom in &ret.domains {
        println!("Domain: {} ({})", dom.name, dom.uuid);
    }

    client.close().await?;
    Ok(())
}
```

### Domain Lifecycle Management

```rust
use libvirt::{Client, DomainLookupByNameArgs, DomainSuspendArgs};

async fn suspend_vm(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Lookup domain by name
    let args = DomainLookupByNameArgs { name: name.to_string() };
    let ret = client.rpc().domain_lookup_by_name(args).await?;

    // Suspend the domain
    let args = DomainSuspendArgs { dom: ret.dom };
    client.rpc().domain_suspend(args).await?;

    Ok(())
}
```

## Building

```bash
# Build the library
cargo build

# Run examples
cargo run --example domain_info
cargo run --example domain_lifecycle -- list
cargo run --example domain_lifecycle -- suspend <vm-name>
```

## Updating Protocol Definitions

To update the generated code when the libvirt protocol changes:

1. Download the latest `.x` files from libvirt source:
   ```bash
   curl -o proto/remote_protocol.x \
     https://raw.githubusercontent.com/libvirt/libvirt/master/src/remote/remote_protocol.x
   ```

2. Rebuild the project:
   ```bash
   cargo build
   ```

The code generator will automatically parse the new protocol and regenerate all types and methods.

## References

- [go-libvirt](https://github.com/digitalocean/go-libvirt) - Reference implementation in Go
- [libvirt RPC Protocol](https://libvirt.org/internals/rpc.html) - Official protocol documentation
- [XDR RFC 4506](https://tools.ietf.org/html/rfc4506) - XDR specification

## License

MIT License
