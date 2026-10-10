//! 配置持久化
//!
//! 策略见 docs/04-数据模型与持久化.md §4：
//! - 便携优先（exe 目录 / data），失败回退 %APPDATA%
//! - 原子写入，防止崩溃损坏配置
//! - 防抖写入，避免频繁落盘
//! - 自动备份，保留最近 5 份

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::config::schema::{Config, CURRENT_VERSION};
use crate::error::{AppError, Result};

/// 备份保留数量
const MAX_BACKUPS: usize = 5;
/// 两次备份之间的最小间隔
const BACKUP_INTERVAL: Duration = Duration::from_secs(600);
/// 防抖写入延迟
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);

/// 配置存储
pub struct ConfigStore {
    inner: Arc<Mutex<Config>>,
    /// 数据目录（含 config.json、icons/ 等）
    data_dir: PathBuf,
    /// 是否有未落盘的修改
    dirty: Arc<AtomicBool>,
    /// 上次备份时间
    last_backup: Arc<Mutex<Instant>>,
    /// 防止并发写入
    write_lock: Arc<Mutex<()>>,
}

impl ConfigStore {
    /// 加载配置；文件不存在则创建默认配置
    pub fn load(_app: &AppHandle) -> Result<Self> {
        let data_dir = resolve_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("icons"))?;
        std::fs::create_dir_all(data_dir.join("backups"))?;

        let path = data_dir.join("config.json");

        let config = if path.exists() {
            match Self::read_from_disk(&path) {
                Ok(c) => c,
                Err(e) => {
                    // 配置损坏：备份损坏文件后使用默认配置
                    eprintln!("[warn] 配置读取失败（{e}），已备份并使用默认配置");
                    let broken = data_dir.join(format!(
                        "config.broken.{}.json",
                        chrono_like_stamp()
                    ));
                    let _ = std::fs::copy(&path, broken);
                    Config::default()
                }
            }
        } else {
            let cfg = Config::default();
            // 首次运行落盘
            let _ = Self::write_to_disk(&path, &cfg);
            cfg
        };

        let store = Self {
            inner: Arc::new(Mutex::new(config)),
            data_dir,
            dirty: Arc::new(AtomicBool::new(false)),
            last_backup: Arc::new(Mutex::new(Instant::now() - BACKUP_INTERVAL)),
            write_lock: Arc::new(Mutex::new(())),
        };

        // 启动防抖写入线程
        store.spawn_flusher();

        Ok(store)
    }

    /// 数据目录
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 读取配置副本
    pub fn get(&self) -> Config {
        self.inner.lock().unwrap().clone()
    }

    /// 全量替换配置并标脏
    pub fn set(&self, cfg: Config) {
        *self.inner.lock().unwrap() = cfg;
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// 以闭包方式修改配置并标脏
    pub fn mutate<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut Config) -> T,
    {
        let result = {
            let mut guard = self.inner.lock().unwrap();
            f(&mut guard)
        };
        self.dirty.store(true, Ordering::SeqCst);
        result
    }

    /// 标记为脏（外部已通过其他方式修改）
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// 立即落盘（忽略防抖）
    pub fn flush_now(&self) -> Result<()> {
        let _lock = self.write_lock.lock().unwrap();
        let cfg = self.inner.lock().unwrap().clone();
        let path = self.data_dir.join("config.json");

        self.maybe_backup(&path)?;
        Self::write_to_disk(&path, &cfg)?;
        self.dirty.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// 后台防抖写入线程
    fn spawn_flusher(&self) {
        let inner = self.inner.clone();
        let dirty = self.dirty.clone();
        let data_dir = self.data_dir.clone();
        let last_backup = self.last_backup.clone();
        let write_lock = self.write_lock.clone();

        std::thread::spawn(move || loop {
            std::thread::sleep(DEBOUNCE_DELAY);

            if !dirty.swap(false, Ordering::SeqCst) {
                continue;
            }

            let _lock = write_lock.lock().unwrap();
            let cfg = inner.lock().unwrap().clone();
            let path = data_dir.join("config.json");

            // 备份检查
            {
                let mut lb = last_backup.lock().unwrap();
                if lb.elapsed() >= BACKUP_INTERVAL {
                    let _ = backup_config(&path, &data_dir);
                    *lb = Instant::now();
                    dirty.store(false, Ordering::SeqCst);
                }
            }

            if let Err(e) = Self::write_to_disk(&path, &cfg) {
                eprintln!("[error] 配置写入失败：{e}");
                // 写失败则重新标脏，下次重试
                dirty.store(true, Ordering::SeqCst);
            }
        });
    }

    fn maybe_backup(&self, path: &Path) -> Result<()> {
        let mut lb = self.last_backup.lock().unwrap();
        if lb.elapsed() >= BACKUP_INTERVAL && path.exists() {
            backup_config(path, &self.data_dir)?;
            *lb = Instant::now();
        }
        Ok(())
    }

    /// 从磁盘读取
    fn read_from_disk(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)?;
        let mut value: serde_json::Value = serde_json::from_str(&text)?;

        migrate(&mut value)?;

        let cfg: Config = serde_json::from_value(value)?;
        Ok(cfg)
    }

    /// 原子写入磁盘
    fn write_to_disk(path: &Path, cfg: &Config) -> Result<()> {
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(cfg)?;

        // 1. 写临时文件
        std::fs::write(&tmp, json.as_bytes())?;

        // 2. 原子替换（Windows 上 rename 会覆盖已存在文件）
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(AppError::Config(format!("替换配置文件失败：{e}")));
        }

        Ok(())
    }
}

