mod glyphy;
use crate::glyphy::Glyphy;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };

    let (device, queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Request device")
    });

    let mut size = window.inner_size();

    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: render_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        },
    );

    let mut glyphy = Glyphy::init(&device, render_format).unwrap();

    window.request_redraw();

    let mut text = "a: vec![#dd1133]";

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::CloseRequested,
            ..
        } => *control_flow = winit::event_loop::ControlFlow::Exit,
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::Resized(new_size),
            ..
        } => {
            size = new_size;

            surface.configure(
                &device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: render_format,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Mailbox,
                },
            );
        }
        winit::event::Event::RedrawRequested { .. } => {
            let frame = surface.get_current_texture().expect("Get next frame");
            let view = &frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            glyphy.render(text, &device, &queue, (size.width, size.height), view, true);
            frame.present();
        }
        _ => {
            *control_flow = winit::event_loop::ControlFlow::Wait;
        }
    })
}
