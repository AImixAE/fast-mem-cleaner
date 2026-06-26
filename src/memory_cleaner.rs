use bitflags::bitflags;
use ntapi::ntexapi::NtSetSystemInformation;
use std::mem;
use sysinfo::System;
use thiserror::Error;
use winapi::shared::minwindef::{DWORD, LPVOID};
use winapi::shared::ntdef::{NTSTATUS, ULONG};
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::SetProcessWorkingSetSizeEx;
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcess, OpenProcessToken};
use winapi::um::psapi::EmptyWorkingSet;
use winapi::um::securitybaseapi::AdjustTokenPrivileges;
use winapi::um::winbase::LookupPrivilegeValueW;
use winapi::um::winnt::{
    HANDLE, LUID, PROCESS_ALL_ACCESS, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};

#[derive(Error, Debug)]
pub enum MemoryCleanerError {
    #[error("Failed to open process token")]
    OpenProcessTokenFailed,

    #[error("Failed to look up privilege value")]
    LookupPrivilegeValueFailed,

    #[error("Failed to adjust token privileges")]
    AdjustTokenPrivilegesFailed,

    #[error("NtSetSystemInformation failed with status: {0}")]
    NtSetSystemInformationFailed(NTSTATUS),
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CleanScope: u32 {
        const EMPTY_WORKING_SETS = 0x00000001;
        const FLUSH_FILE_CACHE = 0x00000002;
        const FLUSH_MODIFIED_LIST = 0x00000004;
        const PURGE_STANDBY_LIST = 0x00000008;
        const PURGE_LOW_PRIORITY_STANDBY_LIST = 0x00000010;
        const ALL = Self::EMPTY_WORKING_SETS.bits()
            | Self::FLUSH_FILE_CACHE.bits()
            | Self::FLUSH_MODIFIED_LIST.bits()
            | Self::PURGE_STANDBY_LIST.bits()
            | Self::PURGE_LOW_PRIORITY_STANDBY_LIST.bits();
    }
}

const SYSTEM_MEMORY_LIST_INFORMATION: ULONG = 0x50;

#[repr(C)]
enum MEMORY_LIST_COMMAND {
    MemoryEmptyWorkingSets = 1,
    MemoryFlushModifiedList = 2,
    MemoryPurgeStandbyList = 3,
    MemoryPurgeLowPriorityStandbyList = 4,
    MemoryFlushFileBuffers = 5,
}

#[repr(C)]
struct SYSTEM_MEMORY_LIST_COMMAND {
    command: MEMORY_LIST_COMMAND,
    unknown: ULONG,
}

pub struct MemoryCleaner {
    privileges_acquired: bool,
}

impl MemoryCleaner {
    pub fn new() -> Self {
        MemoryCleaner {
            privileges_acquired: false,
        }
    }

    pub fn acquire_privileges(&mut self) -> Result<(), MemoryCleanerError> {
        if self.privileges_acquired {
            return Ok(());
        }

        let privileges = [
            "SeProfileSingleProcessPrivilege",
            "SeIncreaseQuotaPrivilege",
            "SeDebugPrivilege",
        ];

        for privilege in privileges.iter() {
            Self::set_privilege(privilege)?;
        }

        self.privileges_acquired = true;
        Ok(())
    }

    fn set_privilege(name: &str) -> Result<(), MemoryCleanerError> {
        unsafe {
            let mut h_token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut h_token,
            ) == 0
            {
                return Err(MemoryCleanerError::OpenProcessTokenFailed);
            }

            let mut luid: LUID = mem::zeroed();
            let privilege_wstr: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            if LookupPrivilegeValueW(std::ptr::null(), privilege_wstr.as_ptr(), &mut luid) == 0 {
                return Err(MemoryCleanerError::LookupPrivilegeValueFailed);
            }

            let mut tp: TOKEN_PRIVILEGES = mem::zeroed();
            tp.PrivilegeCount = 1;
            tp.Privileges[0].Luid = luid;
            tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

            if AdjustTokenPrivileges(
                h_token,
                0,
                &mut tp,
                mem::size_of::<TOKEN_PRIVILEGES>() as DWORD,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(MemoryCleanerError::AdjustTokenPrivilegesFailed);
            }

            CloseHandle(h_token);
        }

