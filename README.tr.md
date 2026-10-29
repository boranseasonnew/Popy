<p align="center"><img src="branding/popy-logo-v1.png" alt="Popy" width="180"></p>

# Popy

Rust ile geliştirilen, bilgisayarında çalışan açık kaynak video iyileştirme uygulaması.

[![Rust checks](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml/badge.svg)](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml)

**0.2 ön sürüm:** Masaüstü arayüzü, sıralı kuyruk, 2× AI upscale, açılıp kapanan otomatik renk düzeltme, kısa önizleme ve tam klip dışa aktarma var. Algısal kalite ve geniş donanım uyumluluğu henüz doğrulanmadı; kararlı 1.0 değildir.

[Windows ZIP indir](https://github.com/boranseasonnew/Popy/releases) · [Kurulum ve kullanım](docs/QUICKSTART.tr.md) · [Durum](docs/STATUS.md) · [Yol haritası](docs/ROADMAP.md) · [English](README.md)

ZIP'i tamamen çıkar, ilk kullanımda `Araclari Kur.cmd` dosyasını çalıştır, ardından `popy-desktop.exe` dosyasını aç. Rust ve Visual Studio gerekmez. Kurulum araçları ve modelleri GitHub'dan indirip SHA-256 ile kontrol eder. İşleme sırasında videolar dış servise gönderilmez.

Video ekle; gerçek çekim için Genel, çizim için Anime modelini seç. İstersen otomatik rengi açıp önce üç saniyelik önizleme al. İlk karede önce/sonra kaydırıcısını kullan; hareketli sonucu videoyu açarak incele. Hazır videoları `Kuyruğu işle` ile sırayla dışa aktar. Her iş yeni klasöre video, karşılaştırma, rapor ve günlük yazar. Kaynak ve mevcut sonuçların üzerine yazılmaz.

Otomatik renk ton/kontrast/doygunluk ayarıdır. Siyah beyaz görüntüye AI ile yeni renk eklemez. Modeller Popy tarafından eğitilmedi; kaynak ve modele göre sonuç değişir. Genel model iGPU'da yavaştır.

Girdi: H.264, sabit FPS, progressive 8-bit yuv420p, açık BT.709 limited SDR etiketi, kare piksel ve sıfıra yakın başlangıç; en fazla bir AAC akışı. HDR, VFR, döndürme ve diğer kodekler henüz desteklenmez. Çıktı 2×; genel model en fazla 960×540, anime modeli 1920×1080 girdi kabul eder.

Yeni motor küçük PNG gruplarıyla çalışır, grup bitince kareleri temizler. Geçici kayıpsız RGB segmentleri video süresiyle büyür. Başarılı işte temizlenir; hata/iptalde inceleme için korunur. Disk alanı kontrolü vardır. Model her grupta yeniden yüklenir; doğrudan Rust/FFI çıkarımı henüz yoktur.

Vulkan sürücüsü gerekir. AMD Radeon iGPU üzerinde teknik testler yapıldı. RTX 30/40/50, ayrı AMD ve Intel kartları hedefleniyor; bu ailelerde test edilmiş uyumluluk iddiası yok.

Geliştirme için Windows x64, PowerShell 7, MSVC ve Windows SDK kütüphaneleri gerekir:

```powershell
.\scripts\bootstrap.ps1
.\scripts\dev.ps1 test
.\scripts\dev.ps1 build --release
.\target\release\popy-desktop.exe
```

CLI ile tam klip:

```powershell
.\target\release\popy.exe enhance --input video.mp4 --out runs/export-01 --model general --auto-color
```

`--preview-seconds 3` kısa örnek üretir. `benchmark` ve `suite`, Lanczos karşılaştırması ve tekrarlı ölçümler için korunmuştur. Ayrıntılar [İngilizce README](README.md) ve [benchmark rehberinde](benchmarks/README.md).

Kod [Apache-2.0](LICENSE) lisanslıdır. FFmpeg, ncnn, Real-ESRGAN ve ağırlıkları ayrı lisanslara tabidir; [envanteri](docs/THIRD_PARTY.md) incele. Popy bağımsızdır, Topaz Labs ile bağlantısı yoktur.
