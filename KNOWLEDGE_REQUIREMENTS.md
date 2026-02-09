# Bevy + Rapier ile Efekt Arayüzü Oluşturmak İçin Gerekli Bilgiler

## 📚 Temel Bilgi Gereksinimleri

Bu döküman, Bevy ve Rapier kullanarak efekt üreten bir arayüz oluşturmak için neleri bilmeniz gerektiğini açıklar.

---

## 1. 🦀 Rust Programlama Temelleri

### Bilmeniz Gerekenler:
- **Ownership & Borrowing**: Rust'ın bellek yönetimi
- **Traits & Generics**: Trait sistemini ve generic programlamayı anlama
- **Structs & Enums**: Veri yapılarını tanımlama
- **Pattern Matching**: `match` ve `if let` kullanımı
- **Closures & Iterators**: Fonksiyonel programlama özellikleri

### Örnek:
```rust
// Ownership örneği
let particle_config = PhysicsParticleConfig::default();
spawn_physics_particle(&mut commands, &asset_server, position, &particle_config);
// config hala kullanılabilir çünkü referans verdik (&)

// Trait örneği
#[derive(Component)]
struct PhysicsParticle {
    lifetime: Timer,
}
```

---

## 2. 🎮 Bevy Entity Component System (ECS)

### Bilmeniz Gerekenler:

#### **Entity (Varlık)**
Oyundaki her nesne bir entity'dir (parçacık, silah, düşman, vb.)

#### **Component (Bileşen)**
Entity'lere eklenen veri parçaları
```rust
#[derive(Component)]
struct PhysicsParticle {
    velocity: Vec2,
    lifetime: Timer,
}
```

#### **System (Sistem)**
Component'leri işleyen fonksiyonlar
```rust
fn update_particles(
    time: Res<Time>,
    mut particles: Query<(&mut PhysicsParticle, &mut Transform)>,
) {
    for (mut particle, mut transform) in particles.iter_mut() {
        // Parçacığı güncelle
    }
}
```

#### **Query (Sorgu)**
Entity'leri filtrelemek için kullanılır
```rust
Query<(&Transform, &Velocity)>  // Transform ve Velocity olan tüm entity'ler
Query<&mut Sprite, With<Particle>>  // Particle olan tüm Sprite'lar
```

#### **Commands (Komutlar)**
Entity oluşturma/silme için
```rust
commands.spawn((
    Sprite { ... },
    Transform::default(),
    PhysicsParticle::default(),
));
```

#### **Resources (Kaynaklar)**
Global veri
```rust
time: Res<Time>,
asset_server: Res<AssetServer>,
```