        Ok(())
    }

    pub fn clean(&mut self, scope: CleanScope) -> Result<(), MemoryCleanerError> {
        self.acquire_privileges()?;

        if scope.contains(CleanScope::FLUSH_FILE_CACHE) {
            Self::nt_flush_file_buffers()?;
        }

        if scope.contains(CleanScope::FLUSH_MODIFIED_LIST) {
            Self::nt_flush_modified_list()?;
        }

        if scope.contains(CleanScope::PURGE_STANDBY_LIST) {
            Self::nt_purge_standby_list()?;
        }

        if scope.contains(CleanScope::PURGE_LOW_PRIORITY_STANDBY_LIST) {
            Self::nt_purge_low_priority_standby_list()?;
        }

        if scope.contains(CleanScope::EMPTY_WORKING_SETS) {
            Self::empty_all_working_sets()?;
        }

        Ok(())
    }

    fn nt_set_memory_list_command(command: MEMORY_LIST_COMMAND) -> Result<(), MemoryCleanerError> {
        unsafe {
            let mut cmd: SYSTEM_MEMORY_LIST_COMMAND = SYSTEM_MEMORY_LIST_COMMAND {
                command,
                unknown: 0,
            };

            let status = NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut _ as LPVOID,
                mem::size_of::<SYSTEM_MEMORY_LIST_COMMAND>() as ULONG,
            );

            if status != 0 {
                return Err(MemoryCleanerError::NtSetSystemInformationFailed(status));
            }
        }

        Ok(())
    }

    fn nt_flush_file_buffers() -> Result<(), MemoryCleanerError> {
        Self::nt_set_memory_list_command(MEMORY_LIST_COMMAND::MemoryFlushFileBuffers)
    }

    fn nt_flush_modified_list() -> Result<(), MemoryCleanerError> {
        Self::nt_set_memory_list_command(MEMORY_LIST_COMMAND::MemoryFlushModifiedList)
    }

    fn nt_purge_standby_list() -> Result<(), MemoryCleanerError> {
        Self::nt_set_memory_list_command(MEMORY_LIST_COMMAND::MemoryPurgeStandbyList)
    }

    fn nt_purge_low_priority_standby_list() -> Result<(), MemoryCleanerError> {
        Self::nt_set_memory_list_command(MEMORY_LIST_COMMAND::MemoryPurgeLowPriorityStandbyList)
    }

    fn nt_empty_working_sets() -> Result<(), MemoryCleanerError> {
        Self::nt_set_memory_list_command(MEMORY_LIST_COMMAND::MemoryEmptyWorkingSets)
    }

    fn empty_all_working_sets() -> Result<(), MemoryCleanerError> {
        Self::nt_empty_working_sets()?;

        let system = System::new_all();
        for process in system.processes().values() {
            unsafe {
                let pid = process.pid().as_u32();
                let h_process = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
                if h_process != std::ptr::null_mut() {
                    EmptyWorkingSet(h_process);
                    SetProcessWorkingSetSizeEx(h_process, 0, 0, 0);
                    CloseHandle(h_process);
                }
            }
        }

        Ok(())
    }

    pub fn get_memory_info(&self) -> MemoryInfo {
        let system = System::new_all();
        MemoryInfo {
            total: system.total_memory(),
            free: system.free_memory(),
            used: system.used_memory(),
        }
    }
}

pub struct MemoryInfo {
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

impl MemoryInfo {
    pub fn to_readable_string(&self) -> String {
        format!(
            "Total: {} | Free: {} | Used: {} ({:.1}%)",
            Self::format_bytes(self.total),
            Self::format_bytes(self.free),
            Self::format_bytes(self.used),
            (self.used as f64 / self.total as f64) * 100.0
        )
    }

    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes == 0 {
            return "0 B".to_string();
        } else if bytes <= KB {
            format!("{} B", bytes)
        } else if bytes <= MB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else if bytes <= GB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        }
    }
}

impl Default for MemoryCleaner {
    fn default() -> Self {
        Self::new()
    }
}
