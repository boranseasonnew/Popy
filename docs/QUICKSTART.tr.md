# Popy ile başla

1. [Sürümler](https://github.com/boranseasonnew/Popy/releases) sayfasından Windows x64 Setup EXE'sini indirip kur. Taşınabilir kullanım için ZIP'i tamamen yazılabilir bir klasöre çıkarabilirsin.
2. Popy'yi aç. İlk kullanımda **Set up tools** düğmesine veya **F8**'e bas. Alternatif: `Setup Tools.cmd`. Kurulum, SHA-256 denetimli FFmpeg, ncnn ve model arşivlerini `.tools/` içine indirir. Rust ve Visual Studio gerekmez.
3. **Ctrl+,** ile Settings'i açıp dili Türkçe seç. Varsayılan dil İngilizcedir; 20 arayüz dili vardır.
4. **Ctrl+O** ile video ekle. Gerçek çekim için genel modeli, çizim için anime modelini seç. İstersen otomatik renk ayarını aç.
5. **F5** ile kısa önizleme üret. Ayırıcıyı sürükle, **Space** tuşunu basılı tutarak kaynağı gör veya **B / A** ile önce/sonra seç. Karşılaştırma ilk kareyi gösterir; hareketli sonucu sistem oynatıcısında açabilirsin.
6. **Ctrl+Enter** ile kuyruğu işle, **Ctrl+Shift+X** ile durdur. Her iş yeni klasörde `enhanced.mp4`, karşılaştırma, rapor ve günlük üretir. Kaynak ve mevcut çıktı klasörleri üzerine yazılmaz.

## Sana uygun çalışma alanı

Ayarlar'dan açık/koyu tema, tilki/ayı/gizli eşlikçi, azaltılmış hareket, arayüz boyutu ve kısayolları değiştir. **F6** temayı, **Ctrl+B** eşlikçiyi değiştirir. **F1** kısayol düzenleyicisini açar. Tab / Shift+Tab kontroller arasında dolaşır. Yerel dosya pencereleri işletim sisteminin dilini kullanır; bazı teknik hata ayrıntıları çevrilmemiştir.

**Alt+Sol / Sağ** iş seçer, **Alt+Yukarı / Aşağı** sıralar. **Delete**, çalışmayan seçili işi listeden kaldırır; kaynak videoyu silmez. **Ctrl+Shift+Enter** sonucu, **Ctrl+Shift+O** sonuç klasörünü açar. **Ctrl+Shift+S** kayıt klasörünü değiştirir. **Alt+C** otomatik rengi, **Alt+1 / 2** modeli seçer. **F11** tam ekranı açar. Kısayollar Ayarlar'dan değiştirilebilir.

Ayarlar sıraya alırken işe kaydedilir. Durdurma kuyruktaki işleri bekletir. Yeniden deneme sıfırdan başlar; kaldığı kareden devam henüz yoktur. Başarılı işte geçici kareler temizlenir; hata/iptalde inceleme için korunur.

## Uygulama içinden güncelle

**Ctrl+U** veya **Güncellemeleri denetle** düğmesini kullan. Yeni imzalı sürüm bulunduğunda **İndir ve kur** ile devam et. Popy yayıncı imzasını ve paket dosyalarını doğrular, kapanır, uygulama dosyalarını değiştirir ve yeniden açılır. Kurulumdan önce çalışan işi bitir veya durdur. Ön sürümleri dahil etme seçeneği Ayarlar'dadır.

`.tools/`, ayarlar ve çıktılar korunur. Dosya değiştirme hatasında kurucu önceki dosyaları geri yüklemeye çalışır ve yedek bırakır; sonraki açılış sonucu gösterir. Zamanlanmış arka plan güncelleme kontrolü yoktur. İlk araç kurulumu ve isteğe bağlı güncelleme internet kullanır; video işleme bilgisayarda gerçekleşir.

## Destek ve sınırlar

Windows x64 ve güncel Vulkan sürücüsü gerekir. Yerel GPU testleri AMD Radeon iGPU'da yapıldı; RTX 30/40/50, ayrı AMD ve Intel kartları henüz doğrulanmadı. Windows EXE'sinde Authenticode imzası yoktur. Güncelleme paketlerinin Ed25519 imzası ayrı bir mekanizmadır.

Bu ön sürüm tek progressive H.264, sabit 1–120 FPS, 8-bit yuv420p, açık BT.709 limited SDR etiketi, kare piksel ve sıfıra yakın başlangıç kabul eder. En fazla bir hizalı AAC akışı desteklenir. HDR, VFR, döndürme, interlaced, etiketsiz renk ve diğer kodekler henüz desteklenmez.

Çıktı 2×. Genel model içeride 4× çalışır; girdi sınırı 960×540'tır. Anime girdi sınırı 1920×1080'dir. Genel model iGPU'da yavaştır. Otomatik renk ton/kontrast/doygunluk düzeltir; siyah beyaz görüntüye yeni renk eklemez.

PNG grupları sınırlıdır fakat geçici kayıpsız segmentlerin toplam alanı video süresiyle büyür. Boş disk kontrolü yapılır. 6 saatlik süre sınırı bir güvenlik sınırıdır; 6 saatlik dayanıklılık testi iddiası değildir. GPU bellek hatasında tile değerini 64'ten 32'ye düşürmeyi dene.

Popy kodu Apache-2.0; üçüncü taraf araç, model ve fontların lisansları ayrıdır. Bildirimler `DEPENDENCIES.md`, `licenses/` ve `THIRD_PARTY.md` içindedir.