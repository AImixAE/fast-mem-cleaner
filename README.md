# Fast Memory Cleaner

一个基于 Windows API 的高性能内存清理 Rust 库。

## 功能特性

- **多维度内存清理**：支持工作集、文件缓存、修改页列表、备用列表等多种内存区域清理
- **权限管理**：自动获取系统特权（SeDebugPrivilege 等）
- **安全可靠**：使用原生 Windows NT API，清理操作安全可控
- **易于使用**：简洁的 API 设计，开箱即用

## 清理范围

| 选项                              | 说明                   |
| --------------------------------- | ---------------------- |
| `EMPTY_WORKING_SETS`              | 清空所有进程的工作集   |
| `FLUSH_FILE_CACHE`                | 刷新文件系统缓存       |
| `FLUSH_MODIFIED_LIST`             | 刷新修改页列表（脏页） |
| `PURGE_STANDBY_LIST`              | 清除备用列表内存       |
| `PURGE_LOW_PRIORITY_STANDBY_LIST` | 清除低优先级备用列表   |
| `ALL`                             | 执行所有清理操作       |

## 快速开始

### 添加依赖

```toml
[dependencies]
fast-mem-cleaner = { path = "path/to/this/library" }
```

### 基本使用

```rust
use memory_cleaner::{CleanScope, MemoryCleaner, MemoryInfo};

let mut cleaner = MemoryCleaner::new();

// 获取清理前内存状态
let before = cleaner.get_memory_info();
println!("清理前: {}", before.to_readable_string());

// 获取系统权限（需要管理员权限）
cleaner.acquire_privileges()?;

// 执行全面清理
cleaner.clean(CleanScope::ALL)?;

// 获取清理后内存状态
let after = cleaner.get_memory_info();
println!("清理后: {}", after.to_readable_string());
```

### 仅清理工作集

```rust
cleaner.clean(CleanScope::EMPTY_WORKING_SETS)?;
```

### 组合清理范围

```rust
use memory_cleaner::CleanScope;

// 只清理工作集和备用列表
let scope = CleanScope::EMPTY_WORKING_SETS | CleanScope::PURGE_STANDBY_LIST;
cleaner.clean(scope)?;
```

### 内存信息查询

```rust
let info = cleaner.get_memory_info();

// 格式化输出
println!("{}", info.to_readable_string());

// 单独访问
println!("总内存: {}", MemoryInfo::format_bytes(info.total));
println!("已使用: {}", MemoryInfo::format_bytes(info.used));
println!("空闲内存: {}", MemoryInfo::format_bytes(info.free));
```

## 技术实现

底层使用以下 Windows NT API：

- **`NtSetSystemInformation`** - 核心内核接口，用于操作系统内存列表
- **`EmptyWorkingSet`** - 清空进程工作集
- **`SetProcessWorkingSetSizeEx`** - 设置进程工作集大小
- **`AdjustTokenPrivileges`** - 获取系统特权

## 权限要求

部分清理功能需要管理员权限才能完整生效：

- 清空工作集
- 刷新修改页列表
- 清除备用列表

普通用户权限下部分操作可能受限。

## 示例程序

运行示例程序查看实际效果：

```bash
# 普通模式（部分功能受限）
cargo run

# 管理员模式（推荐）
# 以管理员身份运行生成的可执行文件
```

## 项目结构

```
src/
├── main.rs              # 示例程序
└── memory_cleaner.rs     # 核心库代码
```

## 许可证

MIT License
