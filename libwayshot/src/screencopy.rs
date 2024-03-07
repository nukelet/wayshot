use std::{
    ffi::CString,
    os::fd::{AsRawFd, IntoRawFd, OwnedFd},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, ImageBuffer};
use memmap2::MmapMut;
use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};
use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_output, wl_shm::Format, wl_shm_pool::WlShmPool,
};

use crate::{
    convert::{
        bgr888_to_rgb8,
        abgr8888_to_rgba8,
        argb8888_to_rgba8,
        abgr2101010_to_rgba16
    },
    region::{LogicalRegion, Size},
    Error, Result,
};

pub struct FrameGuard {
    pub buffer: WlBuffer,
    pub shm_pool: WlShmPool,
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.shm_pool.destroy();
    }
}

/// Type of frame supported by the compositor. For now we only support Argb8888, Xrgb8888, and
/// Xbgr8888.
///
/// See `zwlr_screencopy_frame_v1::Event::Buffer` as it's retrieved from there.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameFormat {
    pub format: Format,
    /// Size of the frame in pixels. This will always be in "landscape" so a
    /// portrait 1080x1920 frame will be 1920x1080 and will need to be rotated!
    pub size: Size,
    /// Stride is the number of bytes between the start of a row and the start of the next row.
    pub stride: u32,
}

impl FrameFormat {
    /// Returns the size of the frame in bytes, which is the stride * height.
    pub fn byte_size(&self) -> u64 {
        self.stride as u64 * self.size.height as u64
    }
}

/// The copied frame comprising of the FrameFormat, ColorType (Rgba8), and a memory backed shm
/// file that holds the image data in it.
#[derive(Debug)]
pub struct FrameCopy {
    pub frame_format: FrameFormat,
    pub frame_mmap: MmapMut,
    pub transform: wl_output::Transform,
    /// Logical region with the transform already applied.
    pub logical_region: LogicalRegion,
    pub physical_size: Size,
}

impl TryFrom<&FrameCopy> for DynamicImage {
    type Error = Error;

    fn try_from(value: &FrameCopy) -> Result<Self> {
        let format = value.frame_format.format;
        let width = value.frame_format.size.width;
        let height = value.frame_format.size.height;
        let data = &value.frame_mmap;
        match format {
            Format::Bgr888 => {
                let buf = bgr888_to_rgb8(data);
                let imgbuf = ImageBuffer::from_vec(width, height, buf)
                    .ok_or(Error::BufferTooSmall)?;
                Ok(Self::ImageRgb8(imgbuf))
            }
            Format::Xbgr8888 | Format::Abgr8888 => {
                let buf = abgr8888_to_rgba8(data);
                let imgbuf = ImageBuffer::from_vec(width, height, buf)
                    .ok_or(Error::BufferTooSmall)?;
                Ok(Self::ImageRgba8(imgbuf))
            }
            Format::Xrgb8888 | Format::Argb8888 => {
                let buf = argb8888_to_rgba8(data);
                let imgbuf = ImageBuffer::from_vec(width, height, buf)
                    .ok_or(Error::BufferTooSmall)?;
                Ok(Self::ImageRgba8(imgbuf))
            }
            Format::Xbgr2101010 | Format::Abgr2101010 => {
                let buf = abgr2101010_to_rgba16(data);
                let imgbuf = ImageBuffer::from_vec(width, height, buf)
                    .ok_or(Error::BufferTooSmall)?;
                Ok(Self::ImageRgba16(imgbuf))
            }
            _ => {
                tracing::error!("Unsupported buffer format: {:?}", format);
                tracing::error!("You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot.");
                Err(Error::NoSupportedBufferFormat)
            },
        }
    }
}

fn get_mem_file_handle() -> String {
    format!(
        "/libwayshot-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.subsec_nanos().to_string())
            .unwrap_or("unknown".into())
    )
}

/// Return a RawFd to a shm file. We use memfd create on linux and shm_open for BSD support.
/// You don't need to mess around with this function, it is only used by
/// capture_output_frame.
pub fn create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on succesful execution and seal it's operations.
        match memfd::memfd_create(
            CString::new("libwayshot")?.as_c_str(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd.as_raw_fd(),
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::ENOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let mut mem_file_handle = get_mem_file_handle();
    loop {
        match mman::shm_open(
            // O_CREAT = Create file if does not exist.
            // O_EXCL = Error if create and file exists.
            // O_RDWR = Open for reading and writing.
            // O_CLOEXEC = Close on succesful execution.
            // S_IRUSR = Set user read permission bit .
            // S_IWUSR = Set user write permission bit.
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd.into_raw_fd()) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(errno) => return Err(std::io::Error::from(errno)),
                },
            },
            Err(nix::errno::Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = get_mem_file_handle();
                continue;
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}
