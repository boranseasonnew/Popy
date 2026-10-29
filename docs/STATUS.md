# Popy — geliştirme durumu

29 Ekim 2026. **Aşama 0 / ilk çalışan teknik dilim tamamlandı. Kalite kapısı A henüz geçilmedi.**

## Çalışan parçalar

- Rust CLI: doctor, fixtures, inspect, benchmark.
- Tekrar üretilebilir 10 saniyelik 720p sentetik kaynak; 360p, 144p ve soluk renkli 144p girdiler.
- Gerçek ncnn/Vulkan çıkarımı: animevideo 2× ve genel x4plus → 2× çıktı.
- Gerçek zaman damgalarıyla CFR denetimi; HDR/renk/rotasyon/SAR/ses başlangıcı denetimi.
- RGB/sRGB ↔ BT.709 için zscale ile gerçek matris/transfer dönüşümü.
- Açılıp kapanan otomatik ton/kontrast ve hafif doygunluk. Siyah beyaz AI renklendirmesi değil.
- Model/araç/kaynak hash'leri, aşama süreleri, alt süreç tepe RAM'i ve durum/günlük dosyaları.
- Çıktı çözünürlüğü, kare sayısı, rasyonel FPS, süre, ses varlığı ve AAC paketlerinin kaynak önekiyle eşleşmesi denetimi.
- Mevcut çıktı üzerine yazmama, kaynak koruma, süre sınırı ve Ctrl+C ile alt süreci durdurma.
- Windows'ta aynı proje içinde aynı GPU için iki Popy işinin çakışmasını önleyen, süreç kapanınca serbest kalan OS dosya paylaşım kilidi.
- Proje içi araç kurulumu, checksum doğrulama, Cargo.lock, format/test/Clippy ve yerel smoke scripti.

## Ölçülen sonuçlar

Makine: AMD Radeon(TM) Graphics iGPU, sürücü 31.0.21925.1001, yaklaşık 16 GB sistem RAM'i. Windows'un bildirdiği 512 MB ayrılmış adapter belleği, iGPU'nun toplam kullanılabilir belleği değildir. ncnn logunda seçilen cihaz 0: AMD Radeon(TM) Graphics.

Tüm 10 saniyelik klipler 12 FPS / 120 kare. Tek GPU işi, tile 64, 1:1:1 iş parçacığı ayarı. Süreler tek çalıştırma; soğuk model yüklemesi ve PNG I/O içerir.

| Deneme | Model | AI aşaması | Tüm karşılaştırma işi | AI süreci tepe RAM |
|---|---|---:|---:|---:|
| 144p → 288p, 10 sn | animevideo 2× | 21,55 sn | 23,16 sn | 163,2 MiB |
| 360p → 720p, 10 sn | animevideo 2× | 125,20 sn | 130,04 sn | 170,8 MiB |
| Soluk 144p, renk kapalı, 10 sn | animevideo 2× | 21,10 sn | 22,70 sn | 148,2 MiB |
| Soluk 144p, renk açık, 10 sn | animevideo 2× | 22,12 sn | 23,88 sn | 135,4 MiB |
| Genel model uyumluluk denemesi: yalnızca 2 kare | x4plus, yerel 4× → 2× | 14,25 sn | 14,90 sn | 1078,7 MiB |

Genel model satırı 10 saniyelik benchmark değildir ve diğer satırlarla hız sıralaması için kullanılmamalı. Küçük deneme, bu iGPU'da genel modelin belirgin maliyetini gösteriyor. Anime modelinin hızlı olması gerçek çekimde uygun varsayılan olduğunu kanıtlamıyor.

RAM ölçümü alt sürecin working-set tepe değeri. Toplam sistem belleği, sürücü belleği ve VRAM ayrıca ölçülmedi. Karşılaştırma işi Lanczos, görsel üretimi ve doğrulamaları da içeriyor; saf dışa aktarma FPS'i değil. Sonraki sürümün yeni AAC doğrulama adımı bu eski ölçümlerin içinde değil; yeni release ayrıca smoke testinden geçti.

## Doğrulama

