//! Ayarları uygulayan, eski hali yedekleyen ve geri alan motor.
//!
//! Kural: bir ayar ilk kez uygulanırken dokunulacak her registry değerinin
//! o anki hali `%LOCALAPPDATA%\TrashWin\backup.json` dosyasına yazılır ve
//! dosya diske kaydedilmeden hiçbir değer değiştirilmez. Geri alma bu yedeği
//! birebir geri yükler; değer önceden yoksa siler, anahtarı biz açtıysak
//! boş kalan anahtarları da temizler.

use crate::apps::Pkg;
use crate::tweaks::{Cmd, Hive, Reg, Tweak, Val};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Applied,
    Partial,
    NotApplied,
    /// Komutla yapılan ayarların durumu registry'den okunamaz.
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum Old {
    Missing,
    Dword(u32),
    Sz(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Saved {
    key: String,
    old: Old,
    /// Uygulamadan önce var olmayan en üst anahtar; geri alırken boşsa silinir.
    created: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Backup(HashMap<String, Vec<Saved>>);

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("TrashWin")
}

impl Backup {
    fn path() -> PathBuf {
        data_dir().join("backup.json")
    }

    fn load() -> Result<Self, String> {
        match std::fs::read_to_string(Self::path()) {
            Ok(s) => serde_json::from_str(&s).map_err(|e| format!("backup.json okunamadı ({e}); dosyayı silmeden önce yedekle")),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("backup.json açılamadı: {e}")),
        }
    }

    /// Önce geçici dosyaya yaz, sonra yerine taşı: yarım yazılmış yedek kalmasın.
    fn save(&self) -> Result<(), String> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let tmp = dir.join("backup.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| format!("yedek yazılamadı: {e}"))?;
        std::fs::rename(&tmp, Self::path()).map_err(|e| format!("yedek kaydedilemedi: {e}"))
    }
}

fn root(h: Hive) -> RegKey {
    RegKey::predef(match h {
        Hive::Hklm => HKEY_LOCAL_MACHINE,
        Hive::Hkcu => HKEY_CURRENT_USER,
    })
}

fn key_id(r: &Reg) -> String {
    format!("{:?}\\{}\\{}", r.hive, r.path, r.name)
}

fn read(r: &Reg) -> Result<Old, String> {
    let key = match root(r.hive).open_subkey_with_flags(r.path, KEY_READ) {
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Old::Missing),
        Err(e) => return Err(format!("{r}: okunamadı: {e}")),
    };
    let raw = match key.get_raw_value(r.name) {
        Ok(v) => v,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Old::Missing),
        Err(e) => return Err(format!("{r}: okunamadı: {e}")),
    };
    let err = |e: std::io::Error| format!("{r}: okunamadı: {e}");
    match raw.vtype {
        RegType::REG_DWORD => key.get_value::<u32, _>(r.name).map(Old::Dword).map_err(err),
        RegType::REG_SZ => key.get_value::<String, _>(r.name).map(Old::Sz).map_err(err),
        // Bilmediğimiz tipi yedekleyemeyiz; üzerine yazıp geri alınamaz hale getirme.
        other => Err(format!("{r}: beklenmeyen tip {other:?}, dokunulmadı")),
    }
}

fn write(r: &Reg, v: &Old) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{r}: yazılamadı: {e}");
    match v {
        Old::Missing => match root(r.hive).open_subkey_with_flags(r.path, KEY_READ | winreg::enums::KEY_SET_VALUE) {
            Ok(k) => match k.delete_value(r.name) {
                Err(e) if e.kind() != ErrorKind::NotFound => Err(err(e)),
                _ => Ok(()),
            },
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(err(e)),
        },
        Old::Dword(d) => {
            let (k, _) = root(r.hive).create_subkey(r.path).map_err(err)?;
            k.set_value(r.name, d).map_err(err)
        }
        Old::Sz(s) => {
            let (k, _) = root(r.hive).create_subkey(r.path).map_err(err)?;
            k.set_value(r.name, s).map_err(err)
        }
    }
}

fn to_old(v: Val) -> Old {
    match v {
        Val::Dword(d) => Old::Dword(d),
        Val::Sz(s) => Old::Sz(s.to_string()),
    }
}

/// Yoldaki ilk var olmayan anahtar (biz oluşturacağız), hepsi varsa None.
fn first_missing(r: &Reg) -> Option<String> {
    let rk = root(r.hive);
    let mut cur = String::new();
    for part in r.path.split('\\') {
        if !cur.is_empty() {
            cur.push('\\');
        }
        cur.push_str(part);
        if rk.open_subkey_with_flags(&cur, KEY_READ).is_err() {
            return Some(cur);
        }
    }
    None
}

/// Bizim oluşturduğumuz anahtarları, içleri boşsa, yaprağından yukarı doğru siler.
fn prune(r: &Reg, created: &str) {
    let rk = root(r.hive);
    let mut cur = r.path.to_string();
    loop {
        let empty = match rk.open_subkey_with_flags(&cur, KEY_READ).and_then(|k| k.query_info()) {
            Ok(i) => i.sub_keys == 0 && i.values == 0,
            Err(_) => false,
        };
        if !empty || rk.delete_subkey(&cur).is_err() || cur.len() <= created.len() {
            return;
        }
        match cur.rfind('\\') {
            Some(i) => cur.truncate(i),
            None => return,
        }
    }
}

