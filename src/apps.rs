//! WinGet ile kurulabilen uygulamalar. Kimlikler sabit; kullanıcı girdisi komuta eklenmez.

pub struct Pkg {
    pub name: &'static str,
    pub id: &'static str,
    /// "winget" veya "msstore"
    pub source: &'static str,
    pub group: &'static str,
}

const fn w(group: &'static str, name: &'static str, id: &'static str) -> Pkg {
    Pkg { name, id, source: "winget", group }
}

const fn s(group: &'static str, name: &'static str, id: &'static str) -> Pkg {
    Pkg { name, id, source: "msstore", group }
}

pub static PKGS: &[Pkg] = &[
    w("Tarayıcı", "Firefox", "Mozilla.Firefox"),
    w("Tarayıcı", "Brave", "Brave.Brave"),
    w("Tarayıcı", "Google Chrome", "Google.Chrome"),
    w("Arşiv", "7-Zip", "7zip.7zip"),
    w("Arşiv", "NanaZip", "M2Team.NanaZip"),
    w("Arşiv", "WinRAR", "RARLab.WinRAR"),
    w("Araçlar", "Everything", "voidtools.Everything"),
    w("Araçlar", "Notepad++", "Notepad++.Notepad++"),
    w("Araçlar", "PowerToys", "Microsoft.PowerToys"),
    w("Araçlar", "ShareX", "ShareX.ShareX"),
    w("Araçlar", "SumatraPDF", "SumatraPDF.SumatraPDF"),
    w("Araçlar", "ImageGlass", "DuongDieuPhap.ImageGlass"),
    w("Araçlar", "Bulk Crap Uninstaller", "Klocman.BulkCrapUninstaller"),
    w("Araçlar", "qBittorrent", "qBittorrent.qBittorrent"),
    w("Medya", "VLC", "VideoLAN.VLC"),
    w("Medya", "Spotify", "Spotify.Spotify"),
    w("Medya", "OBS Studio", "OBSProject.OBSStudio"),
    w("İletişim", "Discord", "Discord.Discord"),
    w("İletişim", "Telegram", "Telegram.TelegramDesktop"),
    w("Oyun", "Steam", "Valve.Steam"),
    w("Oyun", "Epic Games Launcher", "EpicGames.EpicGamesLauncher"),
    w("Oyun", "MSI Afterburner", "Guru3D.Afterburner"),
    w("Geliştirme", "Git", "Git.Git"),
    w("Geliştirme", "Visual Studio Code", "Microsoft.VisualStudioCode"),
    w("Geliştirme", "Windows Terminal", "Microsoft.WindowsTerminal"),
    w("Bileşenler", "VC++ 2015-2022 x64", "Microsoft.VCRedist.2015+.x64"),
    w("Bileşenler", "VC++ 2015-2022 x86", "Microsoft.VCRedist.2015+.x86"),
    w("Bileşenler", "DirectX çalışma zamanı", "Microsoft.DirectX"),
    s("LTSC / Store", "Xbox", "9MV0B5HZVK9Z"),
    s("LTSC / Store", "Gaming Services", "9MWPM2CQNLHN"),
    s("LTSC / Store", "Hesap Makinesi", "9WZDNCRFHVN5"),
    s("LTSC / Store", "Ekran Alıntısı Aracı", "9MZ95KL8MR0L"),
    s("LTSC / Store", "HEVC uzantısı", "9N4WGH0Z6VHQ"),
    s("LTSC / Store", "VP9 uzantısı", "9N4D0MSV0403"),
];

/// Temel paket: format sonrası çoğu kişinin ilk kurduğu şeyler.
pub const ESSENTIALS: &[&str] = &[
    "7zip.7zip",
    "Notepad++.Notepad++",
    "voidtools.Everything",
    "VideoLAN.VLC",
    "Microsoft.VCRedist.2015+.x64",
    "Microsoft.VCRedist.2015+.x86",
];

pub fn groups() -> Vec<&'static str> {
    let mut g: Vec<&str> = Vec::new();
    for p in PKGS {
        if !g.contains(&p.group) {
            g.push(p.group);
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unique_and_essentials_exist() {
        let mut seen = std::collections::HashSet::new();
        for p in PKGS {
            assert!(seen.insert(p.id), "tekrar: {}", p.id);
            assert!(!p.id.contains(' ') && !p.id.contains('"'), "güvensiz id: {}", p.id);
        }
        for e in ESSENTIALS {
            assert!(seen.contains(e), "temel pakette olmayan id: {e}");
        }
    }
}
