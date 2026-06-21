use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, Node, RenderGraphExt},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        Render, RenderApp, RenderSystems,
    },
};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use rand::Rng;

use crate::{SimulationParams, components::GpuParticle};

pub struct GpuPhysicsPlugin;

impl Plugin for GpuPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GpuSimulationParams>()
           .add_systems(Update, update_gpu_params);
        
        app.add_plugins(ExtractResourcePlugin::<GpuSimulationParams>::default());
        app.add_plugins(ExtractResourcePlugin::<crate::SimulationParams>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                Render,
                (
                    prepare_gpu_buffers.in_set(RenderSystems::Prepare),
                    queue_bind_group.in_set(RenderSystems::Queue),
                ),
            )
            .add_render_graph_node::<PhysicsComputeNode>(
                bevy::core_pipeline::core_2d::graph::Core2d,
                PhysicsComputeLabel,
            )
            .add_render_graph_edge(
                bevy::core_pipeline::core_2d::graph::Core2d,
                PhysicsComputeLabel,
                bevy::core_pipeline::core_2d::graph::Node2d::StartMainPass,
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<PhysicsPipeline>()
            .init_resource::<SimulationParamsBuffer>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PhysicsComputeLabel;

#[derive(Resource, Default)]
pub struct SimulationParamsBuffer(pub UniformBuffer<GpuSimulationParams>);

#[derive(Resource, Clone, Copy, Pod, Zeroable, ExtractResource, ShaderType)]
#[repr(C)]
pub struct GpuSimulationParams {
    pub interaction_matrix: [Vec4; 25],
    pub colors: [Vec4; 10],
    pub attraction_strength: f32,
    pub time_scale: f32,
    pub min_dist: f32,
    pub interaction_radius: f32,
    pub density_limit: f32,
    pub dampening: f32,
    pub particle_count: u32,
    pub region_size_x: f32,
    pub region_size_y: f32,
    pub scale: f32,
    pub global_gravity: f32,
    pub infinite_space: u32,
}

impl Default for GpuSimulationParams {
    fn default() -> Self {
        Self {
            interaction_matrix: [Vec4::ZERO; 25],
            colors: [Vec4::ZERO; 10],
            attraction_strength: 0.0,
            time_scale: 0.0,
            min_dist: 0.0,
            interaction_radius: 0.0,
            density_limit: 0.0,
            dampening: 0.0,
            particle_count: 0,
            region_size_x: 0.0,
            region_size_y: 0.0,
            scale: 0.0,
            global_gravity: 0.0,
            infinite_space: 0,
        }
    }
}

pub fn update_gpu_params(
    params: Res<SimulationParams>,
    mut gpu_params: ResMut<GpuSimulationParams>,
) {
    let mut flat = [Vec4::ZERO; 25];
    for i in 0..10 {
        for j in 0..10 {
            let idx = i * 10 + j;
            let vec_idx = idx / 4;
            let comp_idx = idx % 4;
            flat[vec_idx][comp_idx] = params.interaction_matrix[i][j];
        }
    }

    let mut colors = [Vec4::ZERO; 10];
    for (i, c) in params.colors.iter().enumerate() {
        colors[i] = LinearRgba::from(*c).to_f32_array().into();
    }

    gpu_params.interaction_matrix = flat;
    gpu_params.colors = colors;
    gpu_params.attraction_strength = params.attraction_strength;
    gpu_params.time_scale = params.time_scale;
    gpu_params.min_dist = params.min_dist;
    gpu_params.interaction_radius = params.interaction_radius;
    gpu_params.density_limit = params.density_limit;
    gpu_params.dampening = params.dampening;
    gpu_params.particle_count = params.particle_count as u32;
    gpu_params.region_size_x = params.region_size.x;
    gpu_params.region_size_y = params.region_size.y;
    gpu_params.scale = params.scale;
    gpu_params.global_gravity = params.global_gravity;
    gpu_params.infinite_space = if params.infinite_space { 1 } else { 0 };
}

#[derive(Resource)]
pub struct ParticleBuffer(pub Buffer);

#[derive(Resource)]
pub struct PhysicsComputeBindGroup(pub BindGroup);

#[derive(Resource)]
pub struct PhysicsRenderBindGroup(pub BindGroup);

#[derive(Resource)]
pub struct PhysicsPipeline {
    pub pipeline: CachedComputePipelineId,
    pub compute_layout: BindGroupLayout,
    pub render_layout: BindGroupLayout,
}

impl FromWorld for PhysicsPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let compute_layout = render_device.create_bind_group_layout(
            "physics_compute_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<GpuSimulationParams>(false),
                    storage_buffer::<GpuParticle>(false),
                ),
            ),
        );

        let render_layout = render_device.create_bind_group_layout(
            "physics_render_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<GpuSimulationParams>(false),
                    storage_buffer_read_only::<GpuParticle>(false),
                ),
            ),
        );

        let shader = world.resource::<AssetServer>().load("shaders/physics.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("physics_pipeline")),
            layout: vec![compute_layout.clone()],
            shader,
            shader_defs: vec![],
            entry_point: Some(Cow::from("main")),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        });

        PhysicsPipeline {
            pipeline,
            compute_layout,
            render_layout,
        }
    }
}

