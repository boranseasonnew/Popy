# Yerel araştırma araçları ve model kaynağı

0.2 ön sürüm ZIP'i Popy CLI/masaüstü EXE'lerini, kurulum scriptini ve lisans bildirimlerini içerir. FFmpeg, ncnn ve model ağırlıkları ZIP'e gömülmez; ilk kurulumda aşağıdaki kaynaklardan ayrıca indirilir. `.tools/`, generated fixtures, `dist/` ve `runs/` Git dışında. Tam çevrimdışı araç/model paketi için yeniden dağıtım koşulları ayrıca incelenmeli. Kullanılan FFmpeg derlemesi GPL özellikleri içeriyor.

| Bileşen | Kaynak | Sabitleme |
|---|---|---|
| Rust | https://static.rust-lang.org/rustup/ | Resmî kurucu SHA-256 kontrolü; bu makinede rustc 1.99.1 |
| FFmpeg / ffprobe | https://github.com/GyanD/codexffmpeg/releases/tag/9.0.2 | Arşiv SHA-256 bootstrap.ps1 içinde |
| Real-ESRGAN ncnn ve modeller | https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0 | 20220424 Windows arşivi; SHA-256 bootstrap.ps1 içinde |
| Rust bağımlılıkları | crates.io | Cargo.lock sürüm ve checksum kaydı |
| egui / eframe | https://github.com/emilk/egui | 0.37.0; Cargo.lock |
| rfd | https://github.com/PolyMeilex/rfd | 0.17.2; Cargo.lock |

Real-ESRGAN ncnn uygulamasının lisansı MIT, Real-ESRGAN ana kodunun lisansı BSD-3-Clause, ncnn BSD-3-Clause ve kendi üçüncü taraf bildirimleri. Seçilen model dosyaları yazarın resmî sürüm arşivinden; her işte dosya hash'leri kaydedilir. Ağırlıkların yeniden dağıtımı ve tüm bildirim zinciri paketleme aşamasından önce ayrıca incelenecek. Rastgele üçüncü taraf model dönüştürmeleri indirilmedi.

- https://github.com/xinntao/Real-ESRGAN-ncnn-vulkan/blob/master/LICENSE
- https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE
- https://github.com/Tencent/ncnn/blob/master/LICENSE.txt
- https://ffmpeg.org/legal.html

Arşiv hash'leri indirilen baytları tekrar tanımlar; bağımsız yayıncı kimlik doğrulaması olarak sunulmaz. Hiçbir video dış servise yüklenmez.

Masaüstü EXE'si Rust bağımlılıklarını ve egui'nin varsayılan fontlarını içerir. `scripts/package.ps1`, Windows için çözümlenen bağımlılık kaynaklarının lisans metinlerini `licenses/` altına ve envanterini `DEPENDENCIES.md` dosyasına kopyalar. Build bağımlılıkları da dahildir. Fontların OFL-1.1, Ubuntu Font License 1.0 ve ilgili bildirimleri ayrıca korunur. Windows EXE'leri statik MSVC CRT ile derlenir; Vulkan/OpenGL ve sistem DLL'leri işletim sistemi/sürücüden gelir.

Bazı crate arşivlerinde bulunmayan kök lisans metinleri resmî upstream commitlerinden `third_party/licenses/` altına tamamlandı. Kaynak URL'leri ve SHA-256 değerleri `SOURCES.json` içinde, pakette de korunur. egui ailesinin erişilemeyen crate VCS kaydı için resmî 0.37.0 etiketinin commit'i kullanıldı; eframe için erişilebilen kendi VCS commit'i kullanıldı. Metinler değiştirilmedi. Lisans dosyası adı eşleştirmesi Türkçe sistemlerde `I/i` farkından etkilenmemesi için invariant büyük harf kullanır.

Kendi sentetik testleri FFmpeg `testsrc2` ve `sine` kaynaklarından üretildi; indirilen demo/anime videosu kalite raporunda kullanılmadı.
