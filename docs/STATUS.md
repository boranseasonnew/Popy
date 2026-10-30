# Current hotfix — 0.2.0-preview.3

Desktop/enhance now automatically import FFmpeg-decodable inputs rather than rejecting every format outside the research contract. Full-range yuvj420p, different codecs/bit depths, VFR, rotation, flagged interlacing and pixel aspect are normalized. PQ/HLG HDR is tone-mapped to 8-bit SDR. Source files stay unchanged; color assumptions and conversions are recorded. The old desktop model-specific resolution rejection was removed; hardware/disk/encoder limits still apply. Output remains 2× SDR H.264/AAC MP4. See [release notes](RELEASE-0.2.0-preview.3.md).

The new import matrix passed ten synthetic format cases plus a provided 1080p/60 source with 180 frames and audio. Additional rotation/240 FPS checks and 4K-frame/general-model GPU smokes passed; see [import measurements](measurements/2026-10-30-import.json). These are not full natural-footage quality or multi-hour certifications.

The preview.2 measurements below describe the earlier restricted input path and remain historical evidence, not a statement of current desktop input restrictions.

---
# Popy — geliştirme durumu

30 Ekim 2026 · **0.2.0-preview.2**. Yeni masaüstü tasarımı, 20 dil, özelleştirilebilir kısayollar ve imzalı uygulama içi güncelleme. Kararlı 1.0 kalite/donanım kapıları henüz geçilmedi.

## Çalışan özellikler

- İngilizce varsayılan, 20 dilli masaüstü: dosya seçme/sürükleme, model ve otomatik renk ayarı, GPU/tile, kayıt klasörü.
- Açık/koyu yumuşak arayüz; animasyonlu tilki/ayı, gizleme ve azaltılmış hareket; arayüz boyutu ayarı.
- 29 özelleştirilebilir kısayol, klavyeyle iş seçme/sıralama, yakınlaştırma ve önce/sonra kontrolü.
- Kullanıcı isteğiyle GitHub sürüm kontrolü; Ed25519 imzalı manifest, paket hash/dosya kontrolü, uygulama içi indirme, kurma, geri alma ve yeniden açma.
- Gizli ve denetimli ilk araç kurulumu; göreli araç yolları ve yarıda kalan kurulumu onarma.
- Sıralı kuyruk, ilerleme, iptal, hatalı/yarıda kalan işi tekrar deneme ve oturum kaydı. Retry sıfırdan başlar.
- Kısa önizleme; ilk kare önce/sonra kaydırıcısı; sonucu sistem video oynatıcısında açma.
- Real-ESRGAN genel model: yerel 4× → 2× çıktı. Animevideo: yerel 2×.
- Tam klip `enhance`: geri basınçlı RGB pipe decoder, küçük PNG grupları, kayıpsız RGB segmentleri, son kodlama ve doğrulama.
- Mevcut grubun tahmini ham RGB bütçesi 192 MiB; grup PNG'leri her gruptan sonra silinir. Kayıpsız segmentler süreyle büyür, başarılı işte temizlenir. Disk boşluğu önceden kontrol edilir.
- Otomatik ton/kontrast ve hafif doygunluk; grup birleşiminden sonra tek filtre zinciriyle uygulanır. Siyah beyaz AI renklendirme değildir.
- Gerçek sRGB/BT.709 matris/transfer dönüşümü, CFR zaman damgaları ve ses denetimi. Karşılaştırma PNG'leri de sRGB'ye dönüştürülür.
- Kaynak/model/araç hash'leri, kare/FPS/süre/çözünürlük ve AAC paket denetimi; kaynak son hash'iyle değişmezlik kontrolü.
- Atomik ilerleme JSON'u, zaman sınırları, GUI iptal dosyası, Ctrl+C, Windows Job Object ile alt süreç/torun yaşam döngüsü.
- Mevcut çıktı klasörüne yazmama, aynı proje/GPU için OS dosya kilidi; hata/iptalde günlükleri koruma.
- CLI `doctor`, `inspect`, `fixtures`, `enhance`, `benchmark`, `suite`, `--version`.
- Windows ZIP paketleme, statik MSVC CRT, lisans/font bildirimleri ve SHA-256 manifesti. Runtime/model dosyaları ZIP dışında; ilk kurulumda sabit arşivler indirilir. Son kullanıcı Rust kurmaz.

## Yapılan doğrulamalar

Makine: AMD Radeon(TM) Graphics iGPU, sürücü 31.0.21925.1001, yaklaşık 16 GB sistem RAM'i. Ayrılmış adapter RAM bildirimi toplam iGPU belleği olarak yorumlanmadı. VRAM ölçülmedi.