pub fn status(t: &Tweak) -> Status {
    if t.regs.is_empty() {
        return Status::Unknown;
    }
    let hits = t.regs.iter().filter(|r| read(r).ok() == Some(to_old(r.val))).count();
    match hits {
        0 => Status::NotApplied,
        n if n == t.regs.len() => Status::Applied,
        _ => Status::Partial,
    }
}

pub fn apply(t: &Tweak, log: &mut dyn FnMut(String)) -> Result<(), String> {
    if !t.regs.is_empty() {
        let mut store = Backup::load()?;
        // Yedek zaten varsa koru: ikinci uygulamada kendi değerimizi "eski" diye saklamayalım.
        if !store.0.contains_key(t.id) {
            let mut saved = Vec::with_capacity(t.regs.len());
            for r in t.regs {
                saved.push(Saved { key: key_id(r), old: read(r)?, created: first_missing(r) });
            }
            store.0.insert(t.id.to_string(), saved);
            store.save()?;
        }
        for r in t.regs {
            write(r, &to_old(r.val))?;
            log(format!("  ✔ {r}"));
        }
    }
    for c in t.apply {
        let out = run(c)?;
        log(format!("  ✔ {c}{}", tail(&out)));
    }
    Ok(())
}

pub fn revert(t: &Tweak, log: &mut dyn FnMut(String)) -> Result<(), String> {
    if !t.regs.is_empty() {
        let mut store = Backup::load()?;
        let saved = store.0.get(t.id).cloned().unwrap_or_default();
        for r in t.regs {
            match saved.iter().find(|s| s.key == key_id(r)) {
                Some(s) => {
                    write(r, &s.old)?;
                    if let (Old::Missing, Some(c)) = (&s.old, &s.created) {
                        prune(r, c);
                    }
                    log(format!("  ↩ {r}  (yedekten geri yüklendi)"));
                }
                None => {
                    let v = r.fallback.map(to_old).unwrap_or(Old::Missing);
                    write(r, &v)?;
                    log(format!("  ↩ {r}  (yedek yok, Windows varsayılanı)"));
                }
            }
        }
        store.0.remove(t.id);
        store.save()?;
    }
    for c in t.undo {
        let out = run(c)?;
        log(format!("  ↩ {c}{}", tail(&out)));
    }
    Ok(())
}

fn tail(out: &str) -> String {
    match out.lines().rev().find(|l| !l.trim().is_empty()) {
        Some(l) => format!("  — {}", l.trim()),
        None => String::new(),
    }
}

/// PowerShell betiğini -EncodedCommand ile verir: tırnak/kaçış sorunları olmaz.
fn ps_encoded(script: &str) -> String {
    let full = format!("$ProgressPreference='SilentlyContinue'; [Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}");
    let bytes: Vec<u8> = full.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    base64(&bytes)
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

pub fn run(c: &Cmd) -> Result<String, String> {
    let mut cmd = match c {
        Cmd::Run(exe, args) => {
            let mut x = Command::new(exe);
            x.args(*args);
            x
        }
        Cmd::Ps(script) => {
            let mut x = Command::new("powershell.exe");
            x.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &ps_encoded(script)]);
            x
        }
    };
    exec(&mut cmd, &c.to_string(), &[])
}

/// 3010 = başarılı, yeniden başlatma gerekli (DISM).
fn exec(cmd: &mut Command, label: &str, extra_ok: &[i32]) -> Result<String, String> {
    let out = cmd
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("{label}: başlatılamadı: {e}"))?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let code = out.status.code().unwrap_or(-1);
    if out.status.success() || code == 3010 || extra_ok.contains(&code) {
        Ok(text.trim().to_string())
    } else {
        Err(format!("{label}: çıkış kodu {code}{}", tail(&text)))
    }
}

// WinGet: "zaten kurulu" / "güncelleme yok" kodları başarısızlık sayılmaz.
const WINGET_ALREADY: &[i32] = &[0x8A15_002B_u32 as i32, 0x8A15_0061_u32 as i32];

pub fn winget_available() -> bool {
    Command::new("winget").arg("--version").creation_flags(CREATE_NO_WINDOW).output().is_ok_and(|o| o.status.success())
}

pub fn install(p: &Pkg) -> Result<String, String> {
    let mut cmd = Command::new("winget");
    cmd.args([
        "install",
        "--id",
        p.id,
        "--exact",
        "--source",
        p.source,
        "--silent",
        "--accept-source-agreements",
        "--accept-package-agreements",
        "--disable-interactivity",
    ]);
    exec(&mut cmd, p.name, WINGET_ALREADY)
}

pub struct Logger(Option<std::fs::File>);

impl Logger {
    pub fn open() -> Self {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(&dir);
        Logger(std::fs::OpenOptions::new().create(true).append(true).open(dir.join("trashwin.log")).ok())
    }
    pub fn line(&mut self, s: &str) {
        if let Some(f) = &mut self.0 {
            let _ = writeln!(f, "{s}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn backup_roundtrip() {
        let mut b = Backup::default();
        b.0.insert(
            "x".into(),
            vec![Saved { key: "k".into(), old: Old::Sz("ğüşı".into()), created: Some("a\\b".into()) }],
        );
        let s = serde_json::to_string(&b).unwrap();
        let back: Backup = serde_json::from_str(&s).unwrap();
        assert_eq!(back.0["x"][0].old, Old::Sz("ğüşı".into()));
    }
}
