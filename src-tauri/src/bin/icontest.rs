//! 图标提取独立验证程序
//!
//! 用途：不启动完整 GUI，直接验证 Win32 图标提取链路是否可用。
//! 运行：cargo run --bin icontest

use std::path::PathBuf;

#[cfg(windows)]
fn main() {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    // 测试目标
    let targets = [
        ("C:\\Windows\\System32\\notepad.exe", "记事本"),
        ("C:\\Windows\\System32\\calc.exe", "计算器"),
        ("C:\\Windows\\explorer.exe", "资源管理器"),
    ];

    let out_dir = PathBuf::from("artifacts/icon_test");
    std::fs::create_dir_all(&out_dir).expect("创建输出目录失败");

    for (path, label) in targets {
        println!("\n=== {label}: {path} ===");

        if !std::path::Path::new(path).exists() {
            println!("  ⚠️ 文件不存在，跳过");
            continue;
        }

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
            println!("  ❌ 图标提取失败 (ret={ret})");
            continue;
        }
        println!("  ✓ 取得 HICON: {:?}", info.hIcon);

        // 转 PNG
        let png = unsafe { hicon_to_png(info.hIcon) };

        match png {
            Ok(bytes) => {
                let out = out_dir.join(format!("{}.png", label));
                std::fs::write(&out, &bytes).expect("写入失败");
                println!("  ✓ PNG 已保存 ({len} 字节): {out:?}", len = bytes.len());

                // 校验 PNG magic
                if bytes.len() > 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
                    println!("  ✓ PNG 文件头校验通过");
                } else {
                    println!("  ⚠️ PNG 文件头异常");
                }
            }
            Err(e) => println!("  ❌ 转换失败: {e}"),
        }

        unsafe {
            let _ = DestroyIcon(info.hIcon);
        }
    }

    println!("\n=== 完成 ===");

    // ---- 内部函数 ----

    unsafe fn hicon_to_png(
        hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
    ) -> Result<Vec<u8>, String> {
        let mut ii = ICONINFO::default();
        GetIconInfo(hicon, &mut ii).map_err(|e| format!("GetIconInfo: {e}"))?;

        let hbm_color = ii.hbmColor;
        let hbm_mask = ii.hbmMask;

        let mut bm = BITMAP::default();
        if GetObjectW(
            HGDIOBJ(hbm_color.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        ) == 0
        {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return Err("GetObjectW 失败".into());
        }

        let (width, height) = (bm.bmWidth, bm.bmHeight);
        println!("  · 位图尺寸: {width}x{height}");

        if width <= 0 || height <= 0 {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return Err("尺寸无效".into());
        }

        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
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
            return Err("GetDIBits 失败".into());
        }

        // 检测是否有有效 alpha
        let has_alpha = pixels.chunks_exact(4).any(|px| px[3] != 0);
        println!("  · 含 alpha 通道: {has_alpha}");

        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2); // BGRA -> RGBA
            if !has_alpha {
                px[3] = 255;
            }
        }

        // 编码 PNG（手写最简实现，避免依赖）
        Ok(encode_png_minimal(&pixels, width as u32, height as u32)?)
    }

    /// 最简 PNG 编码（仅用于验证，产品代码用 image crate）
    fn encode_png_minimal(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
        // 使用 image crate（已在依赖中）
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(w, h, rgba.to_vec()).ok_or("构建缓冲失败")?;
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .map_err(|e| format!("PNG 编码: {e}"))?;
        Ok(out.into_inner())
    }

    use std::os::windows::ffi::OsStrExt;
}

#[cfg(not(windows))]
fn main() {
    println!("仅支持 Windows");
}
