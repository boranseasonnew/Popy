# Yerel araştırma araçları ve model kaynağı

Bu envanter dağıtım onayı değildir. `.tools/`, generated fixtures ve `runs/` Git kapsamı dışında; Popy exe'si üçüncü taraf programları içine gömmez. Kullanılan yerel FFmpeg derlemesi GPL özellikleri içeriyor. Herkese dağıtılan installer için planlanan LGPL/GPL uyumu ayrıca yapılmalı.

| Bileşen | Kaynak | Sabitleme |
|---|---|---|
| Rust | https://static.rust-lang.org/rustup/ | Resmî kurucu SHA-256 kontrolü; bu makinede rustc 1.99.1 |
| FFmpeg / ffprobe | https://github.com/GyanD/codexffmpeg/releases/tag/9.0.2 | Arşiv SHA-256 bootstrap.ps1 içinde |
| Real-ESRGAN ncnn ve modeller | https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0 | 20220424 Windows arşivi; SHA-256 bootstrap.ps1 içinde |
| Rust bağımlılıkları | crates.io | Cargo.lock sürüm ve checksum kaydı |

Real-ESRGAN ncnn uygulamasının lisansı MIT, Real-ESRGAN ana kodunun lisansı BSD-3-Clause, ncnn BSD-3-Clause ve kendi üçüncü taraf bildirimleri. Seçilen model dosyaları yazarın resmî sürüm arşivinden; her işte dosya hash'leri kaydedilir. Ağırlıkların yeniden dağıtımı ve tüm bildirim zinciri paketleme aşamasından önce ayrıca incelenecek. Rastgele üçüncü taraf model dönüştürmeleri indirilmedi.

- https://github.com/xinntao/Real-ESRGAN-ncnn-vulkan/blob/master/LICENSE
- https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE
- https://github.com/Tencent/ncnn/blob/master/LICENSE.txt
- https://ffmpeg.org/legal.html

Arşiv hash'leri indirilen baytları tekrar tanımlar; bağımsız yayıncı kimlik doğrulaması olarak sunulmaz. Hiçbir video dış servise yüklenmez.

Kendi sentetik testleri FFmpeg `testsrc2` ve `sine` kaynaklarından üretildi; indirilen demo/anime videosu kalite raporunda kullanılmadı.
