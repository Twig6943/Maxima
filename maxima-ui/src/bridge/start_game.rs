use log::{debug, error, info};
use maxima::{core::{launch::{self, LaunchMode}, LockedMaxima}, rtm::client::BasicPresence};

pub async fn start_game_request(maxima_arc: LockedMaxima, offer_id: String, hardcode_paths: bool) {
    let maxima = maxima_arc.lock().await;
    let logged_in = maxima.auth_storage().lock().await.current().is_some();
    if !logged_in {
        info!("Ignoring request to start game, not logged in.");
        return;
    }

    debug!("got request to start game {:?}", offer_id);

    let maybe_args: Vec<String> = if offer_id.eq("Origin.OFR.50.0001456") || offer_id.eq("Origin.OFR.50.0002304") {
        vec!["-windowed".to_string(), "-novid".to_string(), "-northstar".to_string()]
    } else if offer_id.eq("Origin.OFR.50.0000739") {
        vec!["-windowed".to_string(), "-novid".to_string()]
    } else {
        vec![]
    };

    drop(maxima);
    let result = launch::start_game(maxima_arc.clone(), LaunchMode::Online(offer_id), None, maybe_args).await;
    if result.is_err() {
        error!("Failed to start game! Reason: {}", result.err().unwrap());
    }
    

}
