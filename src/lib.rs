mod game_addrs;
mod game_types;
mod hooking;

use crate::game_addrs::{
    CODE_GetResourceData_ADDR, CODE_PlatformDecryptResource_ADDR, GameMemory, GetResourceDataFn,
    PlatformDecryptResourceFn,
};
use crate::game_types::{
    AssetInfo, ASSET_FLAG_ENCRYPTED, ASSET_TYPE_BMF, ASSET_TYPE_DXBC, ASSET_TYPE_MASK,
    ASSET_TYPE_MISC, ASSET_TYPE_OGG, ASSET_TYPE_PNG,
};
use crate::hooking::Trampoline;
use hudhook::{hooks::dx12::ImguiDx12Hooks, ImguiRenderLoop};
use imgui::{Condition, TableColumnSetup, Ui};
use minhook::MinHook;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::{ffi::c_void, fs, ptr, slice, str, thread};
use tracing::{debug_span, error, info, info_span};
use tracing_subscriber::fmt::format::FmtSpan;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

struct ModState {
    game_memory: GameMemory,
    asset_replacements: Vec<()>,
}

fn extension_for_asset_type(flags: u8) -> &'static str {
    match flags & ASSET_TYPE_MASK {
        ASSET_TYPE_MISC => "txt",
        ASSET_TYPE_PNG => "png",
        ASSET_TYPE_OGG => "ogg",
        ASSET_TYPE_DXBC => "dxbc",
        ASSET_TYPE_BMF => "bmf",
        _ => "bin",
    }
}

impl ModState {
    fn new() -> Self {
        ModState {
            game_memory: unsafe { GameMemory::from_process() },
            asset_replacements: Vec::new(),
        }
    }

    fn dump_assets(&self) -> std::io::Result<()> {
        let _span = info_span!("dumping assets").entered();

        let dump_dir = Path::new("dumped_assets");
        fs::create_dir_all(dump_dir)?;
        let asset_table = self.game_memory.asset_info_table();
        for i in 0..asset_table.len() {
            let Some(asset) = asset_table.get_asset_info(i) else {
                continue;
            };
            let file_ext = extension_for_asset_type(asset.flags);
            let filename = format!("{i:03}.{file_ext}");
            let mut file_path = dump_dir.join(filename);
            if asset.flags & ASSET_FLAG_ENCRYPTED != 0 {
                let mut pstr = file_path.into_os_string();
                pstr.push(".encrypted");
                file_path = pstr.into();
            }

            let contents = unsafe { slice::from_raw_parts(asset.data, asset.size as usize) };
            if let Err(err) = fs::write(&file_path, contents) {
                error!(?file_path, ?err);
            }
        }

        Ok(())
    }

    fn dump_single_asset(&self, asset_id: u32) -> std::io::Result<()> {
        let _span = debug_span!("dumping asset", asset_id).entered();

        let dump_dir = Path::new("dumped_assets");
        fs::create_dir_all(dump_dir)?;

        let asset_table = self.game_memory.asset_info_table();
        let Some(asset) = asset_table.get_asset_info(asset_id) else {
            return Ok(()); // Not sure about this one lol
        };
        let file_ext = extension_for_asset_type(asset.flags);
        let filename = format!("{asset_id:03}.{file_ext}");
        let mut file_path = dump_dir.join(filename);
        if asset.flags & ASSET_FLAG_ENCRYPTED != 0 {
            file_path.push(".encrypted");
        }

        let contents = unsafe { slice::from_raw_parts(asset.data, asset.size as usize) };
        if let Err(err) = fs::write(&file_path, contents) {
            error!(?file_path, ?err);
        }

        Ok(())
    }

    fn install_hooks(&self) {
        unsafe {
            TRAMPOLINE_GetResourceData.create_hook(
                self.game_memory.mut_ptr(CODE_GetResourceData_ADDR),
                GetResourceData_hook,
            );
            TRAMPOLINE_PlatformDecryptResource.create_hook(
                self.game_memory.mut_ptr(CODE_PlatformDecryptResource_ADDR),
                PlatformDecryptResource_hook,
            );
            MinHook::enable_all_hooks().unwrap();
        }
    }

