mod memory_cleaner;

use memory_cleaner::{CleanScope, MemoryCleaner, MemoryInfo};
use std::time::Duration;

fn main() {
    println!("=== Fast Memory Cleaner ===");
    println!("");

    let status;
    let mut error = String::new();

    let scope = CleanScope::ALL;
    let mut cleaner = MemoryCleaner::new();

    if let Err(e) = cleaner.acquire_privileges() {
        println!("权限获取失败: {}", e);
        println!("注意：部分清理功能可能需要管理员权限");
    }

    println!("正在执行内存清理...");
    println!("清理范围: {:?}", scope);
    println!("");

    let before = cleaner.get_memory_info();
    println!("清理前: {}", before.to_readable_string());

    match cleaner.clean(scope) {
        Ok(_) => {
            status = true;
        }
        Err(e) => {
            status = false;
            error = e.to_string();
        }
    }

    std::thread::sleep(Duration::from_millis(500));

    let after = cleaner.get_memory_info();
    println!("清理后: {}", after.to_readable_string());

    if status {
        println!("清理完成!");
    } else {
        println!("清理失败: {}", error);
        return;
    }

    let freed = before.used as i64 - after.used as i64;
    println!("");
    if freed > 0 {
        println!("成功释放内存: {}", MemoryInfo::format_bytes(freed as u64));
    } else {
        println!("内存变化: {}", MemoryInfo::format_bytes((-freed) as u64));
    }

    println!("");
    println!("=== 完成 ===");
}
