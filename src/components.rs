use bevy::{prelude::*, render::render_resource::ShaderType};
use bytemuck::{Pod, Zeroable};

#[derive(Component)]
pub struct Particle;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticleType(pub usize);

impl ParticleType {
    pub fn index(&self) -> usize {
        self.0
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Velocity(pub Vec2);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
pub struct GpuParticle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub kind: u32,
    pub padding: u32,
}