/// 解析数据目录：便携优先，回退 APPDATA
fn resolve_data_dir() -> Result<PathBuf> {
    // 1. 尝试 exe 同级的 data 目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("data");
            // 探测可写性
            if is_writable(dir) {
                return Ok(portable);
            }
        }
    }

    // 2. 回退 APPDATA
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Ok(PathBuf::from(appdata).join("AcrylicDesk"));
    }

    Err(AppError::Config("无法确定数据目录".into()))
}

/// 探测目录是否可写
fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".write_probe");
    match std::fs::write(&probe, b"1") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 备份配置，保留最近 MAX_BACKUPS 份
fn backup_config(path: &Path, data_dir: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let backup_dir = data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    let name = format!("config-{}.json", chrono_like_stamp());
    std::fs::copy(path, backup_dir.join(&name))?;

    // 清理旧备份
    let mut entries: Vec<_> = std::fs::read_dir(&backup_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config-")
        })
        .collect();

    if entries.len() > MAX_BACKUPS {
        entries.sort_by_key(|e| e.file_name());
        let remove_count = entries.len() - MAX_BACKUPS;
        for entry in entries.into_iter().take(remove_count) {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(())
}

/// 配置版本迁移（幂等）
fn migrate(value: &mut serde_json::Value) -> Result<()> {
    let ver = value
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if ver < 1 {
        // v0 → v1：初始版本，无需迁移
    }

    // 未来版本在此追加：
    // if ver < 2 { ... }

    value["version"] = serde_json::json!(CURRENT_VERSION);
    Ok(())
}

/// 简易时间戳 YYYYMMDD-HHMMSS（避免引入 chrono 依赖）
fn chrono_like_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // 简单的 UTC 时间换算（够用即可）
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // 从 1970-01-01 推算日期
    let mut year = 1970i64;
    let mut d = days as i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let ydays = if leap { 366 } else { 365 };
        if d < ydays {
            break;
        }
        d -= ydays;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while month < 12 && d >= mdays[month] as i64 {
        d -= mdays[month] as i64;
        month += 1;
    }

    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year,
        month + 1,
        d + 1,
        h,
        m,
        s
    )
}

/// 发出配置变更事件
pub fn emit_config_changed(app: &AppHandle, cfg: &Config) {
    let _ = app.emit("config://changed", cfg);
}

/// 便捷：获取全局配置存储
pub fn store_of(app: &AppHandle) -> Option<Arc<ConfigStore>> {
    app.try_state::<crate::AppState>()
        .map(|s| s.config.clone())
}