fn prepare_gpu_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_params: Res<GpuSimulationParams>,
    sim_params: Res<crate::SimulationParams>,
    mut params_buffer: ResMut<SimulationParamsBuffer>,
    mut particle_buffer: Local<Option<Buffer>>,
    mut current_count: Local<u32>,
    mut current_seed: Local<u32>,
) {
    // Update uniform buffer
    params_buffer.0.set(extracted_params.clone());
    params_buffer.0.write_buffer(&render_device, &render_queue);

    let count = extracted_params.particle_count;

    if *current_count != count || particle_buffer.is_none() || *current_seed != sim_params.spawn_seed {
        let mut rng = rand::thread_rng();
        let range_x = extracted_params.region_size_x * 0.5;
        let range_y = extracted_params.region_size_y * 0.5;
        
        let mut particles = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let max_type = sim_params.particle_types;
            let total_weight: f32 = sim_params.type_proportions[..max_type].iter().sum();
            let mut val = rng.gen_range(0.0..total_weight);
            let mut p_type = 0;
            for (i, &weight) in sim_params.type_proportions[..max_type].iter().enumerate() {
                if val <= weight {
                    p_type = i as u32;
                    break;
                }
                val -= weight;
            }
            
            let px = rng.gen_range(-range_x..range_x);
            let py = rng.gen_range(-range_y..range_y);
            
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let r = rng.gen_range(0.0f32..1.0).sqrt();
            let vx = r * theta.cos() * 0.1;
            let vy = r * theta.sin() * 0.1;

            particles.push(GpuParticle {
                position: Vec2::new(px, py),
                velocity: Vec2::new(vx, vy),
                kind: p_type,
                padding: 0,
            });
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("particle_storage_buffer"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&particles),
        });

        *particle_buffer = Some(buffer.clone());
        commands.insert_resource(ParticleBuffer(buffer.clone()));
        *current_count = count;
        *current_seed = sim_params.spawn_seed;
        commands.insert_resource(ParticleBuffer(buffer));
    }
}

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<PhysicsPipeline>,
    render_device: Res<RenderDevice>,
    params_buffer: Res<SimulationParamsBuffer>,
    particle_buffer: Option<Res<ParticleBuffer>>,
) {
    if let (Some(params_binding), Some(particle_buffer)) = (params_buffer.0.binding(), particle_buffer) {
        let particle_binding = particle_buffer.0.as_entire_binding();
        
        let compute_bind_group = render_device.create_bind_group(
            Some("physics_compute_bind_group"),
            &pipeline.compute_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: params_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: particle_binding.clone(),
                },
            ],
        );
        let render_bind_group = render_device.create_bind_group(
            Some("physics_render_bind_group"),
            &pipeline.render_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: params_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: particle_binding.clone(),
                },
            ],
        );
        commands.insert_resource(PhysicsComputeBindGroup(compute_bind_group));
        commands.insert_resource(PhysicsRenderBindGroup(render_bind_group));
    }
}

pub struct PhysicsComputeNode;

impl FromWorld for PhysicsComputeNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}

impl Node for PhysicsComputeNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<PhysicsPipeline>();
        let params = world.get_resource::<GpuSimulationParams>();

        if let (Some(bind_group), Some(params)) = (world.get_resource::<PhysicsComputeBindGroup>(), params) {
            if let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
                let mut pass = render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor::default());

                pass.set_pipeline(compute_pipeline);
                pass.set_bind_group(0, &bind_group.0, &[]);
                
                let workgroups = (params.particle_count + 63) / 64;
                if workgroups > 0 {
                    pass.dispatch_workgroups(workgroups, 1, 1);
                }
            }
        }
        Ok(())
    }
}
