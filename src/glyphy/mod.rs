use futures::executor::LocalSpawner;
use futures::task::SpawnExt;
use wgpu_glyph::{
    ab_glyph::{self, InvalidFont},
    GlyphBrush, GlyphBrushBuilder, Section, Text,
};

pub struct Glyphy {
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures::executor::LocalPool,
    local_spawner: LocalSpawner,
    brush: GlyphBrush<()>,
}

fn hex_str_to_rgba<'a>(s: &'a str) -> [f32; 4] {
    let re = regex::Regex::new(r"#([a-fA-F0-9]{6})").unwrap();
    if !re.is_match(s) {
        panic!("{} is not in hex format", s);
    };

    let mut rgba: Vec<f32> = [&s[1..3], &s[3..5], &s[5..7]]
        .iter()
        .map(|v| hex::decode(v).unwrap()[0] as f32 / 255.0)
        .collect();
    rgba.append(&mut vec![1.0]);

    [rgba[0], rgba[1], rgba[2], rgba[3]]
}

#[test]
#[should_panic]
fn test_bad_hex_str_to_rgba() {
    let bad_hex_str = "af4573";
    hex_str_to_rgba(bad_hex_str);
}

#[test]
#[should_panic]
fn test_bad_hex_str_to_rgba_2() {
    let bad_hex_str = "#af457";
    hex_str_to_rgba(bad_hex_str);
}

#[test]
fn test_hex_str_to_rgba() {
    let hex_str = "#af4573";
    let rgba = hex_str_to_rgba(hex_str);
    assert_eq!(rgba, [0.6862745, 0.27058825, 0.4509804, 1.0,])
}

impl Glyphy {
    pub fn init(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self, InvalidFont> {
        // Create staging belt and a local pool
        let staging_belt = wgpu::util::StagingBelt::new(1024);
        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();
        // Prepare glyph_brush
        let inconsolata =
            ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf"))?;
        let brush = GlyphBrushBuilder::using_font(inconsolata).build(&device, format);

        Ok(Self {
            brush,
            staging_belt,
            local_pool,
            local_spawner,
        })
    }
    pub fn render<'a>(
        &mut self,
        text: &'a str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        view: &wgpu::TextureView,
        clear: bool,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Redraw"),
        });

        // Clear frame
        {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            })
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
        }

        let hex_str = "#af4573";
        // ^(0x|0X)?[a-fA-F0-9]+$'

        // let normalized = [rgb.r / 255.0, rgb.g / 255.0, rgb.b / 255.0 / 0, 1.0];
        // dbg!(normalized);

        self.brush.queue(Section {
            screen_position: (size.0 as f32 - 30.0 * text.len() as f32, 30.0),
            bounds: (size.0 as f32, size.1 as f32),
            text: vec![Text::new(text)
                .with_color(hex_str_to_rgba(hex_str))
                .with_scale(40.0)],
            ..Section::default()
        });

        // Draw the text!
        self.brush
            .draw_queued(
                &device,
                &mut self.staging_belt,
                &mut encoder,
                view,
                size.0,
                size.1,
            )
            .expect("Draw queued");

        // Submit the work!
        self.staging_belt.finish();
        queue.submit(Some(encoder.finish()));

        // Recall unused staging buffers
        self.local_spawner
            .spawn(self.staging_belt.recall())
            .expect("Recall staging belt");

        self.local_pool.run_until_stalled();
    }
}
