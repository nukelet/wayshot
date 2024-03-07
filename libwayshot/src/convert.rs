pub fn abgr8888_to_rgba8(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

pub fn argb8888_to_rgba8(data: &[u8]) -> Vec<u8> {
    data
        .chunks_exact(4)
        .map(|chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
        .flatten()
        .collect()
}

pub fn bgr888_to_rgb8(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

fn pixel_abgr2101010_to_rgba16(chunk: &[u8; 4]) -> [u16; 4] {
    let pixel = u32::from_le_bytes(chunk.clone());
    // Extract bits[31:30]
    let a2 = ((pixel & 0xC0000000) >> 30) as u32;
    // Extract bits[29:20]
    let b10 = ((pixel & 0x3FF00000) >> 20) as u32;
    // Extract bits[19:10]
    let g10 = ((pixel & 0x000FFC00) >> 10) as u32;
    // Extract bits[9:0]
    let r10 = (pixel & 0x000003FF) as u32;

    let mut converted: [u16; 4] = [0; 4];
    converted[0] = (a2 << 14) as u16;
    converted[1] = (r10 << 6) as u16;
    converted[2] = (g10 << 6) as u16;
    converted[3] = (b10 << 6) as u16;

    converted
}

pub fn abgr2101010_to_rgba16(data: &[u8]) -> Vec<u16> {
    data
        .chunks_exact(4)
        // SAFETY: This should never panic since we're always iterating
        // over &[u8; 4] chunks
        .map(|chunk| pixel_abgr2101010_to_rgba16(chunk.try_into().unwrap()))
        .flatten()
        .collect()
}
