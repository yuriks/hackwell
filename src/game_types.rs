#![allow(dead_code)] // Don't warn for unused constants in this module

use static_assertions::assert_eq_size;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct AssetInfo {
    pub flags: u8,
    pub unk1: [u8; 7],
    pub data: *const u8,
    pub size: u32,
    pub unk14: u32,
    pub unk18: u64,
    pub original_data: *const u8,
    pub unk28: u64,
}
assert_eq_size!(AssetInfo, [u8; 0x30]);

pub const ASSET_TYPE_MISC: u8 = 0x0;
pub const ASSET_TYPE_MAP: u8 = 0x1;
pub const ASSET_TYPE_PNG: u8 = 0x2;
pub const ASSET_TYPE_OGG: u8 = 0x3;
pub const ASSET_TYPE_SPRITEDATA: u8 = 0x5; // TODO: Not sure what people are calling this now
pub const ASSET_TYPE_DXBC: u8 = 0x7; // DirectX Shader bytecode
pub const ASSET_TYPE_BMF: u8 = 0x8; // Bitmap font: https://www.angelcode.com/products/bmfont/doc/file_format.html

pub const ASSET_TYPE_MASK: u8 = 0x3F;
pub const ASSET_FLAG_ENCRYPTED: u8 = 0x40;
pub const ASSET_FLAG_DECRYPTED: u8 = 0x80;
