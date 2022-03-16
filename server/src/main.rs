#![feature(or_patterns)]
#![recursion_limit="256"]

use dotenv::dotenv;
use regex::Regex;
use bytes::Bytes;

use std::{
  thread,
  io,
  net::SocketAddr,
  net::ToSocketAddrs,
  path::Path,
  time::Duration,
  sync::mpsc,
  sync::Arc,
};

use tokio::sync::mpsc as async_mpsc;
use serde::{Serialize, Deserialize};

use futures::FutureExt;

use uuid::Uuid;

use networking::server::connection::{
  Connection,
  Message,
  Score,
};

use utils::{
  environment,
  environment_with_default,
  rpc_service,
  get_socket,
  resolve_address,
};
use server_utils::producer::{
  create_producer,
  send_to_queue,
  send_as_json,
};

const MAX_UDP_PAYLOAD_SIZE: usize = 65507;

#[derive(Debug, Serialize)]
pub enum Input {
  Up { pressure: f32 },
  Right { pressure: f32 },
  Down { pressure: f32 },
  Left { pressure: f32 },
}

#[derive(Serialize, Deserialize)]
struct PlayerData {
  pub count: u32,
}

#[tokio::main]
async fn main() {
  let location = if cfg!(feature = "with-prod") {
    environment_with_default!("GAME_ENV", "../.env.production")
  } else {
    environment_with_default!("GAME_ENV", "../.env.local")
  };

  dotenv::from_filename(location).ok();
  env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

  let rpc_address = {
    let address = environment!("RPC_ADDRESS");
    let port = environment!("RPC_PORT");
    format!("http://{}:{}", address, port)
  };

  let session_address = {
    let address = environment!("ADDRESS");
    let port = environment!("PORT");

    resolve_address!(address, port).unwrap()
  };

  let session_id = Uuid::new_v4();

  let ultimate = Connection::new(
    rpc_address,
    session_address,
    session_address,
  ).await;

  let bus = ultimate.get_bus();

  let loop_bus = bus.clone();
  let game_bus = bus.clone();

  let listener = ultimate.listen().await;
  log::info!("start listening...");

  let (incoming_sender, incoming_receiver) = crossbeam::channel::unbounded::<Input>();

  let mut game = Game::new(
    session_id,
    incoming_receiver,
    game_bus,
  ).await;

  // NOTE: This calls marks the channel ready to start, getting rid of the lobby, and
  // starting the gameplay experience
  let channel = loop_bus.start_game().await;
  log::info!("Channel {:#?}", channel);

  loop {
    if let Ok(message) = listener.recv() {
      match message {
        // NOTE: Packets received from the network. This can be a reliable or
        // unreliable messages. We receive those through the same bus.
        Message::Packet { player_id, data } => {
          let input:Input = bincode::deserialize(&data).unwrap();
          //loop_bus.send_unreliable(player_id, data);
          //loop_bus.send_reliable(player_id, data);
        },
        // NOTE:: You receive this message when a user logs in
        Message::PlayerJoined { player_id, username, protocol } => {
          // NOTE: You can get more information about the player
          let player = loop_bus.get_player(player_id).await;
          log::info!("player joined ({:?}, {:?}): {:?}", player_id, protocol, player);

          // NOTE: You can submit an achievement (it must first be defined in the Arcadefile)
          loop_bus.submit_achievement("first-kill", player_id).await;

          // NOTE: I'm using that information to add a new player to the game itself
          game.new_player(player_id, 0, 0);

          // NOTE: This is a data store for player data that is stores across game sessions
          // This is the getter
          let count = match loop_bus.get_player_data::<PlayerData>(player_id).await {
            Ok(Some(data)) => data.count,
            _ => 0,
          }; 
          log::info!("player data: {:?}", count);

          // NOTE: This is a data store for player data that is stores across game sessions
          // This is the setter
          let result = loop_bus.save_player_data::<PlayerData>(player_id, PlayerData {
            count: count + 5,
          }).await; 
          log::info!("player data: {:?}", result);

          for _ in 0..3 {
            // NOTE: This submits points for ranking. You must define a rank in
            // the Arcadefile first
            loop_bus.submit_point("kills", player_id, None, None, 1).await;
          }

          // NOTE: This submits the end results of a game to the platform. Should be called
          // at least once, but can be called more than once.
          loop_bus.submit_result("end game", None, None, vec![
            Score { ranking: String::from("kills"), player_id, score: 1},
            Score { ranking: String::from("deaths"), player_id, score: 5},
          ]).await;

          // NOTE: These calls mark the player (un)ready to play (lobby management)
          loop_bus.mark_player_ready(player_id).await;
          loop_bus.mark_player_unready(player_id).await;
        },
        _ => {
          log::info!("???");
        },
      }
    }

    // Game loop
    game.run();
  }

  // NOTE: This calls marks the gameplay ended, shows the lobby again, and
  // shows the chat again, so people can comment on the game.
  let channel = loop_bus.end_game().await;
}
