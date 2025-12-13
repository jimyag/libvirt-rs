# libvirt-rs 设计方案

参考 `github.com/digitalocean/go-libvirt`，用 Rust 实现 libvirt 客户端库。

## 整体架构

```plaintext
┌─────────────────────────────────────────────────────────────────┐
│                         libvirt-rs                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   .x 协议文件                                                    │
│       │                                                          │
│       ▼                                                          │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  libvirt-codegen (build.rs 调用)                        │   │
│   │  • 解析 .x 文件 (nom)                                   │   │
│   │  • 生成 Rust 代码 (quote + syn)                         │   │
│   │  • 格式化输出 (prettyplease)                            │   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│               ┌──────────────┴──────────────┐                   │
│               ▼                             ▼                   │
│   ┌─────────────────────┐       ┌─────────────────────────┐    │
│   │  生成的类型定义      │       │  生成的 API 方法         │    │
│   │  #[derive(Serialize,│       │  impl Client {           │    │
│   │   Deserialize)]     │       │    fn domain_start()     │    │
│   │  struct Domain {}   │       │    fn list_domains()     │    │
│   └─────────────────────┘       │  }                       │    │
│               │                 └─────────────────────────┘    │
│               ▼                             │                   │
│   ┌─────────────────────┐                   │                   │
│   │  libvirt-xdr        │                   │                   │
│   │  (serde 实现)       │◄──────────────────┘                   │
│   │  • XdrSerializer    │                                       │
│   │  • XdrDeserializer  │                                       │
│   └─────────────────────┘                                       │
│               │                                                  │
│               ▼                                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  libvirt-rpc                                            │   │
│   │  • 连接管理 (Unix/TCP/TLS)                              │   │
│   │  • 数据包封装/解析                                       │   │
│   │  • 请求/响应调度                                        │   │
│   │  • 事件流处理                                           │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 项目结构

```plaintext
libvirt-rs/
├── Cargo.toml                    # workspace 配置
├── proto/                        # libvirt 协议文件
│   ├── virnetprotocol.x          # 网络协议
│   ├── remote_protocol.x         # 远程协议 (主要)
│   └── qemu_protocol.x           # QEMU 特定协议
│
├── crates/
│   ├── libvirt-xdr/              # XDR 编解码 (serde 实现)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ser.rs            # serde Serializer
│   │       ├── de.rs             # serde Deserializer
│   │       └── error.rs
│   │
│   ├── libvirt-codegen/          # 代码生成器
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs         # .x 文件解析 (nom)
│   │       ├── ast.rs            # AST 定义
│   │       └── generator.rs      # Rust 代码生成 (quote)
│   │
│   └── libvirt/                  # 主库
│       ├── Cargo.toml
│       ├── build.rs              # 调用 libvirt-codegen
│       └── src/
│           ├── lib.rs
│           ├── generated.rs      # include! 生成代码
│           ├── client.rs         # 高层 API 封装
│           ├── connection.rs     # 连接管理
│           ├── transport/        # 传输层
│           │   ├── mod.rs
│           │   ├── unix.rs       # Unix socket
│           │   ├── tcp.rs        # TCP
│           │   └── tls.rs        # TLS
│           ├── packet.rs         # RPC 数据包
│           └── error.rs
│
└── examples/
    ├── list_domains.rs
    └── domain_lifecycle.rs
```

## 模块详细设计

### 1. libvirt-xdr (serde 实现)

**职责**: XDR 二进制格式的序列化/反序列化

**Cargo.toml**:

```toml
[package]
name = "libvirt-xdr"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
thiserror = "1"
bytes = "1"
```

**核心代码**:

```rust
// src/ser.rs
pub struct XdrSerializer {
    output: Vec<u8>,
}

impl<'a> serde::Serializer for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;
    // ... 实现各类型序列化
}

// src/de.rs
pub struct XdrDeserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> serde::Deserializer<'de> for &mut XdrDeserializer<'de> {
    // ... 实现各类型反序列化
}

