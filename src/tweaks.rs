//! Tüm ayarlar burada veri olarak durur. Yeni ayar = bu dosyaya bir kayıt.
//! Motor (`engine.rs`) registry değerlerinin eski halini kendisi yedekler;
//! burada yalnızca hedef değer ve yedek yoksa kullanılacak `fallback` yazılır.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hive {
    Hklm,
    Hkcu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Val {
    Dword(u32),
    Sz(&'static str),
}

/// Tek bir registry değeri. `fallback`: yedek bulunamazsa geri alırken yazılacak
/// değer. `None` ise değer silinir (Windows varsayılanına döner).
pub struct Reg {
    pub hive: Hive,
    pub path: &'static str,
    pub name: &'static str,
    pub val: Val,
    pub fallback: Option<Val>,
}

pub enum Cmd {
    Run(&'static str, &'static [&'static str]),
    Ps(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Safe,
    Caution,
    Risky,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    Privacy,
    Performance,
    Gaming,
    Network,
    Explorer,
    Maintenance,
}

pub struct Tweak {
    pub id: &'static str,
    pub cat: Cat,
    pub name: &'static str,
    pub desc: &'static str,
    pub risk: Risk,
    pub reboot: bool,
    pub regs: &'static [Reg],
    pub apply: &'static [Cmd],
    pub undo: &'static [Cmd],
}

impl Tweak {
    /// Ne registry ne de geri alma komutu varsa tek seferlik bir işlemdir.
    pub fn undoable(&self) -> bool {
        !self.regs.is_empty() || !self.undo.is_empty()
    }
}

impl Cat {
    pub const ALL: [Cat; 6] = [
        Cat::Privacy,
        Cat::Performance,
        Cat::Gaming,
        Cat::Network,
        Cat::Explorer,
        Cat::Maintenance,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Cat::Privacy => "Gizlilik",
            Cat::Performance => "Performans",
            Cat::Gaming => "Oyun",
            Cat::Network => "Ağ",
            Cat::Explorer => "Görünüm & Gezgin",
            Cat::Maintenance => "Bakım",
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Val::Dword(v) => write!(f, "DWORD {v}"),
            Val::Sz(s) => write!(f, "SZ \"{s}\""),
        }
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hive = match self.hive {
            Hive::Hklm => "HKLM",
            Hive::Hkcu => "HKCU",
        };
        let name = if self.name.is_empty() { "(Varsayılan)" } else { self.name };
        write!(f, "{hive}\\{}  →  {name} = {}", self.path, self.val)
    }
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cmd::Run(exe, args) => write!(f, "{exe} {}", args.join(" ")),
            Cmd::Ps(s) => write!(f, "PowerShell: {s}"),
        }
    }
}

const fn dw(hive: Hive, path: &'static str, name: &'static str, v: u32) -> Reg {
    Reg { hive, path, name, val: Val::Dword(v), fallback: None }
}

const fn sz(hive: Hive, path: &'static str, name: &'static str, v: &'static str, def: &'static str) -> Reg {
    Reg { hive, path, name, val: Val::Sz(v), fallback: Some(Val::Sz(def)) }
}

/// Servis başlangıç tipi. 4 = devre dışı. `def` = Windows'un varsayılanı;
/// Start değeri silinirse servis bozulur, bu yüzden fallback zorunlu.
const fn svc(path: &'static str, start: u32, def: u32) -> Reg {
    Reg { hive: Hive::Hklm, path, name: "Start", val: Val::Dword(start), fallback: Some(Val::Dword(def)) }
}

use Hive::{Hkcu, Hklm};

const ADV: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const GCS: &str = r"System\GameConfigStore";
const MOUSE: &str = r"Control Panel\Mouse";

pub static TWEAKS: &[Tweak] = &[
    // ---------------- Gizlilik ----------------
    Tweak {
        id: "telemetry",
        cat: Cat::Privacy,
        name: "Telemetriyi en aza indir",
        desc: "Tanılama verisi ilkesini 0'a çeker, DiagTrack ve dmwappushservice servislerini kapatır. Home sürümünde Windows en düşük seviyeyi 'Gerekli' olarak uygular.",
        risk: Risk::Safe,
        reboot: true,
        regs: &[
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", 0),
            svc(r"SYSTEM\CurrentControlSet\Services\DiagTrack", 4, 2),
            svc(r"SYSTEM\CurrentControlSet\Services\dmwappushservice", 4, 3),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "adid",
        cat: Cat::Privacy,
        name: "Reklam kimliğini kapat",
        desc: "Uygulamaların kişiselleştirilmiş reklam için kullandığı reklam kimliğini kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", 0),
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo", "DisabledByGroupPolicy", 1),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "bing",
        cat: Cat::Privacy,
        name: "Başlat menüsünde web aramasını kapat",
        desc: "Başlat aramasına yazılanların Bing'e gönderilmesini ve web önerilerini kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Search", "BingSearchEnabled", 0),
            dw(Hkcu, r"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", 1),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "suggestions",
        cat: Cat::Privacy,
        name: "Öneri ve sessiz uygulama kurulumlarını kapat",
        desc: "Başlat menüsü önerilerini, kilit ekranı ipuçlarını ve Windows'un kendiliğinden 'önerilen' uygulama kurmasını kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hkcu, CDM, "SystemPaneSuggestionsEnabled", 0),
            dw(Hkcu, CDM, "SilentInstalledAppsEnabled", 0),
            dw(Hkcu, CDM, "SubscribedContent-338388Enabled", 0),
            dw(Hkcu, CDM, "SubscribedContent-338389Enabled", 0),
            dw(Hkcu, CDM, "SoftLandingEnabled", 0),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "activity",
        cat: Cat::Privacy,
        name: "Etkinlik geçmişini kapat",
        desc: "Etkinlik geçmişinin toplanmasını ve buluta yüklenmesini engeller.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\System", "EnableActivityFeed", 0),
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\System", "PublishUserActivities", 0),
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\System", "UploadUserActivities", 0),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "copilot",
        cat: Cat::Privacy,
        name: "Copilot ve Recall'u kapat",
        desc: "Windows Copilot'u ilkeyle kapatır, Recall'un ekran görüntüsü kaydetmesini engeller. Desteklemeyen sürümlerde etkisizdir.",
        risk: Risk::Safe,
        reboot: true,
        regs: &[
            dw(Hkcu, r"Software\Policies\Microsoft\Windows\WindowsCopilot", "TurnOffWindowsCopilot", 1),
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI", "DisableAIDataAnalysis", 1),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "cortana",
        cat: Cat::Privacy,
        name: "Cortana'yı kapat",
        desc: "Cortana'yı grup ilkesiyle kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "AllowCortana", 0)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "wer",
        cat: Cat::Privacy,
        name: "Hata bildirimini (WER) kapat",
        desc: "Çökme raporlarının Microsoft'a gönderilmesini kapatır. Sorun giderirken işini zorlaştırabilir.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\Windows Error Reporting", "Disabled", 1)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "location",
        cat: Cat::Privacy,
        name: "Konum servislerini kapat",
        desc: "Konumu sistem genelinde kapatır. Hava durumu, Bul (Find My Device) ve harita uygulamaları çalışmaz.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\LocationAndSensors", "DisableLocation", 1)],
        apply: &[],
        undo: &[],
    },
    // ---------------- Performans ----------------
    Tweak {
        id: "ultimate",
        cat: Cat::Performance,
        name: "Nihai Performans güç planı",
        desc: "Gizli 'Ultimate Performance' planını bir kez oluşturur ve etkinleştirir. Dizüstünde pil ömrünü kısaltır. Geri alma Dengeli plana döner.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Ps(
            "$t='a760cf1b-247a-4793-bc2f-c6c0cff210b1'; if (-not ((powercfg /list | Out-String) -match $t)) { powercfg /duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61 $t | Out-Null }; powercfg /setactive $t; exit $LASTEXITCODE",
        )],
        undo: &[Cmd::Run("powercfg", &["/setactive", "381b4222-f694-41f0-9685-ff5bb260df2e"])],
    },
    Tweak {
        id: "sysmain",
        cat: Cat::Performance,
        name: "SysMain (Superfetch) kapat",
        desc: "SSD'li sistemlerde gereksiz disk/RAM kullanımını azaltabilir. HDD'de açılış yavaşlayabilir.",
        risk: Risk::Caution,
        reboot: true,
        regs: &[svc(r"SYSTEM\CurrentControlSet\Services\SysMain", 4, 2)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "hibernate",
        cat: Cat::Performance,
        name: "Hazırda bekletmeyi kapat",
        desc: "hiberfil.sys dosyasını siler (RAM kadar yer açar). Hızlı Başlatma da kapanır.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Run("powercfg", &["/hibernate", "off"])],
        undo: &[Cmd::Run("powercfg", &["/hibernate", "on"])],
    },
    Tweak {
        id: "wu_restart",
        cat: Cat::Performance,
        name: "Güncelleme sonrası zorla yeniden başlatmayı engelle",
        desc: "Oturum açıkken Windows Update'in bilgisayarı kendiliğinden yeniden başlatmasını engeller. Güncellemeler yine iner.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU", "NoAutoRebootWithLoggedOnUsers", 1)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "wu_off",
        cat: Cat::Performance,
        name: "Windows Update'i kapat",
        desc: "Otomatik güncellemeyi ilkeyle ve servisle kapatır. Güvenlik yamaları gelmez; yalnızca geçici kullan ve geri al.",
        risk: Risk::Risky,
        reboot: false,
        regs: &[
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU", "NoAutoUpdate", 1),
            svc(r"SYSTEM\CurrentControlSet\Services\wuauserv", 4, 3),
        ],
        apply: &[Cmd::Ps("Stop-Service wuauserv -Force -ErrorAction SilentlyContinue")],
        undo: &[],
    },
    Tweak {
        id: "vbs",
        cat: Cat::Performance,
        name: "VBS / Bellek Bütünlüğü ve hypervisor'u kapat",
        desc: "Bazı oyunlarda FPS artırabilir. WSL2, Hyper-V, Sandbox, Android alt sistemi çalışmaz ve güvenlik zayıflar. Bazı anti-cheat'ler (Vanguard, FACEIT) VBS/HVCI isteyebilir.",
        risk: Risk::Risky,
        reboot: true,
        regs: &[
            dw(Hklm, r"SYSTEM\CurrentControlSet\Control\DeviceGuard", "EnableVirtualizationBasedSecurity", 0),
            dw(Hklm, r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity", "Enabled", 0),
        ],
        apply: &[Cmd::Run("bcdedit", &["/set", "hypervisorlaunchtype", "off"])],
        undo: &[Cmd::Run("bcdedit", &["/set", "hypervisorlaunchtype", "auto"])],
    },
    // ---------------- Oyun ----------------
    Tweak {
        id: "gamedvr",
        cat: Cat::Gaming,
        name: "Xbox Game DVR kaydını kapat",
        desc: "Arka planda oyun kaydını kapatır. Win+G ile kayıt alınamaz.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hkcu, GCS, "GameDVR_Enabled", 0),
            dw(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\GameDVR", "AllowGameDVR", 0),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "gamemode",
        cat: Cat::Gaming,
        name: "Oyun Modunu aç",
        desc: "Oyun açıkken Windows Update ve arka plan işlerini kısar.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled", 1)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "fso",
        cat: Cat::Gaming,
        name: "Tam ekran optimizasyonlarını kapat",
        desc: "Gerçek özel tam ekran kullanan eski oyunlarda gecikmeyi azaltabilir. Yeni oyunlarda fark olmayabilir.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            dw(Hkcu, GCS, "GameDVR_FSEBehaviorMode", 2),
            dw(Hkcu, GCS, "GameDVR_HonorUserFSEBehaviorMode", 1),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "hags",
        cat: Cat::Gaming,
        name: "Donanım hızlandırmalı GPU zamanlamasını aç",
        desc: "Destekleyen GPU ve sürücülerde (GTX 1000+/RX 5000+) gecikmeyi azaltabilir. Yeniden başlatma gerekir.",
        risk: Risk::Caution,
        reboot: true,
        regs: &[dw(Hklm, r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", 2)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "mouse",
        cat: Cat::Gaming,
        name: "Fare ivmesini kapat",
        desc: "'İşaretçi hassasiyetini artır'ı kapatır; nişan almada tutarlılık sağlar. Oturum yeniden açılınca tam uygulanır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            sz(Hkcu, MOUSE, "MouseSpeed", "0", "1"),
            sz(Hkcu, MOUSE, "MouseThreshold1", "0", "6"),
            sz(Hkcu, MOUSE, "MouseThreshold2", "0", "10"),
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "sticky",
        cat: Cat::Gaming,
        name: "Yapışkan tuş kısayollarını kapat",
        desc: "Oyunda 5 kez Shift'e basınca açılan Yapışkan/Filtre/Geçiş Tuşları pencerelerini kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            sz(Hkcu, r"Control Panel\Accessibility\StickyKeys", "Flags", "506", "510"),
            sz(Hkcu, r"Control Panel\Accessibility\Keyboard Response", "Flags", "122", "126"),
            sz(Hkcu, r"Control Panel\Accessibility\ToggleKeys", "Flags", "58", "62"),
        ],
        apply: &[],
        undo: &[],
    },
    // ---------------- Ağ ----------------
    Tweak {
        id: "netthrottle",
        cat: Cat::Network,
        name: "Ağ kısıtlamasını kaldır",
        desc: "Multimedya oynatılırken ağ paketlerinin kısılmasını kapatır (NetworkThrottlingIndex).",
        risk: Risk::Safe,
        reboot: true,
        regs: &[dw(
            Hklm,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
            "NetworkThrottlingIndex",
            0xFFFF_FFFF,
        )],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "dns",
        cat: Cat::Network,
        name: "Cloudflare DNS (IPv4 + IPv6)",
        desc: "Bağlı fiziksel adaptörlerde DNS'i 1.1.1.1 / 1.0.0.1 ve IPv6 karşılıklarına çeker. Geri alma DHCP'ye (otomatik) döner; önceki elle girilmiş DNS saklanmaz.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Ps(
            "Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | Set-DnsClientServerAddress -ServerAddresses '1.1.1.1','1.0.0.1','2606:4700:4700::1111','2606:4700:4700::1001'; Clear-DnsClientCache",
        )],
        undo: &[Cmd::Ps(
            "Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | Set-DnsClientServerAddress -ResetServerAddresses; Clear-DnsClientCache",
        )],
    },
    Tweak {
        id: "smb1",
        cat: Cat::Network,
        name: "SMBv1'i kapat",
        desc: "WannaCry'ın kullandığı eski dosya paylaşım protokolünü kaldırır. Çok eski NAS/yazıcılar erişilemez olabilir.",
        risk: Risk::Safe,
        reboot: true,
        regs: &[],
        apply: &[Cmd::Run("dism", &["/online", "/Disable-Feature", "/FeatureName:SMB1Protocol", "/Quiet", "/NoRestart"])],
        undo: &[Cmd::Run("dism", &["/online", "/Enable-Feature", "/FeatureName:SMB1Protocol", "/All", "/Quiet", "/NoRestart"])],
    },
    Tweak {
        id: "flushdns",
        cat: Cat::Network,
        name: "DNS önbelleğini temizle",
        desc: "Tek seferlik: ipconfig /flushdns.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Run("ipconfig", &["/flushdns"])],
        undo: &[],
    },
    // ---------------- Görünüm & Gezgin ----------------
    Tweak {
        id: "classicmenu",
        cat: Cat::Explorer,
        name: "Windows 11 klasik sağ tık menüsü",
        desc: "'Daha fazla seçenek göster' adımını kaldırır. Gezgin yeniden başlayınca görünür.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[Reg {
            hive: Hkcu,
            path: r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32",
            name: "",
            val: Val::Sz(""),
            fallback: None,
        }],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "fileext",
        cat: Cat::Explorer,
        name: "Dosya uzantılarını ve gizli dosyaları göster",
        desc: "'belge.pdf.exe' gibi sahte dosyaları fark etmeyi kolaylaştırır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[
            Reg { hive: Hkcu, path: ADV, name: "HideFileExt", val: Val::Dword(0), fallback: Some(Val::Dword(1)) },
            Reg { hive: Hkcu, path: ADV, name: "Hidden", val: Val::Dword(1), fallback: Some(Val::Dword(2)) },
        ],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "taskbarleft",
        cat: Cat::Explorer,
        name: "Görev çubuğunu sola hizala",
        desc: "Windows 11 görev çubuğu simgelerini Windows 10'daki gibi sola alır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hkcu, ADV, "TaskbarAl", 0)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "widgets",
        cat: Cat::Explorer,
        name: "Pencere öğelerini (Widgets/Haberler) kapat",
        desc: "Görev çubuğundaki haber/hava durumu panelini ilkeyle kapatır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hklm, r"SOFTWARE\Policies\Microsoft\Dsh", "AllowNewsAndInterests", 0)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "endtask",
        cat: Cat::Explorer,
        name: "Görev çubuğunda 'Görevi sonlandır'",
        desc: "Görev çubuğunda sağ tık menüsüne donan programı kapatan 'Görevi sonlandır' ekler (Windows 11 23H2+).",
        risk: Risk::Safe,
        reboot: false,
        regs: &[dw(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\TaskbarDeveloperSettings", "TaskbarEndTask", 1)],
        apply: &[],
        undo: &[],
    },
    Tweak {
        id: "restartexplorer",
        cat: Cat::Explorer,
        name: "Gezgini yeniden başlat",
        desc: "Tek seferlik: explorer.exe kapatılır, Windows onu normal kullanıcı yetkisiyle kendisi yeniden açar.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Run("taskkill", &["/f", "/im", "explorer.exe"])],
        undo: &[],
    },
    // ---------------- Bakım ----------------
    Tweak {
        id: "restorepoint",
        cat: Cat::Maintenance,
        name: "Geri yükleme noktası oluştur",
        desc: "Tek seferlik: C: için Sistem Korumasını açar ve nokta oluşturur. Windows 24 saatte bir noktaya izin verir.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Ps(RESTORE_POINT)],
        undo: &[],
    },
    Tweak {
        id: "cleantemp",
        cat: Cat::Maintenance,
        name: "Geçici dosyaları temizle",
        desc: "Tek seferlik: kullanıcı TEMP ve Windows\\Temp içeriğini siler. Kullanımdaki dosyalar atlanır.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Ps(
            "$b=0; foreach ($d in @($env:TEMP, \"$env:SystemRoot\\Temp\")) { Get-ChildItem -LiteralPath $d -Force -ErrorAction SilentlyContinue | ForEach-Object { $s=(Get-ChildItem -LiteralPath $_.FullName -Recurse -Force -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum + ($_.Length -as [long]); Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue; if (-not (Test-Path -LiteralPath $_.FullName)) { $b += $s } } }; \"{0:N1} MB temizlendi\" -f ($b/1MB)",
        )],
        undo: &[],
    },
    Tweak {
        id: "bloat",
        cat: Cat::Maintenance,
        name: "Gereksiz hazır uygulamaları kaldır",
        desc: "Bing Haber/Hava Durumu, Solitaire, Kişiler, Başlarken, Geri Bildirim Merkezi, Office Hub ve Clipchamp'i mevcut kullanıcıdan kaldırır. Medya Oynatıcı, Fotoğraflar ve Hesap Makinesi'ne dokunmaz. Geri getirmek için Microsoft Store kullanılır.",
        risk: Risk::Caution,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Ps(
            "$p='Microsoft.BingNews','Microsoft.BingWeather','Microsoft.MicrosoftSolitaireCollection','Microsoft.People','Microsoft.Getstarted','Microsoft.WindowsFeedbackHub','Microsoft.MicrosoftOfficeHub','Clipchamp.Clipchamp'; foreach ($n in $p) { Get-AppxPackage -Name $n | ForEach-Object { Remove-AppxPackage -Package $_.PackageFullName -ErrorAction SilentlyContinue; \"kaldırıldı: $n\" } }",
        )],
        undo: &[],
    },
    Tweak {
        id: "repair",
        cat: Cat::Maintenance,
        name: "Sistem dosyalarını onar (DISM + SFC)",
        desc: "Tek seferlik: DISM RestoreHealth ve sfc /scannow. 10-30 dakika sürebilir, internet gerekir.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[
            Cmd::Run("dism", &["/online", "/Cleanup-Image", "/RestoreHealth"]),
            Cmd::Run("sfc", &["/scannow"]),
        ],
        undo: &[],
    },
    Tweak {
        id: "netfx3",
        cat: Cat::Maintenance,
        name: ".NET Framework 3.5",
        desc: "Eski oyun ve programların istediği .NET 3.5'i Windows Update üzerinden kurar.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Run("dism", &["/online", "/Enable-Feature", "/FeatureName:NetFx3", "/All", "/Quiet", "/NoRestart"])],
        undo: &[Cmd::Run("dism", &["/online", "/Disable-Feature", "/FeatureName:NetFx3", "/Quiet", "/NoRestart"])],
    },
    Tweak {
        id: "directplay",
        cat: Cat::Maintenance,
        name: "DirectPlay (eski oyunlar)",
        desc: "2000'ler dönemi oyunlarının istediği DirectPlay bileşenini açar.",
        risk: Risk::Safe,
        reboot: false,
        regs: &[],
        apply: &[Cmd::Run("dism", &["/online", "/Enable-Feature", "/FeatureName:DirectPlay", "/All", "/Quiet", "/NoRestart"])],
        undo: &[Cmd::Run("dism", &["/online", "/Disable-Feature", "/FeatureName:DirectPlay", "/Quiet", "/NoRestart"])],
    },
];

