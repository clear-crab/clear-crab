//@only-target-windows: this directly tests windows only random functions
use core::ffi::c_void;
use core::mem::size_of_val;
use core::ptr::null_mut;

// Windows API definitions.
type NTSTATUS = i32;
type BOOLEAN = u8;
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x00000002;
const BCRYPT_RNG_ALG_HANDLE: *mut c_void = 0x81 as *mut c_void;
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(
        halgorithm: *mut c_void,
        pbbuffer: *mut u8,
        cbbuffer: u32,
        dwflags: u32,
    ) -> NTSTATUS;
}
#[link(name = "advapi32")]
extern "system" {
    #[link_name = "SystemFunction036"]
    fn RtlGenRandom(RandomBuffer: *mut u8, RandomBufferLength: u32) -> BOOLEAN;
}

fn main() {
    let mut key = [0u8; 24];
    let len: u32 = size_of_val(&key).try_into().unwrap();
    let ret = unsafe {
        BCryptGenRandom(null_mut(), key.as_mut_ptr(), len, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
    };
    // NTSTATUS codes use the high bit to indicate an error
    assert!(ret >= 0);

    let ret = unsafe { BCryptGenRandom(BCRYPT_RNG_ALG_HANDLE, key.as_mut_ptr(), len, 0) };
    assert!(ret >= 0);

    let ret = unsafe { RtlGenRandom(key.as_mut_ptr(), len) };
    // RtlGenRandom returns a BOOLEAN where 0 indicates an error
    assert_ne!(ret, 0);
}
