//! 图标提取（Win32 实现）
//!
//! ⚠️ 重要（见 docs/04 §7）：
//! **不能**让前端直接用 `file:///` 引用本地 exe 图标 ——
//! WebView2 禁止跨协议加载本地文件。必须由 Rust 提取为 PNG，
//! 再由 `get_icon_data_url` 以 base64 data URL 提供给前端。
//!
//! 实现路径：
//! 1. `SHGetFileInfoW` 取图标句柄（含 .lnk 的目标图标）
//! 2. `GetIconInfo` 取位图
//! 3. 逐像素转 RGBA，写入 PNG

use std::path::Path;

use crate::config::schema::ItemType;
use crate::error::{AppError, Result};

/// 根据路径判断图标项类型
pub fn detect_item_type(path: &str) -> ItemType {
    let lower = path.to_ascii_lowercase();

    if lower.starts_with("http://") || lower.starts_with("https://") {
        return ItemType::Url;
    }

    let p = Path::new(path);
    if p.is_dir() {
        return ItemType::Folder;
    }

    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) if ext == "exe" || ext == "lnk" || ext == "bat" || ext == "cmd" => {
            ItemType::App
        }
        _ => ItemType::File,
    }
}

/// 由路径生成稳定的短哈希（用于图标缓存文件名）
///
/// 使用 FNV-1a 64 位，避免引入额外依赖。
pub fn short_hash(s: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// 生成图标缓存文件路径（相对 data 目录）
pub fn icon_cache_name(target: &str) -> String {
    format!("icons/{}.png", short_hash(target))
}

/// 解析 .lnk 快捷方式，返回 (目标路径, 参数, 工作目录)
#[cfg(windows)]
pub fn resolve_lnk(path: &str) -> Result<(String, String, String)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

    unsafe {
        // 初始化 COM（失败可能是因为已初始化，忽略）
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let need_uninit = hr.is_ok();

        let result = (|| -> Result<(String, String, String)> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| AppError::Icon(format!("创建 ShellLink 失败：{e}")))?;

            // 需要 IPersistFile 来加载 .lnk 文件
            use windows::Win32::System::Com::IPersistFile;
            let persist: IPersistFile = link
                .cast()
                .map_err(|e| AppError::Icon(format!("获取 IPersistFile 失败：{e}")))?;

            let wide: Vec<u16> = std::ffi::OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            persist
                .Load(PCWSTR(wide.as_ptr()), windows::Win32::System::Com::STGM_READ)
                .map_err(|e| AppError::Icon(format!("加载快捷方式失败：{e}")))?;

            // 目标路径
            // 注意：windows 0.58 的签名是 (&mut [u16], *mut WIN32_FIND_DATAW, u32)
            let mut buf = [0u16; 1024];
            link.GetPath(
                &mut buf,
                std::ptr::null_mut(),
                SLGP_RAWPATH.0 as u32,
            )
            .map_err(|e| AppError::Icon(format!("读取快捷方式目标失败：{e}")))?;

            let target = String::from_utf16_lossy(
                &buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)],
            );

            // 参数
            let mut abuf = [0u16; 1024];
            let _ = link.GetArguments(&mut abuf);
            let args = String::from_utf16_lossy(
                &abuf[..abuf.iter().position(|&c| c == 0).unwrap_or(0)],
            );

            // 工作目录
            let mut dbuf = [0u16; 1024];
            let _ = link.GetWorkingDirectory(&mut dbuf);
            let work_dir = String::from_utf16_lossy(
                &dbuf[..dbuf.iter().position(|&c| c == 0).unwrap_or(0)],
            );

            if target.is_empty() {
                return Err(AppError::Icon("快捷方式没有有效目标".into()));
            }

            Ok((target, args, work_dir))
        })();

        if need_uninit {
            CoUninitialize();
        }

        result
    }
}

#[cfg(not(windows))]
pub fn resolve_lnk(_path: &str) -> Result<(String, String, String)> {
    Err(AppError::NotImplemented("仅支持 Windows".into()))
}

