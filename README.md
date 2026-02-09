# 🧛 Vampire Survivors Clone - Rust & Bevy

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/bevy-0.18.0-blue.svg)](https://bevyengine.org/)
[![License](https://img.shields.io/badge/license-Educational-green.svg)](LICENSE)

> **Vampire Survivors tarzı bir hayatta kalma oyunu - Rust ve Bevy Engine ile geliştirilmiştir**

## 📖 Hakkında

Bu proje, Bevy oyun motoru kullanılarak Vampire Survivors tarzında bir hayatta kalma oyunu uygulamasıdır. Oyun, dalga bazlı düşman spawn sistemi, karakter gelişimi ve çeşitli silah sistemleri içermektedir.

**⚠️ Not:** Bu proje tamamen eğitim ve eğlence amaçlıdır. Aktif geliştirme aşamasındadır.

## ✨ Özellikler

### 🎯 Mevcut Özellikler

#### Oyuncu Sistemi
- **Hareket Kontrolleri**: WASD tuşları ile 8 yönlü hareket
- **Animasyonlu Karakterler**: Sprite tabanlı karakter animasyonları
- **Otomatik Kamera Takibi**: Yumuş kamera hareketi

#### Düşman Sistemi
- **Dinamik Spawn Sistemi**: Artan zorlukla otomatik düşman oluşturma
- **Akıllı AI**: Düşmanlar oyuncuyu takip eder ve kovalar
- **Zorluk Skalası**: Zamanla artan güç ve hız

#### Silah Sistemleri
- **Lazer Silahlar**: Özelleştirilebilir renk ve güç seviyeleri
- **Roket/Mermi Silahları**: Projektil tabanlı saldırı sistemi
- **Yakın Dövüş Silahları**: Kalkan ve benzeri oyuncuya bağlı silahlar
- **Otomatik Ateşleme**: Silahlar otomatik olarak ateş eder

#### İlerleme Sistemi
- **XP Toplama**: Yenilen düşmanlardan XP kazanımı
- **Seviye Atlama**: XP ile karakter seviye sistemi
- **Yükseltme Seçimi**: Her seviyede 3 rastgele silah yükseltmesi
- **Stat Geliştirme**: Silah hasarı, ateş hızı ve diğer özellikler

#### Oyun Durumları
- **Yükleme Ekranı**: Asset yükleme ve başlangıç
- **Aktif Oynanış**: Ana oyun döngüsü
- **Yükseltme Menüsü**: Seviye atlama silah seçimi
- **Oyun Bitti Ekranı**: Yeniden başlatma seçeneği ile

#### UI Sistemi
- **Skor Takibi**: Anlık skor gösterimi
- **XP Barı**: Görsel ilerleme göstergesi
- **Seviye Göstergesi**: Mevcut karakter seviyesi

#### Dünya Sistemi
- **Sonsuz Zemin**: Dinamik chunk tabanlı zemin oluşturma
- **Performans Optimizasyonu**: Uzaktaki chunk'ların temizlenmesi

## 🎮 Kontroller

| Tuş         | Fonksiyon                             |
|-------------|---------------------------------------|
| **W/A/S/D** | Karakter hareketi                     |
| **R**       | Oyunu yeniden başlat (Game Over'da)   |
| **Mouse**   | Yükseltme seçimi (Seviye atladığında) |
| **ESC**     | Oyundan çık                           |
| **C**       | XP' leri topla                        |


## 🛠️ Teknik Detaylar

### Teknoloji Stack
- **Dil**: Rust (Edition 2024)
- **Oyun Motoru**: Bevy 0.18.0
- **Fizik Motoru**: Rapier2D 0.28.0 (yeni eklendi!)
- **ECS**: Entity Component System mimarisi
- **Bağımlılıklar**:
  - `rand` - Rastgele sayı üretimi
  - `bevy_rapier2d` - 2D fizik simülasyonu

### Sistem Gereksinimleri
- **OS**: Windows 10/11, Linux, macOS
- **RAM**: 4GB minimum, 8GB önerilen
- **GPU**: OpenGL 3.3+ destekli herhangi bir GPU

## 📦 Kurulum ve Çalıştırma

### Gereksinimler
```bash
# Rust kurulumu (eğer yoksa)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Projeyi Klonlama ve Çalıştırma
```bash
# Repoyu klonlayın
git clone https://github.com/TunahanYavuz/VampireSurvivals_Game_Practice_Rust_Bevy.git
cd VampireSurvivals_Game_Practice_Rust_Bevy

# Geliştirme modunda çalıştırın
cargo run

# Release modunda çalıştırın (optimize edilmiş)
cargo run --release
```

### Build Profilleri
```toml
# Development: Hızlı derleme (opt-level 1)
# Dependencies: Yüksek optimizasyon (opt-level 3)
# Release: Tam optimizasyon (opt-level 3, LTO aktif)
```

## 📁 Proje Yapısı

```
gameDeveloping/
├── src/
│   ├── main.rs                 # Ana oyun döngüsü ve sistem kurulumu
│   └── plugins/
│       ├── player.rs           # Oyuncu hareketi ve davranışı
│       ├── enemy.rs            # Düşman spawn ve AI sistemi
│       ├── weapons.rs          # Silah sistemleri ve ateşleme
│       ├── weapon_stats.rs     # Silah konfigürasyonu ve stats
│       ├── weapon_upgrade.rs   # Yükseltme seçim sistemi
│       ├── timers.rs           # Oyun zamanlayıcıları ve spawn oranları
│       ├── aabb.rs             # Çarpışma algılama (AABB)
│       ├── game_state.rs       # Oyun durumu yönetimi
│       ├── score.rs            # Skor takibi ve UI
│       ├── ground.rs           # Zemin oluşturma sistemi
│       └── texture_handling.rs # Asset yönetimi
├── assets/                     # Oyun varlıkları
│   ├── sprites/               # Karakter ve düşman sprite'ları
│   ├── weapons/               # Silah grafikleri
│   └── ui/                    # Kullanıcı arayüzü öğeleri
├── Cargo.toml                 # Proje bağımlılıkları
└── README.md                  # Bu dosya
```

## 🎮 Oynanış

Dalga dalga gelen düşmanlara karşı hayatta kalmaya çalışın, hareket edin ve XP toplayın. Seviye atladıkça, silahlarınızı güçlendirmek için rastgele yükseltmeler arasından seçim yapın. Her yükseltme, silahlarınızı iyileştirir veya yeni silahlar ekler. Oyun, düşmanlar daha sık ve güçlü hale geldikçe giderek daha zorlayıcı hale gelir.

## 🚧 Geliştirme Aşamasında

Bu proje aktif olarak geliştirilmektedir. Gelecek sürümlerdeki olası iyileştirmeler şunları içerebilir:
- Ekstra silah tipleri
- Daha fazla düşman çeşidi
- Güçlendirici öğeler
- Ses efektleri ve müzik
- Görsel efektler iyileştirmeleri
- Boss karşılaşmaları
- Zorluk seviyeleri
- Başarım sistemi
- Kayıt/yükleme sistemi
- Çoklu oyuncu karakter seçenekleri

## 🎆 YENİ: Rapier Fizik Tabanlı Efekt Sistemi

Proje artık **Rapier** fizik motoru ile entegre gelişmiş efekt sistemi içeriyor!

### Özellikler:
- 🔥 **Fiziksel Parçacıklar**: Gerçekçi çarpışma, yerçekimi ve sürtünme
- 💥 **Patlama Efektleri**: Radyal kuvvet dalgaları
- ⚡ **Çarpışma Tetikleyicileri**: Otomatik efekt oluşturma
- 🎨 **Hazır Preset'ler**: Ateş, kıvılcım, duman, enkaz efektleri

### Kullanım:

Detaylı dokümantasyon için: [**RAPIER_EFFECTS_GUIDE.md**](RAPIER_EFFECTS_GUIDE.md)

**Hızlı Başlangıç:**
```rust
use crate::plugins::rapier_effects::{spawn_explosion_effect, presets};

// Patlama efekti oluştur
let config = presets::fire_particle_config(&asset_server);
spawn_explosion_effect(
    &mut commands,
    &asset_server,
    position,
    150.0,   // yarıçap
    3000.0,  // kuvvet
    24,      // parçacık sayısı
    &config,
);
```

**Demo Mod:**
Oyun içinde demo efektleri test etmek için:
- `E` - Patlama efekti
- `F` - Ateş parçacıkları
- `S` - Kıvılcım efekti
- `D` - Duman efekti
- `R` - Enkaz parçacıkları
- `Space` - Fiziksel top (çarpışma efektli)

## 🤝 Katkıda Bulunma

Bu kişisel bir öğrenme projesi olduğu için şu anda katkı kabul edilmemektedir. Ancak fork'layıp kendi fikirlerinizi deneyebilirsiniz!

## 📄 Lisans

Bu proje yalnızca eğitim ve eğlence amaçlıdır.

## 📬 İletişim

**Proje Sahibi**: [TunahanYavuz](https://github.com/TunahanYavuz)

---

⭐ Projeyi beğendiyseniz bir yıldız bırakmayı unutmayın!

*Rust ve Bevy ile ❤️ ile yapılmıştır*
