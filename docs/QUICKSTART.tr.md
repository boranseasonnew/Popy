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

Masaüstü ve `enhance`, kurulu FFmpeg'in okuyabildiği videoları otomatik hazırlar: MP4/MOV/MKV/WebM/AVI ve diğer konteynerler, H.264/HEVC/VP9 ve mevcut diğer çözücüler, farklı piksel biçimleri ve renk derinlikleri. Dosya seçicide Tüm dosyalar seçeneği vardır. İlk gerçek video ve ilk ses akışı kullanılır; diğer ses, altyazı ve veri akışları aktarılmaz.

Uygun girdiler doğrudan işlenir. Diğerleri geçici, kayıpsız H.264 kodlamalı 8-bit BT.709 SDR işleme kopyasına dönüştürülür. VFR sabit FPS'ye çevrilir, rotasyon uygulanır, işaretli interlaced görüntü düzeltilir, piksel oranı düzeltilir ve tek sayılı boyutlara dolgu eklenir. PQ/HLG HDR, SDR'ye ton eşleme ile dönüştürülür. Eksik renk bilgilerindeki varsayımlar rapora yazılır. İçe aktarma dönüşümü gerektiğinde ses videoya hizalanıp stereo AAC olarak kodlanır. Kaynak değişmez. `inspect`/`benchmark` araştırma komutları sıkı biçim denetimini korur; `prepare`, GPU kullanmadan içe aktarmayı sınar.

Çıktı hâlâ 2× H.264/AAC MP4, 8-bit SDR'dir. HDR ve kaynak kodlamanın tüm özellikleri korunmaz. Eski modele özel girdi çözünürlük sınırları kaldırıldı; yüksek çözünürlüklü kaynak sessizce küçültülmez. Gerçek GPU/RAM, geçici disk ve kodlayıcı sınırları geçerlidir. Genel model yüksek çözünürlükte çok yavaş olabilir. Bozuk, şifreli veya desteklenmeyen özel biçimler yine hata verebilir.

6 saatlik süre sınırı dayanıklılık testi iddiası değildir. PNG grupları sınırlıdır; kayıpsız segmentlerin toplam alanı süreyle büyür. GPU bellek hatasında tile değerini düşürmeyi dene. Otomatik renk, ton/kontrast/doygunluk düzeltir; siyah beyaz görüntüye yeni renk eklemez.

Windows x64 ve Vulkan sürücüsü gerekir. Yerel GPU testleri AMD Radeon iGPU'da yapıldı; diğer aileler henüz doğrulanmadı. EXE'de Windows Authenticode imzası yoktur; güncelleme paketleri ayrı Ed25519 imzası kullanır. Popy kodu Apache-2.0; bağımlılık bildirimleri `DEPENDENCIES.md`, `licenses/` ve `THIRD_PARTY.md` içindedir.