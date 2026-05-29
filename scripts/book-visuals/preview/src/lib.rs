use waterui::app::App;
use waterui::barcode::Barcode;
use waterui::graphics::{
    FilterViewExt, GpuContext, GpuFrame, GpuSurface, GpuView, Gradient, ResolvedColor,
    ShaderSurface, impl_gpu_subview,
};
use waterui::layout::{Point, Rect, Size};
use waterui::prelude::*;
use waterui::preview;
use waterui_canvas::{Canvas, DrawingContext};

fn resolved_color(red: f32, green: f32, blue: f32) -> ResolvedColor {
    ResolvedColor {
        red,
        green,
        blue,
        opacity: 1.0,
        headroom: 0.0,
    }
}

fn solid_background() -> impl View {
    Color::srgb(247, 249, 252)
}

fn centered_panel(content: impl View) -> impl View {
    zstack((solid_background(), content))
}

#[preview]
fn barcode_qr_book_waterui_dev() -> impl View {
    centered_panel(
        Barcode::qr("https://book.waterui.dev")
            .dark_color(Color::srgb(12, 26, 45))
            .light_color(Color::srgb(255, 255, 255))
            .size(300.0, 300.0),
    )
}

#[preview]
fn canvas_shapes() -> impl View {
    Canvas::new(|context: &mut DrawingContext| {
        context.set_fill_style(resolved_color(0.97, 0.98, 1.0));
        context.fill_rect(Rect::from_size(Size::new(context.width, context.height)));

        context.set_fill_style(resolved_color(0.09, 0.18, 0.30));
        context.fill_rect(Rect::new(
            Point::new(52.0, 58.0),
            Size::new(174.0, 116.0),
        ));

        context.set_fill_style(resolved_color(0.13, 0.72, 0.83));
        context.fill_circle(Point::new(304.0, 134.0), 72.0);

        context.set_stroke_style(resolved_color(0.98, 0.47, 0.58));
        context.set_line_width(10.0);
        let mut path = context.begin_path();
        path.move_to(Point::new(400.0, 220.0));
        path.quadratic_to(Point::new(494.0, 56.0), Point::new(588.0, 210.0));
        context.stroke_path(&path);

        context.set_fill_style(resolved_color(0.99, 0.78, 0.25));
        context.save();
        context.translate(168.0, 242.0);
        context.rotate(0.65);
        context.fill_rect(Rect::new(
            Point::new(-48.0, -48.0),
            Size::new(96.0, 96.0),
        ));
        context.restore();

        context.set_stroke_style(resolved_color(0.09, 0.18, 0.30));
        context.set_line_width(4.0);
        context.stroke_circle(Point::new(486.0, 258.0), 54.0);
    })
}

#[derive(Default)]
struct TriangleRenderer {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl GpuView for TriangleRenderer {
    async fn setup(&mut self, context: &GpuContext<'_>, environment: &mut Environment) {
        let _ = environment;
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("book triangle shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/triangle.wgsl").into()),
            });
        let layout = context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("book triangle layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let blend = if context.is_hdr() {
            None
        } else {
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
        };
        self.pipeline = Some(context.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("book triangle pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: context.pipeline_cache,
            },
        ));
    }

    fn render(&mut self, frame: &mut GpuFrame) {
        let pipeline = self
            .pipeline
            .as_ref()
            .expect("TriangleRenderer pipeline must be initialized before render");
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("book triangle encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("book triangle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.035,
                            g: 0.060,
                            b: 0.100,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }
        frame.queue.submit(core::iter::once(encoder.finish()));
    }
}

impl_gpu_subview!(TriangleRenderer);

#[preview]
fn gpu_surface_triangle() -> impl View {
    GpuSurface::new(TriangleRenderer::default())
}

#[preview]
fn shader_plasma() -> impl View {
    ShaderSurface::with_label(
        "book_plasma.wgsl",
        include_str!("shaders/book_plasma.wgsl"),
    )
}

#[preview]
fn filter_frosted_gradient() -> impl View {
    zstack((
        Gradient::linear(
            vec![
                (0.0, resolved_color(0.10, 0.40, 0.92)),
                (0.45, resolved_color(0.18, 0.82, 0.70)),
                (1.0, resolved_color(0.98, 0.42, 0.55)),
            ],
            [0.0, 0.0],
            [1.0, 1.0],
        ),
        vstack((
            text("Filtered View").size(44.0).foreground(Color::srgb(255, 255, 255)),
            text("blur + brightness + contrast").foreground(Color::srgb(234, 242, 255)),
        ))
        .spacing(10.0)
        .padding(),
    ))
    .blur(7.0)
    .brightness(0.08)
    .contrast(1.18)
}

#[preview]
fn particle_confetti() -> impl View {
    Canvas::new(|context: &mut DrawingContext| {
        context.set_fill_style(resolved_color(0.02, 0.04, 0.08));
        context.fill_rect(Rect::from_size(Size::new(context.width, context.height)));

        let center = Point::new(context.width * 0.5, context.height * 0.08);
        let palette = [
            resolved_color(1.00, 0.80, 0.32),
            resolved_color(0.28, 0.82, 0.86),
            resolved_color(0.98, 0.42, 0.55),
            resolved_color(0.48, 0.58, 1.00),
            resolved_color(0.26, 0.92, 0.62),
        ];
        let mut rng = fastrand::Rng::with_seed(29);

        for index in 0..220 {
            let spread = rng.f32() - 0.5;
            let fall = rng.f32();
            let drift = (fall * core::f32::consts::TAU * 1.7).sin() * 34.0;
            let x = center.x + spread * context.width * 0.86 + drift;
            let y = center.y + fall.powf(0.68) * context.height * 0.86;
            let radius = 2.0 + rng.f32() * 4.8;

            context.set_fill_style(palette[index % palette.len()]);
            context.fill_circle(Point::new(x, y), radius);
        }

        context.set_fill_style(resolved_color(0.10, 0.16, 0.28));
        context.fill_rect(Rect::new(
            Point::new(context.width * 0.35, context.height * 0.03),
            Size::new(context.width * 0.30, 18.0),
        ));
    })
}

#[preview]
fn gradient_mesh() -> impl View {
    Gradient::mesh(
        3,
        3,
        vec![
            ([0.0, 0.0], resolved_color(0.08, 0.23, 0.42)),
            ([0.5, 0.0], resolved_color(0.10, 0.62, 0.72)),
            ([1.0, 0.0], resolved_color(0.95, 0.43, 0.52)),
            ([0.0, 0.5], resolved_color(0.29, 0.17, 0.58)),
            ([0.5, 0.5], resolved_color(1.00, 0.86, 0.38)),
            ([1.0, 0.5], resolved_color(0.22, 0.78, 0.52)),
            ([0.0, 1.0], resolved_color(0.03, 0.08, 0.18)),
            ([0.5, 1.0], resolved_color(0.20, 0.40, 0.88)),
            ([1.0, 1.0], resolved_color(0.88, 0.20, 0.46)),
        ],
        true,
    )
}

fn book_visual_catalog() -> impl View {
    vstack((
        barcode_qr_book_waterui_dev(),
        canvas_shapes(),
        gpu_surface_triangle(),
        shader_plasma(),
        filter_frosted_gradient(),
        particle_confetti(),
        gradient_mesh(),
    ))
}

pub fn app(environment: Environment) -> App {
    App::new(book_visual_catalog, environment)
}