// src/lib.rs
pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>>;
pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T>;
```

**XDR 类型映射**:

```plaintext
┌─────────────────┬─────────────────┬─────────────────────────────┐
│ XDR 类型        │ Rust 类型       │ 序列化规则                   │
├─────────────────┼─────────────────┼─────────────────────────────┤
│ int             │ i32             │ 4 字节大端                   │
│ unsigned int    │ u32             │ 4 字节大端                   │
│ hyper           │ i64             │ 8 字节大端                   │
│ unsigned hyper  │ u64             │ 8 字节大端                   │
│ float           │ f32             │ IEEE 754                    │
│ double          │ f64             │ IEEE 754                    │
│ bool            │ bool            │ 4 字节 (0 或 1)             │
│ string<N>       │ String          │ 长度(4) + 内容 + 填充(4对齐) │
│ opaque<N>       │ Vec<u8>         │ 长度(4) + 内容 + 填充(4对齐) │
│ opaque[N]       │ [u8; N]         │ 内容 + 填充(4对齐)          │
│ T<N>            │ Vec<T>          │ 长度(4) + 元素...           │
│ T[N]            │ [T; N]          │ 元素...                     │
│ T *             │ Option<T>       │ 标志(4) + 值(如果有)        │
│ enum            │ enum (repr i32) │ 4 字节大端                   │
│ struct          │ struct          │ 字段依次序列化               │
│ union           │ enum            │ 判别值(4) + 对应分支        │
└─────────────────┴─────────────────┴─────────────────────────────┘
```

### 2. libvirt-codegen (代码生成器)

**职责**: 解析 .x 文件，生成 Rust 类型定义和 API 方法

**Cargo.toml**:

```toml
[package]
name = "libvirt-codegen"
version = "0.1.0"
edition = "2021"

[dependencies]
nom = "7"              # .x 文件解析
quote = "1"            # 代码生成
syn = "2"              # 语法树
proc-macro2 = "1"      # TokenStream
prettyplease = "0.2"   # 代码格式化
heck = "0.4"           # 命名转换 (snake_case, PascalCase)
```

**AST 定义**:

```rust
// src/ast.rs
pub struct Protocol {
    pub name: String,
    pub types: Vec<TypeDef>,
    pub procedures: Vec<Procedure>,
    pub constants: Vec<Constant>,
}

pub enum TypeDef {
    Struct {
        name: String,
        fields: Vec<Field>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    Union {
        name: String,
        discriminant: Type,
        cases: Vec<UnionCase>,
        default: Option<Box<Type>>,
    },
    Typedef {
        name: String,
        target: Type,
    },
}

pub struct Procedure {
    pub name: String,           // REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME
    pub number: u32,            // 23
    pub args: Option<String>,   // 参数类型名
    pub ret: Option<String>,    // 返回类型名
    pub priority: Priority,     // HIGH / LOW
}

pub enum Type {
    Int,
    UInt,
    Hyper,
    UHyper,
    Float,
    Double,
    Bool,
    String { max_len: Option<u32> },
    Opaque { len: LengthSpec },
    Array { elem: Box<Type>, len: LengthSpec },
    Optional(Box<Type>),
    Named(String),
}

pub enum LengthSpec {
    Fixed(u32),
    Variable { max: Option<u32> },
}
```

**解析器** (使用 nom):

```rust
// src/parser.rs
use nom::{
    IResult,
    bytes::complete::tag,
    character::complete::{alphanumeric1, multispace0},
    // ...
};

pub fn parse_protocol(input: &str) -> IResult<&str, Protocol> {
    // 解析 .x 文件
}

fn parse_struct(input: &str) -> IResult<&str, TypeDef> {
    // struct name { field1; field2; };
}

fn parse_enum(input: &str) -> IResult<&str, TypeDef> {
    // enum name { VAR1 = 1, VAR2 = 2 };
}

fn parse_procedure(input: &str) -> IResult<&str, Procedure> {
    // REMOTE_PROC_XXX = N
}
```

**代码生成器**:

```rust
// src/generator.rs
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

pub fn generate(protocol: &Protocol) -> String {
    let mut tokens = TokenStream::new();

    // 1. 生成类型定义
    for ty in &protocol.types {
        tokens.extend(generate_type(ty));
    }

    // 2. 生成常量
    for constant in &protocol.constants {
        tokens.extend(generate_constant(constant));
    }

    // 3. 生成 Client impl
    tokens.extend(generate_client(&protocol.procedures));

    // 4. 格式化
    let file = syn::parse2(tokens).expect("generated invalid code");
    prettyplease::unparse(&file)
}

fn generate_type(ty: &TypeDef) -> TokenStream {
    match ty {
        TypeDef::Struct { name, fields } => {
            let name_ident = format_ident!("{}", to_pascal_case(name));
            let field_tokens: Vec<_> = fields.iter().map(|f| {
                let fname = format_ident!("{}", to_snake_case(&f.name));
                let ftype = type_to_tokens(&f.ty);
                quote! { pub #fname: #ftype }
            }).collect();

            quote! {
                #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
                pub struct #name_ident {
                    #(#field_tokens),*
                }
            }
        }
        TypeDef::Enum { name, variants } => {
            let name_ident = format_ident!("{}", to_pascal_case(name));
            let variant_tokens: Vec<_> = variants.iter().map(|v| {
                let vname = format_ident!("{}", to_pascal_case(&v.name));
                let vval = v.value;
                quote! { #vname = #vval }
            }).collect();

            quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[derive(serde::Serialize, serde::Deserialize)]
                #[repr(i32)]
                pub enum #name_ident {
                    #(#variant_tokens),*
                }
            }
        }
        // Union, Typedef...
    }
}

