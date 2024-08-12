use parking_lot::Mutex;
use std::ffi::CStr;
use std::io::{Error, Result};
use std::mem::size_of;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::ReadProcessMemory;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::psapi::{EnumProcessModules, EnumProcesses, GetModuleBaseNameA};
use winapi::um::winnt::PROCESS_ALL_ACCESS;

pub type Address = usize;

pub struct MemoryReader {
    handle: Mutex<*mut winapi::ctypes::c_void>,
}

unsafe impl Send for MemoryReader {}
unsafe impl Sync for MemoryReader {}

impl MemoryReader {
    pub fn new(pid: u32) -> Self {
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        MemoryReader {
            handle: Mutex::new(handle),
        }
    }

    pub fn read<T: Copy>(&self, address: Address) -> Result<T> {
        let handle = self.handle.lock();
        let mut buffer: T = unsafe { std::mem::zeroed() };
        let mut bytes_read = 0;

        let success = unsafe {
            ReadProcessMemory(
                *handle,
                address as *const winapi::ctypes::c_void,
                &mut buffer as *mut T as *mut winapi::ctypes::c_void,
                std::mem::size_of::<T>(),
                &mut bytes_read,
            ) != 0
        };

        if success {
            Ok(buffer)
        } else {
            Err(Error::last_os_error())
        }
    }

    pub fn follow_pointers(&self, base: Address, offsets: &[usize]) -> Result<Address> {
        let mut addr = base;
        for &offset in offsets {
            addr = self.read::<usize>(addr)?;
            addr += offset;
        }
        Ok(addr)
    }

    pub fn get_module_base(&self, module_name: &str) -> Result<Address> {
        let handle = self.handle.lock();
        let mut module = std::ptr::null_mut();
        let mut needed = 0;

        unsafe {
            if EnumProcessModules(
                *handle,
                &mut module,
                size_of::<*mut winapi::ctypes::c_void>() as u32,
                &mut needed,
            ) == 0
            {
                return Err(Error::last_os_error());
            }

            let mut name = [0i8; 260];
            if GetModuleBaseNameA(*handle, module, name.as_mut_ptr(), 260) == 0 {
                return Err(Error::last_os_error());
            }

            let name = CStr::from_ptr(name.as_ptr()).to_string_lossy();
            if name == module_name {
                Ok(module as usize)
            } else {
                Err(Error::new(std::io::ErrorKind::NotFound, "Module not found"))
            }
        }
    }
}

impl Drop for MemoryReader {
    fn drop(&mut self) {
        let handle = self.handle.lock();
        unsafe { CloseHandle(*handle) };
    }
}

pub fn get_pid(process_name: &str) -> Option<u32> {
    let mut processes = [0u32; 1024];
    let mut bytes_returned = 0;

    unsafe {
        if EnumProcesses(
            processes.as_mut_ptr(),
            size_of::<[u32; 1024]>() as u32,
            &mut bytes_returned,
        ) == 0
        {
            return None;
        }
    }

    let count = bytes_returned as usize / size_of::<u32>();

    for &pid in &processes[..count] {
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        if handle.is_null() {
            continue;
        }

        let mut module = std::ptr::null_mut();
        let mut needed = 0;
        unsafe {
            if EnumProcessModules(
                handle,
                &mut module,
                size_of::<*mut winapi::ctypes::c_void>() as u32,
                &mut needed,
            ) != 0
            {
                let mut name = [0i8; 260];
                if GetModuleBaseNameA(handle, module, name.as_mut_ptr(), 260) > 0 {
                    let name = CStr::from_ptr(name.as_ptr()).to_string_lossy();
                    if name == process_name {
                        CloseHandle(handle);
                        return Some(pid);
                    }
                }
            }
            CloseHandle(handle);
        }
    }

    None
}
