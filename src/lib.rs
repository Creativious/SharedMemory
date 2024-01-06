pub mod shared_memory {
    #[cfg(target_os = "windows")]
    extern crate winapi;

    #[cfg(target_os = "linux")]
    extern crate libc;

    use std::ffi::CString;
    use std::{io, ptr};
    use std::io::Error;

    #[cfg(target_os = "windows")]
    use winapi::shared::minwindef::*;

    #[cfg(target_os = "windows")]
    use winapi::ctypes::c_void as win_c_void;

    #[cfg(target_os = "windows")]
    use winapi::um::memoryapi::*;

    #[cfg(target_os = "windows")]
    use winapi::um::handleapi::*;

    #[cfg(target_os = "windows")]
    use winapi::um::winnt::*;

    #[cfg(target_os = "windows")]
    use winapi::um::winbase::*;

    #[cfg(target_os = "windows")]
    use winapi::shared::basetsd::*;

    #[cfg(target_os = "linux")]
    use libc::{c_int, c_void as lin_c_void, c_char, size_t, shm_open, mmap, PROT_READ, PROT_WRITE, MAP_SHARED, O_RDWR, O_CREAT, O_EXCL, close, ftruncate, munmap, shm_unlink};

    use std::os::raw::{c_char};

    pub struct SharedMemory {
        size: i32,
        name: *const c_char,

        #[cfg(target_os = "windows")]
        h_map_file: HANDLE,
        #[cfg(target_os = "windows")]
        p_buf: *mut win_c_void,

        #[cfg(target_os = "linux")]
        p_buf: *mut lin_c_void,
        #[cfg(target_os = "linux")]
        is_create: bool,
    }

    impl SharedMemory {
        pub fn size(&self) -> i32 {
            self.size
        }

        pub fn name(&self) -> String {
            unsafe {
                let c_str = CString::from_raw(self.name as *mut c_char);
                let str_slice = c_str.to_str().unwrap();
                str_slice.to_string()
            }
        }

        #[cfg(target_os = "windows")]
        pub fn address(&self) -> *mut win_c_void {
            self.p_buf
        }

        #[cfg(target_os = "linux")]
        pub fn address(&self) -> *mut lin_c_void {
            self.p_buf
        }

        #[cfg(target_os = "windows")]
        pub fn create(name: &str, size: i32) -> Result<Self, io::Error> {
            let name_c = CString::new(name).expect("CSTRING::new failed");
            let h_map_file = unsafe {
                CreateFileMappingA(
                    INVALID_HANDLE_VALUE,
                    ptr::null_mut(),
                    PAGE_READWRITE | SEC_COMMIT,
                    0,
                    size as DWORD,
                    name_c.as_ptr(),
                )
            };
            if h_map_file.is_null() {
                return Err(Error::last_os_error());
            }
            let p_buf = unsafe {
                MapViewOfFile(
                    h_map_file,
                    FILE_MAP_ALL_ACCESS,
                    0,
                    0,
                    size as SIZE_T
                )
            };
            if p_buf.is_null() {
                unsafe {
                    CloseHandle(h_map_file);
                }
                return Err(Error::last_os_error());
            }
            let shared_memory = SharedMemory {
                size,
                name: name_c.into_raw(),
                h_map_file,
                p_buf,
            };
            Ok(shared_memory)
        }

        #[cfg(target_os = "linux")]
        pub fn create(name: &str, size: i32) -> Result<Self, io::Error> {
            let name_c = CString::new(name).expect("CString::new failed");
            let fd = unsafe {
                shm_open(
                    name_c.as_ptr(),
                    O_RDWR | O_CREAT | O_EXCL,
                    0o600,
                )
            };
            if fd == -1 {
                return Err(io::Error::last_os_error());
            }
            if unsafe { ftruncate(fd, size as off_t) } == -1 {
                unsafe {
                    close(fd);
                }
                return Err(io::Error::last_os_error());
            }
            let p_buf = unsafe {
                mmap(
                    ptr::null_mut(),
                    size as size_t,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED,
                    fd,
                    0,
                )
            };
            if p_buf == libc::MAP_FAILED {
                unsafe {
                    close(fd);
                }
                return Err(io::Error::last_os_error());
            }
            let shared_memory = SharedMemory {
                size,
                name: name_c.into_raw(),
                p_buf,
                is_create: true,
            };
            Ok(shared_memory)
        }

        #[cfg(target_os = "windows")]
        pub fn open(name: &str, size: i32) -> Result<Self, Error> {
            let name_c = CString::new(name).expect("CSTRING::new failed");
            let h_map_file = unsafe {
                OpenFileMappingA(
                    FILE_MAP_ALL_ACCESS,
                    FALSE,
                    name_c.as_ptr(),
                )
            };
            if h_map_file.is_null() {
                return Err(Error::last_os_error());
            }
            let p_buf = unsafe {
                MapViewOfFile(
                    h_map_file,
                    FILE_MAP_ALL_ACCESS,
                    0,
                    0,
                    size as SIZE_T
                )
            };
            if p_buf.is_null() {
                unsafe {
                    CloseHandle(h_map_file);
                }
                return Err(Error::last_os_error());
            }
            let shared_memory = SharedMemory {
                size,
                name: name_c.into_raw(),
                h_map_file,
                p_buf,
            };
            Ok(shared_memory)
        }

        pub fn write_data(&self, data: &[u8]) {
            let data_size = data.len();
            let offset = self.size() as usize - data_size;

            if offset < data_size {
                eprintln!("Error: Attempted to write data with a size larger than the shared memory region");
                return;
            }
            if let Some(size) = self.size.checked_sub(data.len() as i32) {
                let dest = self.address() as *mut u8;
                unsafe {
                    ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());
                    ptr::write_bytes(dest.add(data.len()), 0, size as usize - data.len());
                }
            } else {
                eprintln!("Error: Attempted to write data with a size larger than the shared memory region");
            }
        }

        pub fn read_data(&self) -> &[u8] {
            let dest = self.address() as *const u8;
            let mut size = self.size() as usize;

            // Check for empty bits at the end of the data
            unsafe {
                let mut idx = size - 1;
                while idx >= 0 {
                    if *dest.add(idx) != 0 {
                        break;
                    }
                    idx -= 1;
                }
                size = idx + 1;
            }

            unsafe {
                std::slice::from_raw_parts(dest, size)
            }
        }

        pub fn write_string(&mut self, data: &str) {
            self.write_data(data.as_bytes());
        }

        pub fn read_string(&self) -> String {
            let bytes = self.read_data();
            String::from_utf8_lossy(&bytes).to_string()
        }



        #[cfg(target_os = "linux")]
        pub fn open(name: &str, size: i32) -> Result<Self, io::Error> {
            let name_c = CString::new(name).expect("CString::new failed");
            let fd = unsafe {
                shm_open(
                    name_c.as_ptr(),
                    O_RDWR,
                    0o600,
                )
            };
            if fd == -1 {
                return Err(io::Error::last_os_error());
            }
            let p_buf = unsafe {
                mmap(
                    ptr::null_mut(),
                    size as size_t,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED,
                    fd,
                    0,
                )
            };
            if p_buf == libc::MAP_FAILED {
                unsafe {
                    close(fd);
                }
                return Err(io::Error::last_os_error());
            }
            let shared_memory = SharedMemory {
                size,
                name: name_c.into_raw(),
                p_buf,
                is_create: false,
            };
            Ok(shared_memory)
        }
    }

    impl Drop for SharedMemory {
        fn drop(&mut self) {
            unsafe {
                #[cfg(target_os = "windows")]
                {
                    UnmapViewOfFile(self.p_buf);
                    CloseHandle(self.h_map_file);
                }
                #[cfg(target_os = "linux")]
                {
                    if (!self.p_buf.is_null()) {
                        munmap(self.p_buf, self.size as size_t);
                    }
                    if (self.is_create) {
                        shm_unlink(self.name);
                    }
                }
                let _ = CString::from_raw(self.name as *mut c_char);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
}
