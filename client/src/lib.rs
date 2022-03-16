#[allow(dead_code)]

mod arena;

use winit::{
  event_loop::EventLoop,
  window::WindowBuilder,
  dpi::PhysicalSize,
  platform::web::WindowBuilderExtWebSys,
  event::KeyboardInput,
};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::Promise;

use utils::{get_canvas, init_window};

use engine::{
  renderer::{
    context::Context,
  },
  application::{
    executor::Application,
    input::Input,
  },
};

use crate::{
  arena::ArenaLayer,
};

fn grow_memory(pages: u32) {
  use js_sys::WebAssembly::Memory;
  use wasm_bindgen::memory;
  use wasm_bindgen::JsCast;

  let mem = match memory().dyn_into::<Memory>() {
    Ok(mem) => Ok(mem),
    Err(e) => {
      log::error!("Error: {:?}", e);
      Err(e)
    },
  }.unwrap();
  log::debug!("growing memory: {:?}", mem);

  mem.grow(pages);
}

const DEFAULT_WIDTH:u32 = 1280;
const DEFAULT_HEIGHT:u32 = 1280;

#[wasm_bindgen]
pub struct Game {
}

#[wasm_bindgen]
impl Game {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self { }
  }

  pub fn start(
    &self,
    assets_location: String,
    _session_id: String,
    _asset_url: String,
    _access_token: String,
    _udp_url: String,
    _tcp_url: String,
  ) -> Promise {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::default());
    future_to_promise(async move {
      // ~130MB
      grow_memory(2000);
      log::info!("assets {:}", assets_location);
      let event_loop = EventLoop::new();
      let canvas = get_canvas!("canvas");
      let _window = init_window!(DEFAULT_WIDTH, DEFAULT_HEIGHT, &event_loop, canvas);
      let context = Context::new("canvas").unwrap();

      let mut application = Application::new();

      let arena_layer = ArenaLayer::new(
        &context,
        assets_location,
        DEFAULT_WIDTH,
        DEFAULT_HEIGHT,
      );
      let mut input = Input::new();
      application.push_layer(&context, Box::new(arena_layer)).await;
      let keyboard: Option<KeyboardInput> = None;

      event_loop.run(move |event, _, control_flow| {
        input.process_events(keyboard, control_flow, event);
        application.run(&context, &input);
      });
    })
  }
}
