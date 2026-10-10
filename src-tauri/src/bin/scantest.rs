//! 扫描与图标提取集成验证
//!
//! 用途：不启动 GUI，直接验证
//! 1. 桌面/开始菜单扫描是否返回正确结果
//! 2. 扫描到的项目能否成功提取图标
//!
//! 运行：cargo run --bin scantest

use std::path::PathBuf;

fn main() {
    println!("=== 扫描功能验证 ===\n");

    // ---- 1. 扫描桌面 ----
    println!("--- 桌面扫描 ---");
    match scan_desktop_simple() {
        Ok(items) => {
            println!("  找到 {} 项", items.len());
            for (i, (name, path)) in items.iter().take(8).enumerate() {
                println!("    {}. {name}", i + 1);
                println!("       {path}");
            }
            if items.len() > 8 {
                println!("    … 还有 {} 项", items.len() - 8);
            }

            // 对前 3 项提取图标验证
            println!("\n  --- 图标提取验证（前 3 项）---");
            for (name, path) in items.iter().take(3) {
                match extract_icon(path) {
                    Ok(bytes) => {
                        let valid = bytes.len() > 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n";
                        println!(
                            "    {} {} → {} 字节, PNG={}",
                            if valid { "✓" } else { "⚠" },
                            name,
                            bytes.len(),
                            valid
                        );
                    }
                    Err(e) => println!("    ✗ {name} → {e}"),
                }
            }
        }
        Err(e) => println!("  ✗ 扫描失败: {e}"),
    }

    // ---- 2. 扫描开始菜单 ----
    println!("\n--- 开始菜单扫描 ---");
    match scan_startmenu_simple() {
        Ok(items) => {
            println!("  找到 {} 项", items.len());
            for (i, (name, path)) in items.iter().take(6).enumerate() {
                println!("    {}. {name}", i + 1);
                println!("       {path}");
            }
            if items.len() > 6 {
                println!("    … 还有 {} 项", items.len() - 6);
            }
        }
        Err(e) => println!("  ✗ 扫描失败: {e}"),
    }

    println!("\n=== 完成 ===");
}

/// 简化版桌面扫描（与 icon/scan.rs 逻辑一致，独立实现便于验证）
fn scan_desktop_simple() -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();

    if let Ok(profile) = std::env::var("USERPROFILE") {
        collect(&PathBuf::from(&profile).join("Desktop"), &mut out);
    }
    if let Ok(public) = std::env::var("PUBLIC") {
        collect(&PathBuf::from(&public).join("Desktop"), &mut out);
    }

    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));
    Ok(out)
}

fn scan_startmenu_simple() -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        collect(
            &PathBuf::from(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
            &mut out,
        );
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        collect(
            &PathBuf::from(&pd)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
            &mut out,
        );
    }

    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));
    Ok(out)
}

fn collect(dir: &PathBuf, out: &mut Vec<(String, String)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            collect(&path, out);
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if ext != "lnk" && ext != "exe" && ext != "url" {
            continue;
        }

        let lower = name.to_lowercase();
        if lower.contains("uninstall") || name.contains("卸载") {
            continue;
        }

        let display = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());

        out.push((display, path.to_string_lossy().to_string()));
    }
}

#[cfg(windows)]
fn extract_icon(path: &str) -> Result<Vec<u8>, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

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
        return Err("无法提取图标".into());
    }

    let result = unsafe {
        let mut ii = ICONINFO::default();
        GetIconInfo(info.hIcon, &mut ii).map_err(|e| format!("GetIconInfo: {e}"))?;

        let mut bm = BITMAP::default();
        GetObjectW(
            HGDIOBJ(ii.hbmColor.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );

        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 {
            let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
            let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
            return Err("尺寸无效".into());
        }

        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let hdc: HDC = GetDC(None);
        let got = GetDIBits(
            hdc,
            ii.hbmColor,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);

        let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
        let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));

        if got == 0 {
            return Err("GetDIBits 失败".into());
        }

        let has_alpha = pixels.chunks_exact(4).any(|px| px[3] != 0);
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            if !has_alpha {
                px[3] = 255;
            }
        }

        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(w as u32, h as u32, pixels).ok_or("构建缓冲失败")?;
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .map_err(|e| format!("PNG 编码: {e}"))?;
        Ok(out.into_inner())
    };

    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }

    result
}

#[cfg(not(windows))]
fn extract_icon(_path: &str) -> Result<Vec<u8>, String> {
    Err("仅支持 Windows".into())
}
