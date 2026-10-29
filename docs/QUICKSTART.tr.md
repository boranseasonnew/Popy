# Popy 0.2 ön sürüm

1. ZIP dosyasını yazabildiğin bir klasöre tamamen çıkar. EXE'yi ZIP içinden açma.
2. İlk kullanımda `Araclari Kur.cmd` dosyasını çalıştır. FFmpeg, ncnn ve model dosyaları yazarlarının GitHub sürümlerinden indirilir; arşivlerin SHA-256 değerleri denetlenir. Rust veya Visual Studio kurulmaz. İnternet yalnızca bu kurulum için gerekir.
3. `popy-desktop.exe` dosyasını aç.
4. Video ekle. Gerçek çekim için Genel, çizim için Anime seç. Genel model iGPU'da oldukça yavaştır.
5. İstersen Otomatik renk'yi aç. Bu seçenek ton/kontrast/doygunluk düzeltir; siyah beyaz görüntüye yeni renk eklemez.
6. Önce `3 sn önizleme` dene. Önce/sonra kaydırıcısı ilk kareyi gösterir; hareketli sonucu `Videoyu aç` ile incele.
7. `Kuyruğu işle` ile hazır videoları sırayla dışa aktar. Ayarlar sıraya alırken kaydedilir. Her iş yeni bir klasöre `enhanced.mp4`, karşılaştırma ve rapor yazar.

Videolar bilgisayarında kalır. İşleme sırasında buluta gönderilmez. Kaynak dosya üzerine yazılmaz. `Durdur`, çalışan işi iptal eder ve sıradaki işleri bekletir; yeniden başlatmak için `Kuyruğu işle` kullan. Pencere kapanırken çalışan iş varsa seçenek sunulur. Yarıda kalan işler sıfırdan tekrar denenir; kaldığı kareden devam henüz yoktur.

Bu paket **Windows x64, imzasız bir ön sürümdür**. Vulkan destekli güncel ekran kartı sürücüsü gerekir. Tek AMD Radeon iGPU'da test edilmiştir; RTX 30/40/50, ayrı AMD ve Intel kartlarında doğrulanmış uyumluluk iddiası yoktur.

Girdi: tek H.264 akışı, sabit 1–120 FPS, progressive 8-bit yuv420p, açık BT.709 limited SDR etiketi, kare piksel, sıfıra yakın başlangıç. En fazla bir AAC ses akışı. HDR, VFR, döndürme, interlaced ve diğer kodekler henüz desteklenmez. Renk etiketi bulunmayan dosyalar belirsiz dönüşüm yapılmadan reddedilir. Çıktı 2×; modelin yerel boyut sınırı 3840×2160. Genel model içeride 4× çalıştığı için en fazla 960×540 girdi, anime modeli 1920×1080 girdi kabul eder. Süre üst sınırı 6 saattir; bu sınır 6 saatlik işin test edildiği anlamına gelmez.

PNG kareleri küçük gruplar hâlinde yazılır ve grup tamamlanınca temizlenir. Geçici kayıpsız RGB segmentleri video süresiyle büyür. Uygulama boş disk alanını muhafazakâr bir tahminle kontrol eder; uzun videolar yüksek disk alanı gerektirebilir. Başarısız/iptal işlerin günlükleri ve geçici dosyaları hata incelemesi için korunur. Sonuç klasörünü inceleyip istemediğin işi kendin silebilirsin.

Bellek hatasında tile değerini 64'ten 32'ye düşürüp tekrar dene. GPU numarası ncnn'nin Vulkan sırasıdır; Windows Görev Yöneticisi sırasıyla aynı olmak zorunda değildir. Cihaz adları iş klasöründeki `logs/ai-*.stderr.log` dosyasında bulunur.

Popy henüz Topaz düzeyinde kalite doğrulaması, video oynatıcısı, FPS artırma, HDR, VFR dönüşümü, AI siyah beyaz renklendirme veya otomatik güncelleme sunmaz. Algısal kalite kapısı geçilmediği için kararlı 1.0 olarak yayınlanmaz.

Kaynak: https://github.com/boranseasonnew/Popy

Popy kodu Apache-2.0; bağımlılık bildirimleri `licenses/` ve `DEPENDENCIES.md` içindedir. FFmpeg, ncnn ve modeller bu ZIP'e dahil değildir; kendi lisanslarına tabidir. Ayrı indirilen araçları yeniden dağıtmadan önce lisans koşullarını incele.