pub const RESTORE_POINT: &str = "Enable-ComputerRestore -Drive \"$env:SystemDrive\\\" -ErrorAction SilentlyContinue; Checkpoint-Computer -Description 'TrashWin' -RestorePointType MODIFY_SETTINGS -ErrorAction Stop; 'Geri yükleme noktası hazır'";


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_unique() {
        let mut seen = HashSet::new();
        for t in TWEAKS {
            assert!(seen.insert(t.id), "tekrar eden id: {}", t.id);
        }
    }

    #[test]
    fn service_start_has_fallback() {
        // Start değeri silinirse servis açılmaz; yedek kaybolsa bile geri yazılacak değer şart.
        for t in TWEAKS {
            for r in t.regs {
                if r.path.contains(r"\Services\") {
                    assert!(r.fallback.is_some(), "{}: {} fallback yok", t.id, r.path);
                }
            }
        }
    }

    #[test]
    fn paths_are_relative() {
        for t in TWEAKS {
            for r in t.regs {
                assert!(!r.path.starts_with('\\') && !r.path.ends_with('\\'), "{}: {}", t.id, r.path);
            }
        }
    }

    #[test]
    fn every_tweak_does_something() {
        for t in TWEAKS {
            assert!(!t.regs.is_empty() || !t.apply.is_empty(), "{} boş", t.id);
        }
    }
}
