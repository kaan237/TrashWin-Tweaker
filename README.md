# TrashWin Tweaker

Windows 10/11 için geri alınabilir ayar aracı ve toplu uygulama kurucu. Rust ile yazıldı, tek bir `.exe`, kurulum yok.

[![build](https://github.com/kaan237/TrashWin-Tweaker/actions/workflows/build.yml/badge.svg)](https://github.com/kaan237/TrashWin-Tweaker/actions/workflows/build.yml)
![platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)
![lisans](https://img.shields.io/badge/lisans-MIT-green)

TrashWin Tweaker bir "FPS katlayıcı" değil. Gizlilik, performans, oyun, ağ ve görünüm ayarlarını tek yerde toplar. Her ayarın registry'de neyi değiştirdiğini uygulamadan önce gösterir ve dokunduğu her değerin eski halini saklar. Geri al dediğinde değer, bilgisayarın uygulamadan önceki haline döner. Windows varsayılanını tahmin etmez.

## Neden başka bir tweaker?

Çoğu tweaker "geri al" düğmesinde Windows varsayılanını tahmin eder. Değeri senin ayarın yerine kendi bildiği bir sayıya çeker. TrashWin Tweaker bunu şöyle çözer:

| | TrashWin Tweaker |
|---|---|
| Geri alma | Uygulamadan önce her registry değerinin eski hali `backup.json`'a yazılır. Yedek diske kaydedilmeden hiçbir değer değişmez. Geri alırken birebir eski değer, değer yoksa silme, anahtarı TrashWin Tweaker açtıysa boş anahtarı da temizleme |
| Şeffaflık | Her kartta "Ne değişecek?" bölümü: tam registry yolu, değer ve çalışacak komut |
| Durum | Her ayar için registry okunur: `uygulandı`, `kısmen` veya `varsayılan` |
| Güvenlik | Riskli ayarlar onay penceresi açar. Bilinmeyen tipte bir registry değerinin üzerine yazılmaz. Komutlar `cmd /c` ile birleştirilmez, argüman olarak verilir. PowerShell betikleri `-EncodedCommand` ile gider, tırnak kaçışı sorunu olmaz |
| Toplu işlem | Ayarları ve uygulamaları işaretle, tek tuşla uygula. İstersen önce otomatik geri yükleme noktası alır |
| Boyut | Tek bir Rust `.exe`, .NET gerekmez |

## Özellikler

**Gizlilik**: telemetri ve DiagTrack, reklam kimliği, Başlat menüsü Bing araması, öneriler ve sessiz uygulama kurulumları, etkinlik geçmişi, Copilot ve Recall, Cortana, hata bildirimi (WER), konum.

**Performans**: Nihai Performans güç planı (tekrar tekrar kopyalanmaz), SysMain, hazırda bekletme, güncelleme sonrası zorla yeniden başlatmayı engelleme, Windows Update'i kapatma (riskli), VBS/HVCI ve hypervisor (riskli).

**Oyun**: Game DVR, Oyun Modu, tam ekran optimizasyonları, donanım hızlandırmalı GPU zamanlaması, fare ivmesi, yapışkan tuş kısayolları.

**Ağ**: ağ kısıtlaması (NetworkThrottlingIndex), Cloudflare DNS (IPv4 ve IPv6), SMBv1'i kapatma, DNS önbelleğini temizleme.

**Görünüm ve Gezgin**: Windows 11 klasik sağ tık menüsü, dosya uzantıları ve gizli dosyalar, sola hizalı görev çubuğu, Widgets'ı kapatma, görev çubuğunda "Görevi sonlandır", Gezgini güvenli şekilde yeniden başlatma.

**Bakım**: geri yükleme noktası, geçici dosya temizliği (kaç MB açıldığını yazar), gereksiz hazır uygulamaları kaldırma (Medya Oynatıcı'ya dokunmaz), DISM ve SFC onarımı, .NET 3.5, DirectPlay.

**Uygulama Kur**: WinGet üzerinden 30'dan fazla uygulama: tarayıcılar, 7-Zip, Notepad++, Everything, PowerToys, VLC, OBS, Discord, Steam, VC++ ve DirectX gibi bileşenler. LTSC için Store bileşenleri de var: Xbox, Gaming Services, Hesap Makinesi, Ekran Alıntısı Aracı, HEVC ve VP9. "Temel paket" düğmesi format sonrası ilk kurulanları seçer. Zaten kurulu olan paket hata sayılmaz.

## Kullanım

1. [Releases](https://github.com/kaan237/TrashWin-Tweaker/releases) sayfasından `TrashWin-Tweaker.exe`'yi indir. İstersen `TrashWin-Tweaker.exe.sha256` ile doğrula:
   ```powershell
   (Get-FileHash .\TrashWin-Tweaker.exe -Algorithm SHA256).Hash
   ```
2. Çalıştır. UAC yönetici izni ister, çünkü HKLM ve servis ayarları bunu gerektirir.
3. Soldan kategori seç, ayarları işaretle, **uygula**. Geri almak için aynı ayarları işaretleyip **geri al**.

Yedek ve günlük dosyaları burada durur:

```
%LOCALAPPDATA%\TrashWin\backup.json    uygulanan ayarların eski değerleri
%LOCALAPPDATA%\TrashWin\trashwin.log   yapılan her işlemin kaydı
```

`backup.json`'u silersen geri alma, her ayar için tanımlı güvenli varsayılana döner. Örneğin servisler varsayılan başlangıç tipine döner. Değer tamamen silinmez.

## Kaynaktan derleme

Gereken: [Rust](https://rustup.rs) (stable, MSVC toolchain).

```powershell
git clone https://github.com/kaan237/TrashWin-Tweaker.git
cd TrashWin-Tweaker
cargo test
cargo build --release
# çıktı: target\release\trashwin-tweaker.exe
```

`v*` etiketi push'lanınca GitHub Actions exe'yi derler, SHA-256'sını hesaplar ve Release'e ekler.

## Yeni ayar eklemek

Tüm ayarlar [`src/tweaks.rs`](src/tweaks.rs) içinde veri olarak durur. Yeni bir ayar için bir `Tweak` kaydı eklemen yeterli. Yedekleme, durum okuma ve geri alma otomatik çalışır:

```rust
Tweak {
    id: "ornek",
    cat: Cat::Explorer,
    name: "Örnek ayar",
    desc: "Ne yaptığını ve yan etkisini bir cümleyle yaz.",
    risk: Risk::Safe,
    reboot: false,
    regs: &[dw(Hkcu, r"Software\Ornek", "Deger", 1)],
    apply: &[],
    undo: &[],
},
```

Servis `Start` değerleri için `svc(yol, yeni, varsayılan)` kullan. Testler, servis ayarlarının hepsinde varsayılan değer tanımlı olmasını zorunlu tutar.

## Bilinen sınırlar

- Komutla yapılan ayarların (güç planı, DNS, DISM) durumu registry'den okunamaz. Bunlar "geri alınabilir" ya da "tek seferlik" olarak görünür.
- DNS'i geri almak DHCP'ye (otomatik) döner. Önceden elle girilmiş bir DNS varsa saklanmaz.
- Güç planını geri almak Dengeli plana döner.
- Yönetici izni farklı bir hesapla veriliyorsa, örneğin standart kullanıcı yönetici şifresi girerse, HKCU ayarları o yönetici hesabına yazılır.
- Windows Update'i kapatmak güvenlik yamalarını durdurur. Windows bu servisi bir süre sonra kendisi onarabilir.
- VBS kapatmak WSL2, Hyper-V ve Sandbox'ı bozar. Bazı anti-cheat'ler (Vanguard, FACEIT) VBS/HVCI isteyebilir.
- Hazır uygulama kaldırma geri alınamaz. Gerekirse Microsoft Store'dan yeniden kur.

## Gizlilik

TrashWin Tweaker hiçbir yere veri göndermez. Telemetri yok, güncelleme kontrolü yok. İnternete yalnızca WinGet kurulumları, DISM onarımı ve .NET 3.5 kurulumu çıkar.

## Teşekkür

Fikir, [QT-DelayLess/ChaosBoost-Tweaker](https://github.com/QT-DelayLess/ChaosBoost-Tweaker)'dan esinlendi. TrashWin Tweaker onun kodunu kopyalamaz. Rust ile sıfırdan yazıldı ve yedekten geri alma üzerine kuruldu.

## Lisans

[MIT](LICENSE). Kullanmak senin sorumluluğunda. Ayar uygulamadan önce geri yükleme noktası almak her zaman iyi bir fikir.