- 7 otomatik test geçti; varsayılan olarak atlanan tek test, timeout testinin ayrı süreçte çağırdığı bekleme yardımcısı.
- `cargo fmt --check`, tüm hedeflerde Clippy (`-Dwarnings`) ve release derleme geçti.
- `scripts/smoke.ps1 -WithGpu`: yeni fixture, geçerli girdi, VFR/HDR reddi, gerçek GPU, otomatik renk, Türkçe/boşluk/noktalı virgül içeren yol, AAC payload doğrulaması ve mevcut çıktı reddi geçti.
- Sessiz kaynak inspect ile kabul edildi; iki karelik ayrı GPU dışa aktarımı ses eklenmeden başarıyla tamamlandı (`runs/silent-smoke-final/`).
- 144p tam klipte kaynak/çıktı AAC birleşik SHA-256 eşleşmesi ayrıca doğrulandı. Kısa kesitlerde tüm kaynak hash'i yerine aynı sıradaki kaynak paketlerinin öneki kontrol edilir; AAC paket sınırları korunur.
- Ses zamanlaması metadata ile denetlenir; kısa AAC kesitleri için en fazla 25 ms süre farkı kabul ediliyor. Algısal senkron/waveform hizası sertifikası yok.
- Kaynak dosya hash'i smoke öncesi/sonrası aynı kaldı.
- GitHub Actions dosyası hazırlandı; uzak depoda CI çalıştırılmadı, commit/push/yayın yapılmadı.

## Yerel sonuçlar

Sonuçlar `runs/` altında ve Git dışında:

- `popy-first-demo.mp4`: kaynak 360p / Popy 720p, 10 saniyelik yan yana video.
- `anime-144p-10s-v2/`: başarılı 144p teknik deneme.
- `anime-360p-10s/`: 720p çıktı, Lanczos tabanı ve rapor.
- `general-144p-smoke/`: genel modelin 2 karelik uyumluluk denemesi.
- `renk kapali 10s/`, `renk açık Türkçe 10s/`: renk ayarı karşılaştırması.
- `color-toggle-comparison.png`: renk kapalı/açık karşılaştırması.

Seçilmiş ham ölçümler ve sonradan doğrulanan AAC paket/kaynak hash kontrolleri `docs/measurements/2026-10-29.json` içinde kayıtlı.

Başarısız ilk işler ve logları silinmedi. Bu denemeler Windows verbatim yol uyumsuzluğunu, PNG sonrası piksel oranını ve sRGB transfer bilgisini tespit edip düzeltmek için kullanıldı. `completed` raporu olmayan işleri başarılı saymayın.

## Repo ve tekrarlı benchmark güncellemesi

- Public repo: https://github.com/boranseasonnew/Popy
- İngilizce ana README, Türkçe README ve kısa yol haritası eklendi.
- `suite` komutu: katı JSON manifesti, tekrarlar, sırayla GPU işleri ve JSON/CSV özetleri.
- Kuru çalışma araç/model kurmadan doğrulandı. Dokuz GPU işi tamamlandı; renk açık/kapalı ve 144p/360p vakaları tekrarlandı. Her iş kaynak klibin ilk bir saniyesini işledi.
- Hatalı HDR vakasından sonra geçerli vakanın devam ettiği, suite'in nonzero çıkış ve `completed_with_errors` verdiği doğrulandı. Hatalı işler hız istatistiğine katılmıyor.
- 12 otomatik test, format kontrolü, Clippy ve release derlemesi geçti. Tek atlanan test, timeout testinin ayrı süreçte kullandığı yardımcısıdır.
- Büyük araç/model dosyaları, çıktılar ve önceki logo denemeleri Git dışında. Uzak CI ilk push sonrasında ayrıca doğrulanacak.

## Sıradaki işler

1. Sentetik desen dışında hareketli çizim ve gerçek çekim içeren, kullanım hakkı belli kalite setini oluşturmak; 15 kliplik hedefe ilerlemek.
2. Genel içerik için daha hafif gerçek 2× model adayını seçmek ve ağırlık lisansını belgelemek.
3. Aynı ayarlarla tekrarlı hız ölçümleri, VRAM ölçümü ve temporal kusur değerlendirmesi.
4. Kapı A kararı: hedef içerikte fayda ve pratik hız kanıtı. Geçilirse diske PNG dökmeyen Rust worker/FFI aşaması.
5. Bunun ardından Popy masaüstü ekranı, kısa önizleme ve kuyruk.

Masaüstü arayüzü, uzun video, HDR/VFR dönüşümü, RIFE ve siyah beyaz AI renklendirmesi henüz uygulanmadı. RTX veya ayrı AMD kartı bu makinede test edilmedi. Uygulamanın bütün yol haritası tamamlanmış sayılmaz.
