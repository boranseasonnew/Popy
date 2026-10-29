# Popy

[English](README.md) · Türkçe

Yerel video iyileştirme uygulaması. Rust çekirdeği, ncnn/Vulkan ve turuncu kırmızı panda.

**Şimdiki durum: Aşama 0 araştırma CLI'si. Masaüstü arayüzü ve uzun video motoru henüz yok.**

## İlk çalışan dilim

- 10 saniyelik 720p sentetik kaynak ve ondan 360p, 144p, soluk renkli 144p girdiler üretir.
- Videonun gerçek kare zaman damgalarını okuyarak CFR, BT.709 SDR, ses ve boyut sözleşmesini denetler.
- Gerçek Real-ESRGAN modelini seçilen Vulkan GPU'sunda çalıştırır.
- Aynı klipten Lanczos ve AI sonuçları, karşılaştırma görseli ve ölçüm raporu çıkarır.
- `--auto-color`: zaman içinde yumuşatılmış otomatik ton/kontrast ve hafif doygunluk. **Siyah beyaz videoya renk üreten AI modeli değildir.**
- Her iş yeni bir klasöre yazılır; kaynak ve mevcut sonuçların üzerine yazılmaz. Ctrl+C alt süreci durdurur.

## Windows geliştirme

PowerShell 7, Windows x64, Vulkan destekli güncel sürücü ve MSVC/Windows SDK kitaplıkları gerekir. `bootstrap.ps1`, Rust ve araçları yalnızca `.tools/` içine kurar; kullanıcı PATH'ini değiştirmez. Mevcut makinedeki eksik VS bağlayıcısı için `dev.ps1`, Rust'ın LLD bağlayıcısını SDK/CRT kitaplıklarıyla kullanır. Kitaplıklar yoksa ayrıca Visual Studio Build Tools gerekir; script sistem kurulumunu otomatik değiştirmez.

```powershell
.\scripts\bootstrap.ps1
.\scripts\dev.ps1 test
.\scripts\dev.ps1 clippy --all-targets
.\scripts\dev.ps1 run -- doctor
.\scripts\dev.ps1 run -- fixtures --out fixtures/generated/my-test
.\scripts\dev.ps1 run -- benchmark --input fixtures/generated/my-test/input-144p.mp4 --out runs/my-test --model anime --seconds 10 --gpu 0 --tile 64 --rights "Generated FFmpeg technical fixture"
.\scripts\dev.ps1 run -- benchmark --input fixtures/generated/my-test/faded-144p.mp4 --out runs/my-color-test --model anime --seconds 10 --gpu 0 --tile 64 --rights "Generated FFmpeg technical fixture" --auto-color
```

`--out` klasörünün var olmaması gerekir. Aynı komutu tekrar çalıştırırken yeni ad seçin. `POPY_TOOLS` ortam değişkeni ile farklı bir runtime.json yolu verilebilir.

`general`: RealESRGAN x4plus, yerel 4× çıkarım ve ardından 2× boyuta küçültme. `anime`: gerçek 2× animevideo modeli; genel gerçek çekim kalitesi için varsayılan seçildiği anlamına gelmez.

```powershell
.\scripts\dev.ps1 build --release
.\target\release\popy.exe --help
.\scripts\smoke.ps1 -WithGpu
```

## Deneme sözleşmesi

Tekrarlı karşılaştırmalar için `suite` komutu eklendi. Ayrıntılar [benchmark listelerinde](benchmarks/README.md):

```powershell
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --dry-run
.\scripts\dev.ps1 run -- fixtures --out fixtures/generated/suite-demo
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --out runs/suite-01
```

Örnek liste, 10 saniyelik kaynakların ilk bir saniyesini tekrarlar; başarısız işleri zaman ortalamasına katmaz. Araç kurmadan da `--dry-run` ile manifest denetlenebilir.

Girdi en fazla 30 saniye; işleme en fazla 10 saniye ve 300 kare. H.264, yuv420p, progresif, BT.709 limited-range, kare piksel, sıfıra yakın video/ses başlangıcı ve en fazla tek AAC ses akışı. HDR, VFR, rotasyon ve interlaced açık hata alır. Bu sınırlar geçici araştırma kapsamıdır.

**Bu prototip PNG karelerini diske yazar.** 2 GB tahmini ham kare bütçesi ve 4K yerel model çıkışı sınırı vardır. Uzun video desteği, ayrı akış halinde worker aşamasında geliştirilecek.

Her işte `enhanced.mp4`, `lanczos.mp4`, `comparison.png`, `report.json`, `report.md`, `job.json`, model/araç hash'leri ve günlükler oluşur. RAM ölçümü alt sürecin tepe working-set değeridir; toplam sistem RAM'i veya VRAM değildir. VRAM henüz ölçülmüyor. Workflow süresi karşılaştırma üretimini de kapsar; uygulamanın tek-video dışa aktarma FPS'i olarak yorumlanmamalı.

Sentetik desenler entegrasyonu test eder. Yüz, saç, ince doku, doğal hareket ve zamansal kalite için gerçek çekim seti hâlâ gerekli; Aşama 0 kalite kapısı bu çıktılarla otomatik geçmiş sayılmaz.

## Proje kayıtları

- [Araştırma planı](VIDEO_AI_ARASTIRMA_VE_YOL_HARITASI.md)
- [Güncel ilerleme ve test sonuçları](docs/STATUS.md)
- [Üçüncü taraf kaynakları](docs/THIRD_PARTY.md)
- [Görsel kimlik](branding/IDENTITY.md)
