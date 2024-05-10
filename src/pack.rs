use binrw::{prelude::*, Endian};

#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub struct OmvHeader {
    offset: u32,
    major_version: u8,
    minor_version: u8,
    padding: [u8; 2],
    // padding to 0x2c
    padding2: [u8; 0x24],
    pub metadata: OmvMetadata,
}

#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub struct OmvMetadata {
    pub width: u32,
    pub height: u32,
    frame_time: u32,
    stream_id: u32,
    stream_id2: u32,
    unknown: u32,
    data_pack_count: u32,
    frame_count: u32,
}
