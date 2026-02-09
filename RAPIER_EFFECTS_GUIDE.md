# Bevy + Rapier Efekt Arayüzü Kullanım Kılavuzu

## 📚 İçindekiler
1. [Giriş](#giriş)
2. [Temel Kavramlar](#temel-kavramlar)
3. [Kurulum ve Yapılandırma](#kurulum-ve-yapılandırma)
4. [Kullanım Örnekleri](#kullanım-örnekleri)
5. [API Referansı](#api-referansı)
6. [İleri Düzey Özellikler](#i̇leri-düzey-özellikler)

## Giriş

Bu proje, **Bevy** oyun motoru ve **Rapier** fizik motoru kullanarak gelişmiş efekt sistemleri oluşturmanıza olanak tanır. Rapier'in sağladığı gerçekçi fizik simülasyonu sayesinde parçacıklarınız şu özelliklere sahip olur:

- ✅ Gerçekçi çarpışma algılama
- ✅ Yerçekimi, sürtünme, elastikiyet
- ✅ Kuvvet dalgaları ve patlamalar
- ✅ Fiziksel etkileşimler

## Temel Kavramlar

### 1. Fiziksel Parçacıklar (PhysicsParticle)

Normal parçacıklardan farklı olarak, fiziksel parçacıklar Rapier'in `RigidBody` sistemi ile çalışır. Bu sayede:
- Çarpışma algılama
- Yerçekimi etkisi
- Elastik ve plastik çarpışmalar
- Sürtünme ve hava direnci
- Momentum aktarımı

gibi fiziksel özellikler otomatik olarak simüle edilir.

### 2. Patlama Dalgaları (ExplosionWave)

Belirli bir merkez noktadan radyal olarak kuvvet uygulayan efektlerdir. Çevredeki tüm fiziksel cisimleri iter.

### 3. Çarpışma Efektleri (CollisionEffectTrigger)

Bir nesne başka bir nesneyle çarpıştığında otomatik olarak efekt oluşturan sistem.

## Kurulum ve Yapılandırma

### Gerekli Bağımlılıklar

`Cargo.toml` dosyanıza şunlar eklenmiştir:
```toml
[dependencies]
bevy = { version = "0.18.0" }
bevy_rapier2d = { version = "0.28.0", features = ["debug-render"] }
```

### Plugin Ekleme

`RapierEffectsPlugin` otomatik olarak `main.rs`'e eklenmiştir:
```rust
.add_plugins(RapierEffectsPlugin)
```

## Kullanım Örnekleri

### Örnek 1: Basit Fiziksel Parçacık Oluşturma

```rust
use crate::plugins::rapier_effects::{spawn_physics_particle, PhysicsParticleConfig};

fn spawn_particles_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let config = PhysicsParticleConfig {
        lifetime: 2.0,
        start_color: Color::srgb(1.0, 0.5, 0.0),
        end_color: Color::srgba(1.0, 0.0, 0.0, 0.0),
        velocity_min: Vec2::new(-50.0, 50.0),
        velocity_max: Vec2::new(50.0, 150.0),
        restitution: 0.7,  // Zıplamayı kontrol eder
        friction: 0.5,      // Sürtünmeyi kontrol eder
        gravity_scale: 1.0, // Normal yerçekimi
        ..default()
    };
    
    let position = Vec3::new(0.0, 0.0, 0.0);
    spawn_physics_particle(&mut commands, &asset_server, position, &config);
}
```

### Örnek 2: Patlama Efekti

```rust
use crate::plugins::rapier_effects::{spawn_explosion_effect, PhysicsParticleConfig, presets};

fn create_explosion(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let position = Vec3::new(100.0, 100.0, 0.0);
    let radius = 200.0;      // Patlama yarıçapı
    let force = 5000.0;      // Uygulanan kuvvet
    let particle_count = 32; // Parçacık sayısı
    
    // Ateş efekti preset kullanarak
    let config = presets::fire_particle_config(&asset_server);
    
    spawn_explosion_effect(
        &mut commands,
        &asset_server,
        position,
        radius,
        force,
        particle_count,
        &config,
    );
}
```

### Örnek 3: Çarpışma Efekti Ekleme

```rust
use crate::plugins::rapier_effects::CollisionEffectTrigger;
use bevy_rapier2d::prelude::*;

fn spawn_projectile_with_collision_effect(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        // Normal sprite ve transform
        Sprite {
            image: asset_server.load("projectile.png"),
            ..default()
        },
        Transform::from_translation(Vec3::ZERO),
        // Rapier bileşenleri
        RigidBody::Dynamic,
        Collider::ball(10.0),
        Velocity::linear(Vec2::new(200.0, 0.0)),
        // Çarpışma efekt tetikleyici
        CollisionEffectTrigger {
            particle_count: 10,
            color: Color::srgb(1.0, 1.0, 0.0), // Sarı kıvılcımlar
            enabled: true,
        },
    ));
}
```

### Örnek 4: Hazır Preset'leri Kullanma

```rust
use crate::plugins::rapier_effects::presets::*;

// Ateş parçacıkları
let fire_config = fire_particle_config(&asset_server);
spawn_physics_particle(&mut commands, &asset_server, position, &fire_config);

// Kıvılcımlar
let spark_config = spark_particle_config(&asset_server);
spawn_physics_particle(&mut commands, &asset_server, position, &spark_config);

// Duman
let smoke_config = smoke_particle_config(&asset_server);
spawn_physics_particle(&mut commands, &asset_server, position, &smoke_config);

// Enkaz
let debris_config = debris_particle_config(&asset_server);
spawn_physics_particle(&mut commands, &asset_server, position, &debris_config);
```

## API Referansı

### PhysicsParticleConfig

Fiziksel parçacık davranışını kontrol eden konfigürasyon yapısı.

| Alan | Tip | Açıklama |
|------|-----|----------|
| `lifetime` | `f32` | Parçacığın yaşam süresi (saniye) |
| `velocity_min` | `Vec2` | Minimum başlangıç hızı |
| `velocity_max` | `Vec2` | Maximum başlangıç hızı |
| `start_scale` | `f32` | Başlangıç boyutu |
| `end_scale` | `f32` | Bitiş boyutu |
| `start_color` | `Color` | Başlangıç rengi |
| `end_color` | `Color` | Bitiş rengi |
| `mass` | `f32` | Kütle (fiziksel etkileşimleri etkiler) |
| `restitution` | `f32` | Zıplama katsayısı (0.0-1.0) |
| `friction` | `f32` | Sürtünme katsayısı (0.0-1.0) |
| `gravity_scale` | `f32` | Yerçekimi çarpanı (1.0 = normal) |

### spawn_physics_particle()

```rust
pub fn spawn_physics_particle(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    config: &PhysicsParticleConfig,
)
```

Tek bir fiziksel parçacık oluşturur.

### spawn_explosion_effect()

```rust
pub fn spawn_explosion_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
    force: f32,
    particle_count: u32,
    config: &PhysicsParticleConfig,
)
```

Patlama efekti oluşturur - radyal kuvvet dalgası ve parçacıklar.

### spawn_collision_effect()

```rust
pub fn spawn_collision_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    velocity: Vec2,
    config: &PhysicsParticleConfig,
    count: u32,
)
```

Çarpışma noktasında parçacık efekti oluşturur.

## İleri Düzey Özellikler

### 1. Özel Fizik Parametreleri

```rust
let config = PhysicsParticleConfig {
    mass: 5.0,              // Ağır parçacıklar
    restitution: 0.9,       // Çok zıplayan
    friction: 0.1,          // Düşük sürtünme (kaygan)
    gravity_scale: -1.0,    // Ters yerçekimi (yukarı)
    ..default()
};
```

### 2. Çoklu Patlama Efektleri

```rust
// Zincir patlama
for i in 0..5 {
    let delay = Timer::from_seconds(i as f32 * 0.2, TimerMode::Once);
    // Timer ile geciktirilmiş patlama
}
```

### 3. Özel Çarpışma Filtreleri

Rapier'in collision groups özelliğini kullanarak:
```rust
.insert(CollisionGroups::new(
    Group::GROUP_1,  // Bu nesne hangi gruba ait
    Group::GROUP_2,  // Hangi gruplarla çarpışacak
))
```

### 4. Debug Rendering

Fizik çarpışma şekillerini görmek için:
```rust
// RapierDebugRenderPlugin zaten aktif
// F1 tuşu ile açıp kapatabilirsiniz (default Rapier davranışı)
```

## Performans İpuçları

1. **Parçacık Sayısını Sınırlayın**: Aynı anda çok fazla fiziksel parçacık performansı düşürür
   - Önerilen: Maksimum 200-300 aktif parçacık

2. **Lifetime'ı Kısa Tutun**: Gereksiz uzun süre yaşayan parçacıklardan kaçının

3. **Collider Boyutlarını Optimize Edin**: Daha küçük collider'lar daha hızlıdır

4. **Sensor Collider Kullanımı**: Fiziksel çarpışma yerine sadece algılama için:
   ```rust
   .insert(Sensor)
   ```

## Bilmeniz Gerekenler

### Temel Gereksinimler
1. **Bevy ECS Bilgisi**: Component, System, Query yapıları
2. **Rapier Fizik Temelleri**: RigidBody, Collider, Velocity
3. **Rust Programlama**: Ownership, borrowing, traits

### Önerilen Öğrenme Yolu
1. ✅ Basit parçacık oluşturma ile başlayın
2. ✅ Preset'leri kullanarak deneyimleyin
3. ✅ Patlama efektleri ekleyin
4. ✅ Çarpışma efektlerini entegre edin
5. ✅ Özel konfigürasyonlar oluşturun

### Yararlı Kaynaklar
- [Bevy Dokumentasyonu](https://docs.rs/bevy/latest/bevy/)
- [Rapier2D Dokumentasyonu](https://docs.rs/bevy_rapier2d/latest/bevy_rapier2d/)
- [Bevy Cheatbook](https://bevy-cheatbook.github.io/)

## Örnek Proje Entegrasyonu

Mevcut Vampire Survivors klonunuza eklemek için:

```rust
// weapon_effects.rs dosyanızda
use crate::plugins::rapier_effects::{spawn_explosion_effect, presets};

pub fn weapon_hit_enemy(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // ... diğer parametreler
) {
    // Düşman öldüğünde patlama efekti
    if enemy_health <= 0.0 {
        let config = presets::fire_particle_config(&asset_server);
        spawn_explosion_effect(
            &mut commands,
            &asset_server,
            enemy_position,
            50.0,  // radius
            2000.0, // force
            20,    // particle count
            &config,
        );
    }
}
```

## Sorun Giderme

### "Parçacıklar görünmüyor"
- Asset yollarını kontrol edin: `assets/effects/particle.png`
- Z-index değerini artırın: `transform.translation.z`

### "Performans düşük"
- Parçacık sayısını azaltın
- Lifetime'ları kısaltın
- Collider boyutlarını optimize edin

### "Çarpışmalar algılanmıyor"
- Collider eklendiğinden emin olun
- CollisionEffectTrigger'ın enabled olduğunu kontrol edin
- Collision groups ayarlarını inceleyin

---

**Başarılar!** 🎮✨

Sorularınız için: [GitHub Issues](https://github.com/TunahanYavuz/VampireSurvivals_Game_Practice_Rust_Bevy/issues)
