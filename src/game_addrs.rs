use crate::game_types::AssetInfo;
use std::ffi::c_void;
use std::mem;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

const ASSET_INFO_TABLE_ADDR: usize = 0x20eb000;
const ASSET_INFO_TABLE_END: usize = 0x20f2ec0;

pub type GetResourceDataFn = extern "C" fn(u32) -> AssetInfo;
pub const CODE_GetResourceData_ADDR: usize = 0x00015d0;

pub type PlatformDecryptResourceFn =
    extern "C" fn(id: u32, key: *const [u8; 16]) -> *const AssetInfo;
pub const CODE_PlatformDecryptResource_ADDR: usize = 0x0001650;

pub struct GameMemory {
    module: HMODULE,
    module_base: *mut c_void,
}

impl GameMemory {
    pub unsafe fn from_process() -> Self {
        // TODO: Version check
        let module = unsafe { GetModuleHandleA(None) }.unwrap();
        GameMemory {
            module,
            module_base: module.0 as *mut c_void,
        }
    }

    pub unsafe fn ptr<T>(&self, addr: usize) -> *const T {
        self.module_base.wrapping_add(addr) as *const T
    }

    pub unsafe fn mut_ptr<T>(&self, addr: usize) -> *mut T {
        // TODO: Maybe assert addr is inside .data
        self.ptr::<T>(addr) as *mut T
    }

    pub fn asset_info_table(&self) -> AssetInfoTable {
        unsafe {
            AssetInfoTable {
                table: self.mut_ptr(ASSET_INFO_TABLE_ADDR),
                len: ((ASSET_INFO_TABLE_END - ASSET_INFO_TABLE_ADDR) / mem::size_of::<AssetInfo>())
                    .try_into()
                    .unwrap(),
            }
        }
    }
}

// TODO: Thread-safety is ???. Need to check how the game actually uses threads to figure out what
//       sort of synchronization I'm going to need.
unsafe impl Send for GameMemory {}

pub struct AssetInfoTable {
    table: *mut AssetInfo,
    len: u32,
}

impl AssetInfoTable {
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn get_asset_info(&self, asset_id: u32) -> Option<AssetInfo> {
        if asset_id < self.len {
            unsafe { Some(*self.table.add(asset_id as usize)) }
        } else {
            None
        }
    }

    pub unsafe fn replace_asset_info(&self, asset_id: u32, asset_info: AssetInfo) -> Option<()> {
        if asset_id < self.len {
            unsafe { *self.table.add(asset_id as usize) = asset_info; Some(()) }
        } else {
            None
        }
    }
}