fn generate_client(procedures: &[Procedure]) -> TokenStream {
    let methods: Vec<_> = procedures.iter().map(|proc| {
        let method_name = format_ident!("{}", proc_to_method_name(&proc.name));
        let proc_num = proc.number as i32;

        match (&proc.args, &proc.ret) {
            (Some(args), Some(ret)) => {
                let args_type = format_ident!("{}", to_pascal_case(args));
                let ret_type = format_ident!("{}", to_pascal_case(ret));
                quote! {
                    pub async fn #method_name(&self, args: #args_type) -> Result<#ret_type> {
                        self.call(#proc_num, &args).await
                    }
                }
            }
            (Some(args), None) => {
                let args_type = format_ident!("{}", to_pascal_case(args));
                quote! {
                    pub async fn #method_name(&self, args: #args_type) -> Result<()> {
                        self.call(#proc_num, &args).await
                    }
                }
            }
            (None, Some(ret)) => {
                let ret_type = format_ident!("{}", to_pascal_case(ret));
                quote! {
                    pub async fn #method_name(&self) -> Result<#ret_type> {
                        self.call(#proc_num, &()).await
                    }
                }
            }
            (None, None) => {
                quote! {
                    pub async fn #method_name(&self) -> Result<()> {
                        self.call(#proc_num, &()).await
                    }
                }
            }
        }
    }).collect();

    quote! {
        impl Client {
            #(#methods)*
        }
    }
}
```

### 3. libvirt (主库)

**Cargo.toml**:

```toml
[package]
name = "libvirt"
version = "0.1.0"
edition = "2021"

[dependencies]
libvirt-xdr = { path = "../libvirt-xdr" }
tokio = { version = "1", features = ["net", "io-util", "sync", "time"] }
tokio-rustls = "0.25"       # TLS 支持
serde = { version = "1", features = ["derive"] }
thiserror = "1"
bytes = "1"
dashmap = "5"               # 并发 HashMap

[build-dependencies]
libvirt-codegen = { path = "../libvirt-codegen" }
```

**build.rs**:

```rust
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // 解析并生成代码
    let protocol = libvirt_codegen::parse_file("../../proto/remote_protocol.x")
        .expect("failed to parse protocol");

    let code = libvirt_codegen::generate(&protocol);

    // 写入文件
    let dest = Path::new(&out_dir).join("generated.rs");
    fs::write(&dest, code).expect("failed to write generated code");

    // 监听变化
    println!("cargo:rerun-if-changed=../../proto/remote_protocol.x");
    println!("cargo:rerun-if-changed=build.rs");
}
```

**连接管理**:

```rust
// src/connection.rs
use std::sync::atomic::{AtomicU32, Ordering};
use dashmap::DashMap;
use tokio::sync::oneshot;

pub struct Connection {
    transport: Box<dyn Transport + Send + Sync>,
    serial: AtomicU32,
    pending: DashMap<u32, oneshot::Sender<Vec<u8>>>,
}

