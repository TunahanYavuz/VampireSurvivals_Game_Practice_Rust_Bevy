use bevy::math::Vec3;
use bevy::prelude::Component;

#[derive(Component, Clone)]
pub struct AABB{
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub width: f32,
    pub height: f32,
}
impl AABB {
    pub fn change_point(&mut self, vec: Vec3){
        self.min_y = vec.y - self.height / 2.0;
        self.max_y = vec.y + self.height / 2.0;
        self.min_x = vec.x - self.width / 2.0;
        self.max_x = vec.x + self.width / 2.0;
    }
    pub fn aabb_intersects(a: &AABB, b: &AABB) -> bool {
        !(a.min_x > b.max_x || a.max_x < b.min_x || a.min_y > b.max_y || a.max_y < b.min_y)
    }
    pub fn self_aabb_intersects(&self, b: &AABB) -> bool {
        !(self.min_x > b.max_x || self.max_x < b.min_x || self.min_y > b.max_y || self.max_y < b.min_y)
    }
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x+5. >= self.min_x
            && point.x-5. <= self.max_x
            && point.y+5. >= self.min_y
            && point.y-5. <= self.max_y
    }
}