### Öğrenme Kaynakları:
- [Bevy Quick Start](https://bevyengine.org/learn/quick-start/getting-started/)
- [Bevy ECS Cheatbook](https://bevy-cheatbook.github.io/)

---

## 3. ⚛️ Rapier Fizik Motoru

### Bilmeniz Gerekenler:

#### **RigidBody (Katı Cisim)**
Fiziksel nesnelerin türü:
```rust
RigidBody::Dynamic    // Hareket eden (parçacıklar, düşmanlar)
RigidBody::Fixed      // Sabit (zemin, duvarlar)
RigidBody::Kinematic  // Kontrollü hareket (oyuncu)
```

#### **Collider (Çarpışma Şekli)**
Fiziksel şeklin tanımı:
```rust
Collider::ball(10.0)           // Top şeklinde
Collider::cuboid(50.0, 50.0)  // Kutu şeklinde
```

#### **Velocity (Hız)**
Nesnenin hızı:
```rust
Velocity {
    linvel: Vec2::new(100.0, 50.0),  // Doğrusal hız
    angvel: 2.0,                      // Açısal hız (dönme)
}
```

#### **Fiziksel Özellikler**
```rust
Restitution::coefficient(0.8)  // Zıplama (0.0-1.0)
Friction::coefficient(0.5)     // Sürtünme (0.0-1.0)
GravityScale(1.0)              // Yerçekimi çarpanı
ColliderMassProperties::Mass(2.0)  // Kütle
```

#### **Collision Events (Çarpışma Olayları)**
```rust
fn handle_collisions(
    mut collision_events: EventReader<CollisionEvent>,
) {
    for event in collision_events.read() {
        match event {
            CollisionEvent::Started(e1, e2, _) => {
                // Çarpışma başladı
            }
            CollisionEvent::Stopped(e1, e2, _) => {
                // Çarpışma bitti
            }
        }
    }
}
```

### Öğrenme Kaynakları:
- [Rapier2D User Guide](https://rapier.rs/docs/user_guides/rust/getting_started)
- [Bevy Rapier Plugin](https://github.com/dimforge/bevy_rapier)

---

## 4. 🎨 Parçacık Efekt Sistemleri

### Temel Kavramlar:

#### **Parçacık Yaşam Döngüsü**
1. **Spawn (Oluşturma)**: Parçacık oluşturulur
2. **Update (Güncelleme)**: Her frame'de güncellenir
3. **Cleanup (Temizleme)**: Ömrü bitince silinir

```rust
// Spawn
commands.spawn(ParticleBundle { ... });

// Update
particle.lifetime.tick(time.delta());
let progress = particle.lifetime.fraction();  // 0.0 -> 1.0

// Cleanup
if particle.lifetime.finished() {
    commands.entity(entity).despawn();
}
```

#### **Interpolasyon**
Başlangıç ve bitiş değerleri arasında geçiş:
```rust
// Renk interpolasyonu
let current_color = start_color.lerp(end_color, progress);

// Ölçek interpolasyonu
let scale = start_scale + (end_scale - start_scale) * progress;
```

#### **Emitter (Yayıcı)**
Sürekli parçacık üreten sistem:
```rust
#[derive(Component)]
struct ParticleEmitter {
    spawn_timer: Timer,
    particles_per_spawn: u32,
}
```

### Efekt Tipleri:

1. **Nokta Efektler**: Tek noktadan yayılma
2. **Çizgisel Efektler**: Çizgi boyunca dağıtma
3. **Dairesel Efektler**: Çember üzerinde yerleştirme
4. **İz Efektleri**: Hareket eden nesnelerin arkasında

---

## 5. 🎯 Proje Yapısı ve Plugin Sistemi

### Plugin Oluşturma:
```rust
pub struct MyEffectPlugin;

impl Plugin for MyEffectPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                spawn_particles,
                update_particles,
                cleanup_particles,
            ).run_if(in_state(GameState::Playing)));
    }
}
```

### Ana Uygulamaya Ekleme:
```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin)
        .add_plugins(MyEffectPlugin)
        .run();
}
```

---

## 6. 🔧 Pratik Örnekler

### Örnek 1: Basit Parçacık
```rust
commands.spawn((
    Sprite { ... },
    Transform::from_translation(position),
    RigidBody::Dynamic,
    Velocity::linear(Vec2::new(100.0, 100.0)),
    Collider::ball(5.0),
    PhysicsParticle {
        lifetime: Timer::from_seconds(2.0, TimerMode::Once),
    },
));
```

### Örnek 2: Patlama Efekti
```rust
// Merkez noktadan radyal olarak parçacıklar fırlat
for i in 0..32 {
    let angle = (i as f32 / 32.0) * TAU;
    let direction = Vec2::new(angle.cos(), angle.sin());
    let velocity = direction * 200.0;
    
    spawn_particle(position, velocity);
}
```

### Örnek 3: Çarpışma Efekti
```rust
fn on_collision(
    mut collision_events: EventReader<CollisionEvent>,
    mut commands: Commands,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Çarpışma noktasında efekt oluştur
            spawn_impact_effect(&mut commands, position);
        }
    }
}
```

---

## 7. 🎓 Öğrenme Yol Haritası

### Aşama 1: Temel Rust (1-2 hafta)
- [ ] Ownership & borrowing
- [ ] Structs & enums
- [ ] Traits
- [ ] Basic error handling

### Aşama 2: Bevy Temelleri (1 hafta)
- [ ] Entity-Component-System
- [ ] Basit sprite rendering
- [ ] Input handling
- [ ] Timer kullanımı

### Aşama 3: Rapier Fizik (1 hafta)
- [ ] RigidBody types
- [ ] Colliders
- [ ] Velocity & forces
- [ ] Collision events

### Aşama 4: Parçacık Sistemleri (1-2 hafta)
- [ ] Basit parçacık spawn
- [ ] Lifetime management
- [ ] Interpolation
- [ ] Emitter systems

### Aşama 5: İleri Seviye (2+ hafta)
- [ ] Performans optimizasyonu
- [ ] Karmaşık efekt kombinasyonları
- [ ] Özel shader'lar
- [ ] Texture atlases

---

## 8. 🛠️ Gerekli Araçlar

### Development Environment:
- **Rust**: `rustup` ile en güncel sürüm
- **IDE**: VS Code + rust-analyzer veya IntelliJ IDEA + Rust plugin
- **Build Tools**: Cargo (Rust ile gelir)

### Sistem Gereksinimleri:
- **Grafik**: OpenGL 3.3+ destekli GPU
- **RAM**: En az 4GB (geliştirme için 8GB+ önerilir)
- **OS**: Windows, macOS, veya Linux

---

## 9. 📚 Kaynaklar

### Dokümantasyon:
- [Bevy Documentation](https://docs.rs/bevy/latest/bevy/)
- [Rapier2D Documentation](https://docs.rs/bevy_rapier2d/latest/bevy_rapier2d/)
- [Rust Book](https://doc.rust-lang.org/book/)

### Örnek Projeler:
- Bu proje: `RAPIER_EFFECTS_GUIDE.md`
- [Bevy Examples](https://github.com/bevyengine/bevy/tree/main/examples)
- [Rapier Examples](https://github.com/dimforge/bevy_rapier/tree/master/bevy_rapier2d/examples)

### Community:
- [Bevy Discord](https://discord.gg/bevy)
- [Rust GameDev Discord](https://discord.gg/yNtPTb2)
- [r/rust_gamedev](https://www.reddit.com/r/rust_gamedev/)

---

## 10. ✅ Kontrol Listesi

Başlamadan önce şunları kontrol edin:

- [ ] Rust kurulu ve çalışıyor (`rustc --version`)
- [ ] Cargo çalışıyor (`cargo --version`)
- [ ] Basit bir "Hello World" Bevy uygulaması çalıştırabiliyorum
- [ ] ECS kavramlarını anlıyorum (Entity, Component, System)
- [ ] Fizik kavramlarını biliyorum (velocity, collision, forces)
- [ ] Git kullanımını biliyorum (versiyon kontrolü için)

---

## 💡 İpuçları

1. **Küçük Başlayın**: Önce basit bir parçacık spawn edin, sonra karmaşıklaştırın
2. **Debug Render Kullanın**: Fizik collision'ları görmek için debug render açın
3. **Performa Dikkat**: Aynı anda çok fazla parçacık performansı düşürür
4. **Dokümantasyonu Okuyun**: Rust ve Bevy dokümantasyonu çok iyi
5. **Örnekleri İnceleyin**: Bevy examples klasörü altın değerinde

---

## 🎯 Sonuç

Bevy + Rapier ile efekt arayüzü oluşturmak için:

1. **Rust temellerini** öğrenin
2. **Bevy ECS** sistemini anlayın
3. **Rapier fizik** motorunu kavrayın
4. **Parçacık sistemleri** hakkında bilgi edinin
5. **Pratik yapın** - küçük projelerle başlayın

Bu projede bulunan `rapier_effects.rs` ve `rapier_effects_demo.rs` dosyaları, tam çalışan örnekler içeriyor. Bu dosyaları inceleyerek nasıl çalıştıklarını öğrenebilirsiniz.

**Başarılar!** 🚀