impl Connection {
    pub async fn connect(uri: &str) -> Result<Self> {
        let transport: Box<dyn Transport + Send + Sync> = match uri {
            s if s.contains("///system") || s.contains("///session") => {
                let path = if s.contains("///system") {
                    "/var/run/libvirt/libvirt-sock"
                } else {
                    // ~/.cache/libvirt/libvirt-sock
                    todo!()
                };
                Box::new(UnixTransport::connect(path).await?)
            }
            s if s.contains("+tcp://") => {
                let addr = parse_tcp_addr(s)?;
                Box::new(TcpTransport::connect(addr).await?)
            }
            s if s.contains("+tls://") => {
                let addr = parse_tcp_addr(s)?;
                Box::new(TlsTransport::connect(addr).await?)
            }
            _ => return Err(Error::UnsupportedUri(uri.to_string())),
        };

        let conn = Self {
            transport,
            serial: AtomicU32::new(1),
            pending: DashMap::new(),
        };

        // 启动接收循环
        conn.spawn_receiver();

        Ok(conn)
    }

    pub async fn call<Req, Resp>(&self, procedure: i32, args: &Req) -> Result<Resp>
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let serial = self.serial.fetch_add(1, Ordering::SeqCst);

        // 序列化参数
        let payload = libvirt_xdr::to_bytes(args)?;

        // 构造数据包
        let packet = Packet::new_call(procedure, serial, payload);

        // 注册等待响应
        let (tx, rx) = oneshot::channel();
        self.pending.insert(serial, tx);

        // 发送请求
        self.transport.send(&packet.to_bytes()).await?;

        // 等待响应
        let response_bytes = rx.await.map_err(|_| Error::ConnectionClosed)?;

