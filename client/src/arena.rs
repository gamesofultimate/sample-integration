use std::f32::consts::PI;
use async_trait::async_trait;
use std::{
  path::Path,
  collections::HashMap,
};
use nalgebra::{Vector3, Point3};
use engine::{
  renderer::{
    webgl::WebGlApi,
    context::Context,
    resources::{Resources, Texture, Model},
    camera::PerspectiveCamera,
  },
  application::{
    renderer3d::Renderer3d,
    layer::Layer,
    input::Input,
    scene::Scene,
    components::{
      TransformComponent,
      MeshComponent,
      CameraComponent,
      LightComponent,
      FreeRangeComponent,
    },
  },
};

pub struct ArenaLayer {
  resources: Resources,
  scene: Scene,
  textures: HashMap<String, Texture>,
  renderer: Renderer3d,
  frame: f32,
  width: u32,
  height: u32,
}

impl ArenaLayer {
  pub fn new(context: &Context, assets_location: String, width: u32, height: u32) -> Self {
    let resources = Resources::from_external_path(
      Path::new(&assets_location),
      Path::new("./"),
    ).unwrap();
    let scene = Scene::new(width, height);
    let render = WebGlApi::new();

    Self {
      scene,
      resources: resources.clone(),
      frame: 0.0,
      renderer: Renderer3d::new(context, &resources, width, height, Box::new(render)),
      textures: HashMap::new(),
      width,
      height,
    }
  }
}

#[async_trait(?Send)]
impl Layer for ArenaLayer {
  async fn on_attach(&mut self, context: &Context) {

    {
      let camera = PerspectiveCamera::new(
        self.width as f32,
        self.height as f32,
        30.0,
        0.1,
        10.0,
      );
      let camera_component = CameraComponent::with_camera(camera);
      let free_range_component = FreeRangeComponent::new();
      let transform_component = TransformComponent::with_translation(Vector3::new(
        0.0,
        0.0,
        -0.5,
      ));
      let camera_entity = self.scene.create_entity("Camera A");
      self.scene.add_components(camera_entity, (
        camera_component,
        free_range_component,
        transform_component,
      ));
    }

    let fairy_female = {
      let model = Model::load_gltf(&self.resources, "models/fairy-female/main.gltf").await.unwrap();
      //let mut model = Model::load_gltf(&self.resources, "models/barry/scene.gltf").await.unwrap();
      //let mut model = Model::load_gltf(&self.resources, "models/kabuto/scene.gltf").await.unwrap();
      let model_id = self.renderer.include_model(model);
      self.renderer.load_model(&context, &model_id, 1).await;
      model_id
    };

    {
      let model = Model::load_gltf(&self.resources, "models/level/main.gltf").await.unwrap();
      let model_id = self.renderer.include_model(model);
      self.renderer.load_model(&context, &model_id, 1).await;
      let mut transform = TransformComponent::with_defaults();
      transform.translation = Vector3::new(0.0, 0.0, 0.0);
      transform.rotation = Vector3::new(PI, PI, 0.0);
      transform.scale = Vector3::new(1.0, 1.0, 1.0);

      let mesh_component = MeshComponent::with_id(model_id);
      let entity = self.scene.create_entity("Level");
      self.scene.add_components(entity, (mesh_component, transform));
    }

    {
      let mesh_component = MeshComponent::with_id(fairy_female);

      let mut transform = TransformComponent::with_defaults();
      transform.translation = Vector3::new(0.0, 0.0, 0.0);
      transform.rotation = Vector3::new(PI, PI, 0.0);
      transform.scale = Vector3::new(0.025, 0.025, 0.025);


      let entity = self.scene.create_entity("Player1");
      self.scene.add_components(entity, (mesh_component, transform));
    }

    {
      let transform = TransformComponent::with_translation(Vector3::new(0.5, 0.5, -0.5));
      let pointlight = LightComponent::with_point_light(Vector3::new(0.0, 0.0, 1.0));
      let light = self.scene.create_entity("Light A");
      self.scene.add_components(light, (pointlight, transform));
    }
  }

  fn on_detach(&self) {
  }

  fn on_update(&mut self, context: &Context, input: &Input) {
    self.renderer.new_frame(context);

    for (_id, (transform, _camera, _)) in self.scene.query_mut::<(
      &mut TransformComponent,
      &CameraComponent<PerspectiveCamera>,
      &FreeRangeComponent,
    )>() {
      match input {
        Input { up: true, .. } => transform.translation[2] += 0.01,
        Input { down: true, .. } => transform.translation[2] -= 0.01,
        _ => { }
      };

      match input {
        Input { right: true, .. } => transform.translation[0] += 0.01,
        Input { left: true, .. } => transform.translation[0] -= 0.01,
        _ => { }
      };

      match input {
        Input { mouse_wheel, .. } => transform.translation[2] += 0.001 * mouse_wheel,
      };

      match input {
        Input { delta_x, delta_y, .. } => {
          transform.rotation[0] += delta_x * PI;
          transform.rotation[1] += delta_y * PI;
        },
      };
    }

    for (_id, (transform, camera)) in self.scene.query_mut::<(&mut TransformComponent, &CameraComponent<PerspectiveCamera>)>() {
      let camera_view = transform.look_at(Point3::new(0.0, 0.0, 1.0) + transform.translation);

      let light_direction = Vector3::new(
        self.frame.sin() * 1.0,
        -0.75,
        self.frame.cos() * 1.0,
      );

      let light_color = Vector3::new(
        1.0,
        1.0,
        1.0,
      );

      let base_color = Vector3::new(
        0.03,
        0.03,
        0.03,
      );

      self.renderer.set_environment(
        1.5,
        light_direction,
        light_color,
        base_color,
        &camera.camera,
        &camera_view,
      );

      self.renderer.load_camera(
        context,
        transform.translation,
        camera.get_projection(),
        camera_view,
      );
    }

    self.renderer.load_environment(&context);

    /*
    for (id, (mut transform_l, light)) in self.scene.query_mut::<(&mut TransformComponent, &LightComponent)>() {
      transform_l.translation += Vector3::new(
        self.frame.cos() * 0.001,
        //0.75,
        self.frame.sin() * 0.001,
        0.0,
        //-1.0,
      );

      match light {
        LightComponent::Point { radiance, .. } => {
          self.renderer.load_light(
            context,
            &transform_l.translation,
            &radiance,
          );
        },
        _ => { },
      };
    }
    */


    for (_id, (transform_m, mesh)) in self.scene.query_mut::<(&mut TransformComponent, &MeshComponent)>() {
      self.renderer.draw_model(context, &mesh.id, transform_m.get_transform());
    }

    self.renderer.draw_scene(context);
    self.frame += 0.0025;
  }

  fn on_im_gui_render(&self) {
  }

  fn on_event(&self, /*event: &Event*/) {
  }
}
