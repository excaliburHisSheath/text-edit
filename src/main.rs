extern crate app_units;
extern crate euclid;
extern crate glutin;
extern crate gleam;
extern crate rusttype;
extern crate webrender;
extern crate webrender_traits;

use app_units::Au;
use gleam::gl;
use webrender_traits::*;
use rusttype::*;
use std::fs::File;
use std::io::Read;

static TEST_STRING: &'static str = "Mammon slept. And the beast reborn spread over the earth and its numbers grew legion.";
const FONT_SCALE: f32 = 128.0;
const PIXEL_TO_POINT: f32 = 0.71;

fn main() {
    // Load sample font into memory for layout purposes.
    let mut file = File::open("res/FreeSans.ttf").unwrap();
    let mut font_bytes = vec![];
    file.read_to_end(&mut font_bytes).unwrap();

    let font = FontCollection::from_bytes(&*font_bytes).into_font().unwrap();

    // Create a new glutin window and make its OpenGL context active.
    let window = glutin::WindowBuilder::new()
                .with_title("WebRender Sample")
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
                .build()
                .unwrap();

    unsafe {
        window.make_current().ok();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }

    println!("OpenGL version {}", gl::get_string(gl::VERSION));

    // Configure and build the webrender instance.
    // =============================================================================================
    let (width, height) = window.get_inner_size().unwrap();
    println!("width: {}, height: {}", width, height);

    let opts = webrender::RendererOptions {
        device_pixel_ratio: window.hidpi_factor(),
        // debug: true,
        precache_shaders: true,
        enable_scrollbars: true,
        .. Default::default()
    };

    // Create the renderer and its associated `RenderApi` object.
    let (mut renderer, sender) = webrender::renderer::Renderer::new(opts);
    let api = sender.create_api();

    // Create a `Notifier` object to notify the window when a frame is ready.
    let notifier = Box::new(Notifier::new(window.create_window_proxy()));
    renderer.set_render_notifier(notifier);

    let epoch = Epoch(0);
    let root_background_color = ColorF::new(0.3, 0.1, 0.1, 1.0);

    let pipeline_id = PipelineId(0, 0);
    api.set_root_pipeline(pipeline_id);

    let font_key = api.add_raw_font(font_bytes.clone());

    let hidpi_factor = window.hidpi_factor();

    let builder = build_display_lists(
        pipeline_id,
        font_key,
        &font,
        width as f32,
        height as f32,
    );
    api.set_root_display_list(
        Some(root_background_color),
        epoch,
        LayoutSize::new(width as f32, height as f32),
        builder,
    );

    for event in window.wait_events() {
        match event {
            glutin::Event::Closed => break,
            glutin::Event::KeyboardInput(_element_state, scan_code, _virtual_key_code) => {
                if scan_code == 9 {
                    break;
                }
            }
            _ => {}//println!("Unhandled event: {:?}", event),
        }

        let (width, height) = window.get_inner_size().unwrap();

        renderer.update();
        renderer.render(DeviceUintSize::new(width, height) * hidpi_factor as u32);

        window.swap_buffers().ok();
    }
}