        // 反序列化
        libvirt_xdr::from_bytes(&response_bytes)
    }

    fn spawn_receiver(&self) {
        // 启动后台任务读取响应
        tokio::spawn(async move {
            loop {
                match self.transport.recv().await {
                    Ok(data) => {
                        let packet = Packet::parse(&data)?;
                        if let Some((_, tx)) = self.pending.remove(&packet.serial) {
                            let _ = tx.send(packet.payload);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
}
```

**数据包格式**:

```rust
// src/packet.rs

/// libvirt RPC 数据包
///
/// ```plaintext
/// ┌────────────┬────────────┬────────────┬────────────┬──────────┐
/// │ length (4) │ program(4) │ version(4) │ procedure  │ type (4) │
/// ├────────────┼────────────┼────────────┤    (4)     ├──────────┤
/// │ serial (4) │ status (4) │            payload ...             │
/// └────────────┴────────────┴────────────────────────────────────┘
/// ```
pub struct Packet {
    pub program: u32,
    pub version: u32,
    pub procedure: i32,
    pub msg_type: MsgType,
    pub serial: u32,
    pub status: Status,
    pub payload: Vec<u8>,
}

pub const REMOTE_PROGRAM: u32 = 0x20008086;
pub const REMOTE_PROTOCOL_VERSION: u32 = 1;

#[repr(u32)]
pub enum MsgType {
    Call = 0,
    Reply = 1,
    Message = 2,  // 事件
    Stream = 3,
}

#[repr(u32)]
pub enum Status {
    Ok = 0,
    Error = 1,
    Continue = 2,
}

impl Packet {
    pub fn new_call(procedure: i32, serial: u32, payload: Vec<u8>) -> Self {
        Self {
            program: REMOTE_PROGRAM,
            version: REMOTE_PROTOCOL_VERSION,
            procedure,
            msg_type: MsgType::Call,
            serial,
            status: Status::Ok,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let header_len = 24u32; // 6 * 4 bytes
        let total_len = header_len + self.payload.len() as u32;

        let mut buf = Vec::with_capacity(total_len as usize + 4);
        buf.extend_from_slice(&total_len.to_be_bytes());
        buf.extend_from_slice(&self.program.to_be_bytes());
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.procedure.to_be_bytes());
        buf.extend_from_slice(&(self.msg_type as u32).to_be_bytes());
        buf.extend_from_slice(&self.serial.to_be_bytes());
        buf.extend_from_slice(&(self.status as u32).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        // 解析数据包
        todo!()
    }
}
```

**高层 API**:

```rust
// src/client.rs
use std::sync::Arc;

pub struct Client {
    conn: Arc<Connection>,
}

impl Client {
    pub async fn connect(uri: &str) -> Result<Self> {
        let conn = Connection::connect(uri).await?;

        // 认证流程
        let auth_list = conn.auth_list().await?;
        // 选择认证方式...

        // 打开连接
        conn.connect_open(ConnectOpenArgs {
            name: Some(uri.to_string()),
            flags: 0,
        }).await?;

        Ok(Self { conn: Arc::new(conn) })
    }

    pub async fn list_all_domains(&self, flags: u32) -> Result<Vec<Domain>> {
        let ret = self.conn.connect_list_all_domains(
            ConnectListAllDomainsArgs { need_results: 1, flags }
        ).await?;

        Ok(ret.domains.into_iter()
            .map(|d| Domain::new(self.conn.clone(), d))
            .collect())
    }
}

// src/domain.rs
pub struct Domain {
    conn: Arc<Connection>,
    inner: RemoteNonnullDomain,
}

impl Domain {
    pub fn uuid(&self) -> &[u8; 32] {
        &self.inner.uuid
    }

    pub async fn name(&self) -> Result<String> {
        let ret = self.conn.domain_get_name(DomainGetNameArgs {
            dom: self.inner.clone(),
        }).await?;
        Ok(ret.name)
    }

    pub async fn start(&self) -> Result<()> {
        self.conn.domain_create(DomainCreateArgs {
            dom: self.inner.clone(),
        }).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.conn.domain_shutdown(DomainShutdownArgs {
            dom: self.inner.clone(),
        }).await
    }

    pub async fn get_xml_desc(&self, flags: u32) -> Result<String> {
        let ret = self.conn.domain_get_xml_desc(DomainGetXmlDescArgs {
            dom: self.inner.clone(),
            flags,
        }).await?;
        Ok(ret.xml)
    }
}
```

## 实现步骤

### Phase 1: 基础设施

1. **创建项目结构**
   - 初始化 workspace
   - 创建各 crate 目录
   - 复制 libvirt .x 协议文件

2. **libvirt-xdr: serde XDR 实现**
   - 实现 XdrSerializer
   - 实现 XdrDeserializer
   - 单元测试：各基础类型

3. **libvirt-codegen: .x 解析器**
   - 解析 struct
   - 解析 enum
   - 解析 union
   - 解析 typedef
   - 解析 const
   - 解析 procedure 定义

### Phase 2: 代码生成

4. **libvirt-codegen: Rust 代码生成**
   - 生成 struct 定义
   - 生成 enum 定义
   - 生成 union 定义 (Rust tagged union)
   - 生成 Client impl 方法

5. **libvirt: build.rs 集成**
   - 调用 codegen 生成代码
   - include! 引入生成代码
   - 验证编译通过

### Phase 3: RPC 通信

6. **libvirt: 连接管理**
   - Unix socket transport
   - 数据包封装/解析
   - 请求/响应调度

7. **libvirt: 认证流程**
   - AUTH_NONE (默认)
   - AUTH_POLKIT (可选)

8. **集成测试**
   - 连接 libvirtd
   - 列出 domains
   - 获取 domain 信息

### Phase 4: 完善

9. **高层 API 封装**
   - Domain 操作
   - Network 操作
   - Storage Pool/Volume

10. **事件支持**
    - Domain lifecycle events
    - 事件流处理

11. **其他传输**
    - TCP transport
    - TLS transport

## 依赖汇总

```toml
# workspace Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
# 序列化
serde = { version = "1", features = ["derive"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }

# TLS
tokio-rustls = "0.25"
rustls = "0.22"

# 代码生成
quote = "1"
syn = { version = "2", features = ["full"] }
proc-macro2 = "1"
prettyplease = "0.2"

# 解析
nom = "7"

# 工具
thiserror = "1"
bytes = "1"
dashmap = "5"
heck = "0.4"
```

## 使用示例

```rust
use libvirt::{Client, ListDomainsFlags};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接
    let client = Client::connect("qemu:///system").await?;

    // 列出所有虚拟机
    let domains = client.list_all_domains(ListDomainsFlags::all()).await?;

    for domain in &domains {
        println!("Name: {}", domain.name().await?);
        println!("UUID: {:?}", domain.uuid());
        println!("State: {:?}", domain.state().await?);
        println!();
    }

    // 启动虚拟机
    if let Some(vm) = domains.iter().find(|d| d.name().await.ok() == Some("test-vm".into())) {
        vm.start().await?;
    }

    Ok(())
}
```

## 参考资料

- go-libvirt: https://github.com/digitalocean/go-libvirt
- libvirt RPC 协议: https://libvirt.org/internals/rpc.html
- XDR RFC 4506: https://tools.ietf.org/html/rfc4506
- prost-build (参考实现): https://github.com/tokio-rs/prost
