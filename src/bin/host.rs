use std::{ffi::c_void, num::NonZeroU64};
use windows_sys::Win32::{
    Foundation::*,
    System::{
        Diagnostics::{Debug::ReadProcessMemory, ToolHelp::*},
        Memory::*,
        Threading::*,
    },
};

fn handle(id: u64) -> HANDLE {
    id as usize as HANDLE
}

#[no_mangle]
extern "C" fn process_attach_by_pid(pid: u64) -> Option<NonZeroU64> {
    unsafe {
        NonZeroU64::new(
            OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid as u32) as usize as u64,
        )
    }
}

#[no_mangle]
extern "C" fn process_detach(id: u64) {
    unsafe {
        CloseHandle(handle(id));
    }
}

#[no_mangle]
extern "C" fn process_is_open(id: u64) -> bool {
    let mut code = 0;
    unsafe { GetExitCodeProcess(handle(id), &mut code) != 0 && code == 259 }
}

#[no_mangle]
unsafe extern "C" fn process_read(id: u64, address: u64, out: *mut u8, len: usize) -> bool {
    let mut read = 0;
    ReadProcessMemory(
        handle(id),
        address as usize as *const c_void,
        out.cast(),
        len,
        &mut read,
    ) != 0
        && read == len
}

unsafe fn module(id: u64, name: *const u8, len: usize) -> Option<(u64, u64)> {
    let name = std::str::from_utf8(std::slice::from_raw_parts(name, len)).ok()?;
    let snapshot = CreateToolhelp32Snapshot(
        TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
        GetProcessId(handle(id)),
    );
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut entry: MODULEENTRY32W = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;
    let mut more = Module32FirstW(snapshot, &mut entry) != 0;
    let mut result = None;
    while more {
        let length = entry
            .szModule
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(entry.szModule.len());
        if String::from_utf16_lossy(&entry.szModule[..length]).eq_ignore_ascii_case(name) {
            result = Some((entry.modBaseAddr as usize as u64, entry.modBaseSize as u64));
            break;
        }
        more = Module32NextW(snapshot, &mut entry) != 0;
    }
    CloseHandle(snapshot);
    result
}

#[no_mangle]
unsafe extern "C" fn process_get_module_address(
    id: u64,
    name: *const u8,
    len: usize,
) -> Option<NonZeroU64> {
    NonZeroU64::new(module(id, name, len)?.0)
}
#[no_mangle]
unsafe extern "C" fn process_get_module_size(
    id: u64,
    name: *const u8,
    len: usize,
) -> Option<NonZeroU64> {
    NonZeroU64::new(module(id, name, len)?.1)
}

fn ranges(id: u64) -> Vec<(u64, u64)> {
    let mut result = Vec::new();
    let mut address: usize = 0;
    unsafe {
        let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        while VirtualQueryEx(
            handle(id),
            address as *const c_void,
            &mut info,
            std::mem::size_of_val(&info),
        ) != 0
        {
            if info.State == MEM_COMMIT && info.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0 {
                result.push((info.BaseAddress as usize as u64, info.RegionSize as u64));
            }
            let Some(next) = (info.BaseAddress as usize).checked_add(info.RegionSize) else {
                break;
            };
            if next <= address {
                break;
            }
            address = next;
        }
    }
    result
}
#[no_mangle]
extern "C" fn process_get_memory_range_count(id: u64) -> Option<NonZeroU64> {
    NonZeroU64::new(ranges(id).len() as u64)
}
#[no_mangle]
extern "C" fn process_get_memory_range_address(id: u64, index: u64) -> Option<NonZeroU64> {
    NonZeroU64::new(ranges(id).get(index as usize)?.0)
}
#[no_mangle]
extern "C" fn process_get_memory_range_size(id: u64, index: u64) -> Option<NonZeroU64> {
    NonZeroU64::new(ranges(id).get(index as usize)?.1)
}

#[no_mangle]
unsafe extern "C" fn runtime_print_message(text: *const u8, len: usize) {
    println!(
        "[ASR] {}",
        String::from_utf8_lossy(std::slice::from_raw_parts(text, len))
    );
}
