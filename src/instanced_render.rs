use bevy::{
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, RenderGraphExt, ViewNode, ViewNodeRunner},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};

use crate::gpu_pipeline::{GpuSimulationParams, PhysicsPipeline, PhysicsRenderBindGroup};

pub struct InstancedRenderPlugin;

impl Plugin for InstancedRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        
        render_app
            .add_render_graph_node::<ViewNodeRunner<InstancedRenderNode>>(
                bevy::core_pipeline::core_2d::graph::Core2d,
                InstancedRenderLabel,
            )
            .add_render_graph_edge(
                bevy::core_pipeline::core_2d::graph::Core2d,
                bevy::core_pipeline::core_2d::graph::Node2d::MainTransparentPass,
                InstancedRenderLabel,
            )
            .add_render_graph_edge(
                bevy::core_pipeline::core_2d::graph::Core2d,
                InstancedRenderLabel,
                bevy::core_pipeline::core_2d::graph::Node2d::EndMainPass,
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<InstancedPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct InstancedRenderLabel;

#[derive(Resource)]
pub struct InstancedPipeline {
    pub pipeline_id: CachedRenderPipelineId,
    pub physics_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for InstancedPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let physics_pipeline = world.resource::<PhysicsPipeline>();
        
        let physics_layout = physics_pipeline.render_layout.clone();

        let view_layout = render_device.create_bind_group_layout(
            Some("instanced_view_bind_group_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(bevy::render::view::ViewUniform::min_size()),
                },
                count: None,
            }],
        );

        let shader = world.resource::<AssetServer>().load("shaders/instanced.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some(std::borrow::Cow::from("instanced_render_pipeline")),
            layout: vec![view_layout.clone(), physics_layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Some(std::borrow::Cow::from("vertex")),
                buffers: vec![], // We generate vertices in shader
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: Some(std::borrow::Cow::from("fragment")),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: false,
        });

        Self { pipeline_id, physics_layout, view_layout }
    }
}

pub struct InstancedRenderNode;

impl FromWorld for InstancedRenderNode {
    fn from_world(_world: &mut World) -> Self {
        Self
    }
}

impl ViewNode for InstancedRenderNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static bevy::render::view::ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, view_uniform_offset): bevy::ecs::query::QueryItem<'_, '_, Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<InstancedPipeline>();
        let view_uniforms = world.resource::<bevy::render::view::ViewUniforms>();
        let render_device = world.resource::<RenderDevice>();

        let params = world.get_resource::<crate::gpu_pipeline::GpuSimulationParams>();

        if let (Some(sim_params), Some(physics_bind_group)) = (params, world.get_resource::<PhysicsRenderBindGroup>()) {
            if let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) {
                if let Some(view_binding) = view_uniforms.uniforms.binding() {
                    
                    let view_bind_group = render_device.create_bind_group(
                        Some("view_bind_group"),
                        &pipeline.view_layout,
                        &[BindGroupEntry {
                            binding: 0,
                            resource: view_binding,
                        }],
                    );
                    let color_attachment = view_target.get_color_attachment();
                    let mut render_pass = render_context
                        .begin_tracked_render_pass(RenderPassDescriptor {
                            label: Some("instanced_particles_pass"),
                            color_attachments: &[Some(color_attachment)],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                    render_pass.set_render_pipeline(render_pipeline);
                    render_pass.set_bind_group(0, &view_bind_group, &[view_uniform_offset.offset]);
                    render_pass.set_bind_group(1, &physics_bind_group.0, &[]);
                    
                    render_pass.draw(0..6, 0..sim_params.particle_count);
                }
            }
        }
        Ok(())
    }
}