/// 提取文件图标，返回 PNG 字节
///
/// 使用 `SHGetFileInfoW` + `SHGFI_ICON | SHGFI_LARGEICON`，
/// 对 .lnk 会自动取到目标程序的图标。
#[cfg(windows)]
pub fn extract_icon_png(path: &str) -> Result<Vec<u8>> {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    };

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut info = SHFILEINFOW::default();

    let ret = unsafe {
        SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    if ret == 0 || info.hIcon.is_invalid() {
        return Err(AppError::Icon(format!("无法提取图标：{path}")));
    }

    let png = hicon_to_png(info.hIcon);

    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(info.hIcon);
    }

    png
}

#[cfg(not(windows))]
pub fn extract_icon_png(_path: &str) -> Result<Vec<u8>> {
    Err(AppError::NotImplemented("仅支持 Windows".into()))
}

/// 把 HICON 转为 PNG 字节
#[cfg(windows)]
fn hicon_to_png(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Result<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    unsafe {
        let mut ii = ICONINFO::default();
        GetIconInfo(hicon, &mut ii)
            .map_err(|e| AppError::Icon(format!("GetIconInfo 失败：{e}")))?;

        let hbm_color = ii.hbmColor;
        let hbm_mask = ii.hbmMask;

        // 取位图尺寸
        let mut bm = BITMAP::default();
        if GetObjectW(
            HGDIOBJ(hbm_color.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        ) == 0
        {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return Err(AppError::Icon("读取位图信息失败".into()));
        }

        let width = bm.bmWidth;
        let height = bm.bmHeight;

        if width <= 0 || height <= 0 {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return Err(AppError::Icon("图标尺寸无效".into()));
        }

        // 准备 BITMAPINFO (32bpp RGBA)
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // 负值 = 自上而下
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let hdc: HDC = GetDC(None);
        let got = GetDIBits(
            hdc,
            hbm_color,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);

        let _ = DeleteObject(HGDIOBJ(hbm_color.0));
        let _ = DeleteObject(HGDIOBJ(hbm_mask.0));

        if got == 0 {
            return Err(AppError::Icon("读取图标像素失败".into()));
        }

        // BGRA → RGBA，并处理全透明像素（部分图标 alpha 全 0 但实际不透明）
        let mut has_alpha = false;
        for px in pixels.chunks_exact(4) {
            if px[3] != 0 {
                has_alpha = true;
                break;
            }
        }

        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2); // B <-> R
            if !has_alpha {
                px[3] = 255; // 无 alpha 通道时视为完全不透明
            }
        }

        // 编码为 PNG
        encode_png(&pixels, width as u32, height as u32)
    }
}

/// 把 RGBA 像素编码为 PNG
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    // 尝试使用 image crate（Tauri 已依赖）
    use image::{ImageBuffer, Rgba};

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width, height, rgba.to_vec())
            .ok_or_else(|| AppError::Icon("构建图像缓冲失败".into()))?;

    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| AppError::Icon(format!("PNG 编码失败：{e}")))?;

    Ok(out.into_inner())
}

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_types() {
        assert_eq!(detect_item_type("https://example.com"), ItemType::Url);
        assert_eq!(detect_item_type("http://a.b"), ItemType::Url);
        assert_eq!(detect_item_type("C:\\Windows\\notepad.exe"), ItemType::App);
        assert_eq!(detect_item_type("C:\\a\\b.lnk"), ItemType::App);
        assert_eq!(detect_item_type("C:\\a\\doc.txt"), ItemType::File);
        assert_eq!(detect_item_type("C:\\Windows"), ItemType::Folder);
    }

    #[test]
    fn hash_is_stable() {
        let a = short_hash("C:\\Windows\\notepad.exe");
        let b = short_hash("C:\\Windows\\notepad.exe");
        let c = short_hash("C:\\Windows\\calc.exe");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn cache_name_format() {
        let n = icon_cache_name("C:\\test.exe");
        assert!(n.starts_with("icons/"));
        assert!(n.ends_with(".png"));
    }
}
