use std::ffi::{c_void, CStr};
use std::mem;
use std::sync::OnceLock;
use tracing::info;
use windows::core::s;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameA, GetProcAddress, LoadLibraryA};

struct WrappedFunctions {
    get_state: unsafe extern "system" fn(u32, *mut c_void) -> u32,
    set_state: unsafe extern "system" fn(u32, *mut c_void) -> u32,
}

fn get_wrapped_functions() -> &'static WrappedFunctions {
    static STATE: OnceLock<WrappedFunctions> = OnceLock::new();
    STATE.get_or_init(|| {
        info!("Initializing XInput proxy");
        // TODO: Error handling/reporting
        let external_dll = unsafe { LoadLibraryA(s!("C:\\windows\\system32\\XINPUT9_1_0.dll")) };
        let external_dll = match external_dll {
            Ok(h) => h,
            Err(e) => panic!("Failed to load XINPUT9_1_0.dll: {}", e),
        };

        let res = unsafe {
            WrappedFunctions {
                get_state: mem::transmute(
                    GetProcAddress(external_dll, s!("XInputGetState")).unwrap(),
                ),
                set_state: mem::transmute(
                    GetProcAddress(external_dll, s!("XInputSetState")).unwrap(),
                ),
            }
        };

        let mut fn_buffer = [0u8; 1024];
        unsafe { GetModuleFileNameA(external_dll, &mut fn_buffer) };
        let loaded_fn = CStr::from_bytes_until_nul(&fn_buffer).unwrap();
        info!(?loaded_fn, "XInput proxy init done");
        res
    })
}

#[no_mangle]
pub unsafe extern "system" fn XInputGetState(dwUserIndex: u32, pState: *mut c_void) -> u32 {
    (get_wrapped_functions().get_state)(dwUserIndex, pState)
}

#[no_mangle]
pub unsafe extern "system" fn XInputSetState(dwUserIndex: u32, pVibration: *mut c_void) -> u32 {
    (get_wrapped_functions().set_state)(dwUserIndex, pVibration)
}
