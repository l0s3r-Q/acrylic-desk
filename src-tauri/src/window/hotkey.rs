//! 全局热键
//!
//! 用于在任意时刻强制唤出/收起面板（FR-3.7）。
//!
//! 实现说明：
//! Tauri 官方 `tauri-plugin-global-shortcut` 在 Windows 上通过
//! `RegisterHotKey` 实现，**不使用钩子**，因此不存在杀软误报风险
//! （对比见 docs/03 §3.1）。
//!
//! 本模块使用 Win32 `RegisterHotKey` 直接注册，由 Tauri 的窗口消息循环
//! 分发。原因：插件需要额外依赖，而我们的需求很简单（一个热键），
//! 直接调用更可控，也便于在热键被占用时给出准确提示。

use crate::error::{AppError, Result};

/// 已注册的热键 ID
const HOTKEY_ID: i32 = 0xA1C0;

/// 解析热键字符串，如 "Alt+`" → (modifiers, vk)
///
/// 支持的修饰键：Alt / Ctrl / Shift / Win
/// 支持的按键：A-Z, 0-9, F1-F12, `, -, =, [, ], \, ;, ', ,, ., /
pub fn parse_hotkey(spec: &str) -> Result<(u32, u32)> {
    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_SHIFT: u32 = 0x0004;
    const MOD_WIN: u32 = 0x0008;

    let parts: Vec<&str> = spec.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err(AppError::Invalid("热键不能为空".into()));
    }

    let mut mods = 0u32;
    let mut key: Option<u32> = None;

    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "alt" => mods |= MOD_ALT,
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "win" | "super" | "meta" => mods |= MOD_WIN,
            other => {
                if key.is_some() {
                    return Err(AppError::Invalid(format!(
                        "热键只能包含一个主键：{spec}"
                    )));
                }
                key = Some(parse_key(other)?);
            }
        }
    }

    let vk = key.ok_or_else(|| AppError::Invalid(format!("热键缺少主键：{spec}")))?;

    if mods == 0 {
        return Err(AppError::Invalid(
            "热键必须包含至少一个修饰键（Alt/Ctrl/Shift/Win），否则会拦截正常输入".into(),
        ));
    }

    Ok((mods, vk))
}

/// 解析主键为虚拟键码
fn parse_key(k: &str) -> Result<u32> {
    // 单字符
    if k.len() == 1 {
        let c = k.chars().next().unwrap();
        return match c {
            'a'..='z' => Ok(c.to_ascii_uppercase() as u32), // VK_A = 0x41
            '0'..='9' => Ok(c as u32),                      // VK_0 = 0x30
            '`' => Ok(0xC0),                                // VK_OEM_3
            '-' => Ok(0xBD),                                // VK_OEM_MINUS
            '=' => Ok(0xBB),                                // VK_OEM_PLUS
            '[' => Ok(0xDB),                                // VK_OEM_4
            ']' => Ok(0xDD),                                // VK_OEM_6
            '\\' => Ok(0xDC),                               // VK_OEM_5
            ';' => Ok(0xBA),                                // VK_OEM_1
            '\'' => Ok(0xDE),                               // VK_OEM_7
            ',' => Ok(0xBC),                                // VK_OEM_COMMA
            '.' => Ok(0xBE),                                // VK_OEM_PERIOD
            '/' => Ok(0xBF),                                // VK_OEM_2
            _ => Err(AppError::Invalid(format!("不支持的按键：{k}"))),
        };
    }

    // 功能键 F1-F24
    if let Some(num) = k.strip_prefix('f') {
        if let Ok(n) = num.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Ok(0x70 + n - 1); // VK_F1 = 0x70
            }
        }
    }

    // 具名键
    match k {
        "space" => Ok(0x20),
        "tab" => Ok(0x09),
        "enter" | "return" => Ok(0x0D),
        "esc" | "escape" => Ok(0x1B),
        "backspace" => Ok(0x08),
        "delete" | "del" => Ok(0x2E),
        "insert" | "ins" => Ok(0x2D),
        "home" => Ok(0x24),
        "end" => Ok(0x23),
        "pageup" => Ok(0x21),
        "pagedown" => Ok(0x22),
        "up" => Ok(0x26),
        "down" => Ok(0x28),
        "left" => Ok(0x25),
        "right" => Ok(0x27),
        _ => Err(AppError::Invalid(format!("不支持的按键：{k}"))),
    }
}