fn build_display_lists(
    pipeline_id: PipelineId,
    font_key: FontKey,
    font: &Font,
    width: f32,
    height: f32,
) -> DisplayListBuilder {
    let mut builder = DisplayListBuilder::new(pipeline_id);

    let bounds = LayoutRect::new(LayoutPoint::new(0.0, 0.0), LayoutSize::new(width, height));
    let clip_region = {
        let complex = webrender_traits::ComplexClipRegion::new(
            LayoutRect::new(LayoutPoint::new(0.0, 0.0),
            LayoutSize::new(width, height)),
            webrender_traits::BorderRadius::uniform(0.0),
        );

        builder.new_clip_region(&bounds, vec![complex], None)
    };

    builder.push_stacking_context(
        webrender_traits::ScrollPolicy::Scrollable,
        bounds,
        clip_region,
        0,
        &LayoutTransform::identity(),
        &LayoutTransform::identity(),
        webrender_traits::MixBlendMode::Normal,
        Vec::new(),
    );

    // Yellow rectangle that takes up most of the scren except for 50px around the edges.
    builder.push_rect(
        LayoutRect::new(LayoutPoint::new(0.0, 0.0),
        LayoutSize::new(width, height)),
        clip_region,
        ColorF::new(1.0, 1.0, 0.0, 1.0),
    );

    // Green rectangle sitting towards the middle of the window.
    builder.push_rect(
        LayoutRect::new(LayoutPoint::new(250.0, 250.0),
        LayoutSize::new(100.0, 100.0)),
        clip_region,
        ColorF::new(0.0, 1.0, 0.0, 1.0),
    );
    let dashed_border = BorderSide {
        width: 3.0,
        color: ColorF::new(0.0, 0.0, 1.0, 1.0),
        style: webrender_traits::BorderStyle::Dashed,
    };
    let em_border = BorderSide {
        width: 1.0,
        color: ColorF::new(1.0, 0.0, 1.0, 1.0),
        style: BorderStyle::Solid,
    };
    let glyph_border = BorderSide {
        width: 1.0,
        color: ColorF::new(1.0, 0.0, 0.0, 1.0),
        style: BorderStyle::Solid,
    };
    builder.push_border(
        LayoutRect::new(LayoutPoint::new(250.0, 250.0),
        LayoutSize::new(100.0, 100.0)),
        clip_region,
        dashed_border,
        dashed_border,
        dashed_border,
        dashed_border,
        webrender_traits::BorderRadius::uniform(0.0),
    );

    // Sample text to demonstrate text layout and rendering.
    let text_bounds = LayoutRect::new(LayoutPoint::new(0.0, 0.0), LayoutSize::new(width, height));

    let v_metrics = font.v_metrics(Scale::uniform(FONT_SCALE));
    println!("Font v metrics: {:?}", v_metrics);

    let origin = Point { x: 10.0, y: 200.0 };
    let glyphs = font
        .layout(TEST_STRING, Scale::uniform(FONT_SCALE), origin)
        .inspect(|glyph| {
            let pos = glyph.position();
            let scaled = glyph.unpositioned();
            let h_metrics = scaled.h_metrics();

            // Draw border based on rusttype scaled glyph.
            let rect = LayoutRect::new(
                LayoutPoint::new(pos.x, pos.y - v_metrics.ascent),
                LayoutSize::new(h_metrics.advance_width + h_metrics.left_side_bearing, v_metrics.ascent - v_metrics.descent)
            );
            builder.push_border(
                rect,
                clip_region,
                em_border,
                em_border,
                em_border,
                em_border,
                webrender_traits::BorderRadius::uniform(0.0),
            );

            // Draw border based on webrender glyph dimensions.
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                let rect = LayoutRect::new(
                    LayoutPoint::new(bounding_box.min.x as f32, bounding_box.min.y as f32),
                    LayoutSize::new(bounding_box.width() as f32, bounding_box.height() as f32),
                );
                builder.push_border(
                    rect,
                    clip_region,
                    glyph_border,
                    glyph_border,
                    glyph_border,
                    glyph_border,
                    webrender_traits::BorderRadius::uniform(0.0),
                );
            }
        })
        .map(|glyph| {
            GlyphInstance {
                index: glyph.id().0,
                x: glyph.position().x,
                y: glyph.position().y,
            }
        })
        .collect();

    builder.push_text(
        text_bounds,
        webrender_traits::ClipRegion::simple(&bounds),
        glyphs,
        font_key,
        ColorF::new(0.0, 0.0, 1.0, 1.0),
        Au::from_f32_px(FONT_SCALE * PIXEL_TO_POINT),
        Au::from_px(0),
    );

    builder.pop_stacking_context();

    builder
}

/// Helper struct for updating the window when a frame is done processing.
///
/// Notifier exists so we can implement [`RenderNotifier`][RenderNotifier] for
/// [`glutin::WindowProxy`][glutin::WindowProxy]. This allows us to trigger a window repaint
/// when a frame is done rendering.
///
/// [RenderNotifier]: ./webrender//webrender_traits/trait.RenderNotifier.html
/// [glutin::WindowProxy]: glutin/struct.WindowProxy.html
struct Notifier {
    window_proxy: glutin::WindowProxy,
}

impl Notifier {
    fn new(window_proxy: glutin::WindowProxy) -> Notifier {
        Notifier {
            window_proxy: window_proxy,
        }
    }
}

impl webrender_traits::RenderNotifier for Notifier {
    fn new_frame_ready(&mut self) {
        self.window_proxy.wakeup_event_loop();
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
        self.window_proxy.wakeup_event_loop();
    }

    fn pipeline_size_changed(&mut self, _: PipelineId, _: Option<LayoutSize>) {}
}
