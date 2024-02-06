//use std::io::Error;

use std::sync::Arc;

use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
//use base64::Engine;use clap::{AppSettings, Arg, Command};
//use hyper::service::{make_service_fn, service_fn};
//use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;



#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    m.register_default_codecs().unwrap();

    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m).unwrap();

    // Create the API object with the MediaEngine
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

    // When an ICE candidate is available send to the other Pion instance
    // the other Pion instance will add this candidate by calling AddICECandidate
    //let pc = Arc::downgrade(&peer_connection);
    


    // Create a datachannel with label 'data'
    let data_channel = peer_connection.create_data_channel("data", None).await.unwrap();

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            println!("Peer Connection has gone to failed exiting");
            let _ = done_tx.try_send(());
        }

        Box::pin(async {})
    }));

    // Register channel opening handling
    let d1 = Arc::clone(&data_channel);
    data_channel.on_open(Box::new(move || {
        println!("Data channel '{}'-'{}' open. Random messages will now be sent to any connected DataChannels every 5 seconds", d1.label(), d1.id());

        let d2 = Arc::clone(&d1);
        Box::pin(async move {
            let mut result = Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(5));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        let message = "ping";
                        println!("Sending '{message}'");
                        result = d2.send_text(message).await.map_err(Into::into);
                    }
                };
            }
        })
    }));

    // Register text message handling
    let d_label = data_channel.label().to_owned();
    data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
        let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
        println!("Message from DataChannel '{d_label}': '{msg_str}'");
        Box::pin(async {})
    }));



    let offer = peer_connection.create_offer(None).await?;
    //peer_connection.set_local_description(offer).await;

    // Create channel that is blocked until ICE Gathering is complete
    let mut gather_complete = peer_connection.gathering_complete_promise().await;
    
    // Sets the LocalDescription, and starts our UDP listeners
    peer_connection.set_local_description(offer).await.unwrap();
    let _ = gather_complete.recv().await;

    if let Some(local_desc) = peer_connection.local_description().await {
        let json_str = serde_json::to_string(&local_desc)?;
        let b64 = BASE64_STANDARD.encode(&json_str);
        println!("{}",b64);
    } else {
        println!("generate local_description failed!");
    }


    println!("Paste the SDP offer from the remote peer");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        line = line.trim().to_owned();
        let desc_data = decode(line.as_str()).unwrap();
        let offer = serde_json::from_str::<RTCSessionDescription>(&desc_data)?;
        // Set the remote SessionDescription
        peer_connection.set_remote_description(offer).await.unwrap();



    println!("Press ctrl-c to stop");
    tokio::select! {
        _ = done_rx.recv() => {
            println!("received done signal!");
        }
        _ = tokio::signal::ctrl_c() => {
            println!();
        }
    };

    peer_connection.close().await.unwrap();

    Ok(())
}

fn decode(s: &str) -> Result<String, String> {
    let b =  BASE64_STANDARD.decode(s).unwrap();

    //if COMPRESS {
    //    b = unzip(b)
    //}
    Ok(String::from_utf8(b).unwrap())
    
}

// fn encode(b: &str) -> String {
//     //if COMPRESS {
//     //    b = zip(b)
//     //}

//     BASE64_STANDARD.encode(b)
// }