/// 注册全局热键
///
/// 失败时返回明确原因（热键被其他程序占用是最常见的情况）。
#[cfg(windows)]
pub fn register(spec: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, HOT_KEY_MODIFIERS,
    };

    let (mods, vk) = parse_hotkey(spec)?;

    unsafe {
        // hwnd = None 表示关联到当前线程的消息队列
        RegisterHotKey(None, HOTKEY_ID, HOT_KEY_MODIFIERS(mods), vk).map_err(|_| {
            AppError::Invalid(format!(
                "注册热键失败：{spec} 可能已被其他程序占用，请在设置中更换"
            ))
        })?;
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn register(_spec: &str) -> Result<()> {
    Err(AppError::NotImplemented("仅支持 Windows".into()))
}

/// 注销全局热键
#[cfg(windows)]
pub fn unregister() {
    use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;
    unsafe {
        let _ = UnregisterHotKey(None, HOTKEY_ID);
    }
}

#[cfg(not(windows))]
pub fn unregister() {}

/// 重新注册（配置变更时使用）
pub fn reregister(spec: &str) -> Result<()> {
    unregister();
    register(spec)
}

/// 热键是否合法（供设置界面即时校验）
pub fn validate(spec: &str) -> Result<()> {
    parse_hotkey(spec).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_alt_backtick() {
        let (mods, vk) = parse_hotkey("Alt+`").unwrap();
        assert_eq!(mods, 0x0001);
        assert_eq!(vk, 0xC0);
    }

    #[test]
    fn parse_ctrl_shift_letter() {
        let (mods, vk) = parse_hotkey("Ctrl+Shift+A").unwrap();
        assert_eq!(mods, 0x0002 | 0x0004);
        assert_eq!(vk, 0x41);
    }

    #[test]
    fn parse_win_key() {
        let (mods, vk) = parse_hotkey("Win+Space").unwrap();
        assert_eq!(mods, 0x0008);
        assert_eq!(vk, 0x20);
    }

    #[test]
    fn parse_function_key() {
        let (mods, vk) = parse_hotkey("Alt+F12").unwrap();
        assert_eq!(mods, 0x0001);
        assert_eq!(vk, 0x7B); // VK_F12 = 0x70 + 11
    }

    #[test]
    fn parse_named_keys() {
        assert_eq!(parse_hotkey("Ctrl+Up").unwrap().1, 0x26);
        assert_eq!(parse_hotkey("Ctrl+PageDown").unwrap().1, 0x22);
        assert_eq!(parse_hotkey("Ctrl+Delete").unwrap().1, 0x2E);
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(parse_hotkey("ALT+`").unwrap(), parse_hotkey("alt+`").unwrap());
        assert_eq!(parse_hotkey("CTRL+a").unwrap(), parse_hotkey("ctrl+A").unwrap());
    }

    #[test]
    fn reject_no_modifier() {
        // 无修饰键会拦截正常输入，必须拒绝
        assert!(parse_hotkey("A").is_err());
        assert!(parse_hotkey("F1").is_err());
    }

    #[test]
    fn reject_multiple_main_keys() {
        assert!(parse_hotkey("Ctrl+A+B").is_err());
    }

    #[test]
    fn reject_unknown_key() {
        assert!(parse_hotkey("Ctrl+不存在的键").is_err());
    }

    #[test]
    fn reject_empty() {
        assert!(parse_hotkey("").is_err());
    }

    #[test]
    fn validate_works() {
        assert!(validate("Alt+`").is_ok());
        assert!(validate("A").is_err());
    }
}