    fn replace_assets(&self) -> std::io::Result<()> {
        let asset_table = self.game_memory.asset_info_table();

        let replacement_dir = Path::new("modded_assets");
        for entry in fs::read_dir(replacement_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Some(filename) = path.file_name().and_then(|p| p.to_str()) else {
                continue;
            };
            let id_str = filename
                .rsplit_once('.')
                .map(|(l, _)| l)
                .unwrap_or(filename);
            let Ok(asset_id) = id_str.parse::<u32>() else {
                continue;
            };

            info!(?path, asset_id, "found asset replacement");

            let Some(mut asset_info) = asset_table.get_asset_info(asset_id) else {
                continue;
            };

            let replacement_data = fs::read(path)?;

            asset_info.original_data = asset_info.data;
            asset_info.data = Box::into_raw(replacement_data.into_boxed_slice()) as *const u8;
            asset_info.flags &= ASSET_TYPE_MASK; // Clear any encryption flags

            unsafe {
                asset_table.replace_asset_info(asset_id, asset_info);
            }
        }

        Ok(())
    }
}

static TRAMPOLINE_GetResourceData: Trampoline<GetResourceDataFn> = Trampoline::new();
pub unsafe extern "C" fn GetResourceData_hook(asset_id: u32) -> AssetInfo {
    info!(asset_id, "asset loaded");
    TRAMPOLINE_GetResourceData.get()(asset_id)
}

static TRAMPOLINE_PlatformDecryptResource: Trampoline<PlatformDecryptResourceFn> =
    Trampoline::new();
pub unsafe extern "C" fn PlatformDecryptResource_hook(
    asset_id: u32,
    key: *const [u8; 16],
) -> *const AssetInfo {
    let key_contents: &[u8] = if key.is_null() { &[] } else { &*key };

    info!(asset_id, key_ptr = ?key, key = ?key_contents, "decrypting asset");
    let result = TRAMPOLINE_PlatformDecryptResource.get()(asset_id, key);

    let instance = get_mod_instance().lock().unwrap();
    if let Err(err) = instance.dump_single_asset(asset_id) {
        error!(asset_id, ?err, "failed to dump asset");
    }

    result
}

struct ModHud;

impl ImguiRenderLoop for ModHud {
    fn render(&mut self, ui: &mut Ui) {
        let instance = get_mod_instance().lock().unwrap();

        let asset_table = instance.game_memory.asset_info_table();

        ui.window("Hack Well")
            .size([320.0, 200.0], Condition::Once)
            .collapsed(true, Condition::Appearing)
            .position([0.0, 0.0], Condition::Appearing)
            .build(|| {
                ui.text("Asset List:");
                if let Some(_t) = ui.begin_table_header(
                    "assets-table",
                    [
                        TableColumnSetup::new("id"),
                        TableColumnSetup::new("type"),
                        TableColumnSetup::new("size"),
                        TableColumnSetup::new("magic"),
                    ],
                ) {
                    for i in 0..asset_table.len() {
                        let Some(asset) = asset_table.get_asset_info(i) else {
                            continue;
                        };

                        ui.table_next_column(); // id
                        ui.text(i.to_string());
                        ui.table_next_column(); // type
                        ui.text(format!("{:02X}", asset.flags));
                        ui.table_next_column(); // size
                        ui.text(format!("0x{:X}", asset.size));

                        ui.table_next_column(); // magic
                        if asset.size >= 4 {
                            let mut magic_bytes =
                                unsafe { ptr::read(asset.data as *const [u8; 4]) };
                            for c in &mut magic_bytes {
                                if !c.is_ascii_graphic() {
                                    *c = b'-';
                                }
                            }
                            ui.text(str::from_utf8(&magic_bytes).unwrap());
                        }
                    }
                }
            });
    }
}

fn get_mod_instance() -> &'static Mutex<ModState> {
    static INSTANCE: OnceLock<Mutex<ModState>> = OnceLock::new();
    INSTANCE.get_or_init(|| ModState::new().into())
}

#[no_mangle]
pub unsafe extern "stdcall" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) {
    if reason == DLL_PROCESS_ATTACH {
        let _ = hudhook::alloc_console();
        hudhook::enable_console_colors();
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .finish(),
        )
        .unwrap();
        info!("DllMain()");

        // Force initialization
        get_mod_instance();

        {
            let instance = get_mod_instance().lock().unwrap();
            if let Err(err) = instance.dump_assets() {
                error!(?err, "failed to dump assets");
            }
            instance.install_hooks();
            if let Err(err) = instance.replace_assets() {
                error!(?err, "failed to replace assets");
            }
        }

        thread::spawn(move || {
            if let Err(e) = ::hudhook::Hudhook::builder()
                .with::<ImguiDx12Hooks>(ModHud)
                .with_hmodule(hmodule)
                .build()
                .apply()
            {
                error!("Couldn't apply hooks: {e:?}");
                hudhook::eject();
            }
        });
    }
}