- 24 otomatik test geçti. İki `ignored` test, başka testler tarafından ayrı süreçte çağrılan bekleme/torun süreç yardımcılarıdır.
- Format, tüm hedeflerde Clippy (`-D warnings`) ve Windows release derleme geçti.
- Süre sınırında doğrudan alt sürecin sonlandığı ve Job Object kapanınca torun sürecin de öldüğü test edildi.
- `scripts/export-smoke.ps1 -WithDesktop`: 33 saniye, 128×72, 12 FPS, 396 kare kaynak → 256×144; 17 grup. Kareler, süre, FPS, AAC paketleri ve kaynak hash'i doğrulandı. Başarılı işin `work/` klasörü temizlendi. Mevcut çıktı reddedildi.
- 30000/1001 FPS, 90 kare, sessiz kaynakta çok gruplu dışa aktarma geçti; kesirli FPS ve sessizlik korundu.
- Gerçek çıkarım aşamasında iptal testi: nonzero çıkış, `cancelled` durumu, son video üretilmemesi; ardından yeni GPU işi çalıştı.
- Masaüstü kendi CLI işleyicisini başlatıp otomatik renkli önizleme üretti, sonucu yükledi ve gerçek framebuffer ekran görüntüsü kaydetti. Görüntü elle incelendi.
- Yeni arayüzün 20 dilinde framebuffer kontrolleri yapıldı. İngilizce, Türkçe, Hintçe ve Arapça örnekleri görsel olarak incelendi; Arapça birleşme/sıralama kontrol edildi. Tüm çeviri anahtarları 20 dilde doluluk testinden geçer; ana dil konuşuru çeviri incelemesi henüz yapılmadı.
- Gerçek klavye olaylarıyla tema, eşlikçi, model, renk, ayarlar, yardım ve önce/sonra seçimi doğrulandı.
- Güncelleme yardımcısının başarı, geri alma, kullanıcı verisi koruma ve yol kaçışı reddi senaryoları izole klasörlerde geçti. İmza değişikliği, sürüm düşürme ve başka kaynak reddi birim testleri geçti.
- Dağıtım ZIP'inde 486 dosyanın boyutu ve hash'i doğrulandı. Paket içinde ilk araç kurulumu, doğrulanmış önbellek arşivleriyle gizli Windows PowerShell üzerinden geçti. Uygulama Unicode/boşluk içeren başka klasöre taşındı ve farklı çalışma dizininden GPU önizlemesi üretildi.
- Gerçek Setup EXE'si izole klasöre kuruldu; dosya hash'leri ve sürüm doğrulandı. Kaldırma sırasında test ayarları/çıktıları korundu. Bunlar mevcut makinedeki testlerdir; bağımsız temiz Windows testi değildir.
- Yayınlanan GitHub sürümünden gerçek uygulama içi geçiş: izole `0.2.0-preview.1` test istemcisi `0.2.0-preview.2` paketini indirdi, imza/arşiv/dosyaları doğruladı, kurdu ve yeni GUI'yi açtı. 486 dosya, CLI sürümü, runtime ayarı, Türkçe/koyu tema/ayı tercihleri ve çıktı dosyası doğrulandı. Eski istemci yalnızca yerel test kopyasıdır; ayrıca bir eski kamu sürümü yayınlanmadı.
- `scripts/smoke.ps1 -WithGpu`: yeni 10 saniyelik fixture üretimi, VFR/HDR reddi, GPU + renk, Unicode/boşluk/noktalı virgüllü yol, AAC kontrolü, kaynak koruma ve mevcut çıktı reddi geçti.

Son 33 saniyelik entegrasyon işi yaklaşık 57.78 sn sürdü. Bu, izole/tekrarlı hız benchmark'ı değildir; diğer derleme/denetim işleri süreyi etkilemiş olabilir. 128×72 sentetik kaynak, doğal video kalite kanıtı değildir. Seçilmiş kayıtlar [2026-10-30-export.json](measurements/2026-10-30-export.json) içinde. Önceki 144p/360p kısa benchmark'lar [2026-10-29.json](measurements/2026-10-29.json) içinde; farklı motorların süreleri doğrudan eşdeğer değildir.

GitHub Actions; format, Clippy, test ve Windows release derlemesini çalıştırır. GPU ve masaüstü smoke testleri yereldir; CI'da GPU testi yapıldığı iddia edilmez.

## Sürümün sınırları

Girdi: tek progressive H.264, 8-bit yuv420p, açık BT.709 limited SDR, kare piksel, CFR 1–120 FPS ve sıfıra yakın başlangıç. En fazla bir AAC akışı ve yakın ses/video süreleri. HDR, VFR, rotasyon, interlaced, belirsiz renk etiketleri ve diğer kodek/akış düzenleri reddedilir.

Çıktı 2×. İç model boyutu en fazla 3840×2160: genel girdi en fazla 960×540, anime 1920×1080. `enhance` süre güvenlik sınırı 6 saat; altı saatlik iş test edilmedi. `benchmark` kısa denemeleri 10 sn/300 kare ile sınırlar.

Genel model iGPU'da pahalıdır. ncnn modeli her grupta yeniden yüklenir. PNG'ler sınırlıdır fakat geçici segmentlerin toplam alanı süreyle büyür. Ses kontrolü metadata ve AAC paketleri düzeyindedir; algısal senkron sertifikası değildir. Kaldığı kareden devam ve gömülü video oynatıcı yoktur.

## Kararlı 1.0 için kalanlar

1. Kullanım hakkı belli en az 15 doğal/hareketli klip; yüz, doku, yazı, sıkıştırma ve temporal titreme değerlendirmesi. Kalite kapısı hâlâ `false`.
2. Daha pratik genel model ve doğrulanmış ağırlık/çevrim kaynağı.
3. Çok saatlik, bozuk dosya, disk dolması ve daha geniş kapanma/kurtarma senaryoları.
4. Gerçek RTX 30/40/50, ayrı AMD ve Intel/AMD iGPU donanım matrisi; cihaz listesi ve daha anlaşılır GPU seçimi.
5. Yaygın etiketsiz SDR dosyaları, kodekler, VFR ve ses ofsetleri için açık ve zamanlamayı koruyan içe aktarma.
6. Bağımsız temiz Windows makinesinde kurulum ve sürücü/runtime testi; imzalama kararı.

Bu yüzden sürüm ön sürüm olarak dağıtılır. Tam çevrimdışı runtime/model paketi, HDR, RIFE/FPS artırma ve AI siyah beyaz renklendirme henüz yoktur. [Yol haritası](ROADMAP.md), [kurulum rehberi](QUICKSTART.tr.md), [sürüm notları](RELEASE-0.2.0-preview.2.md).
