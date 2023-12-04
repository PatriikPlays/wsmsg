mod util;

use std::{collections::HashMap, sync::{Arc, RwLock}, vec};
use std::mem::drop;


use lazy_static::lazy_static;
use rand::{SeedableRng, Rng};
use rand_hc::Hc128Rng;
use util::*;
use ws::{listen, Handler, Handshake, Sender, Response};

struct WSHandler {
    out: Sender,
    channels: Vec<Vec<u8>>
}

lazy_static! {
    static ref SUBSCRIPTIONS: Arc<RwLock<HashMap<Vec<u8>, Vec<Sender>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref RNG: Arc<RwLock<Hc128Rng>> = Arc::new(RwLock::new(Hc128Rng::from_entropy()));
}

impl Handler for WSHandler {
    fn on_open(&mut self, _shake: Handshake) -> ws::Result<()> {
        println!("WS connection {} opened", self.out.connection_id());
        
        let headers = _shake.request.headers();

        headers.iter().for_each(|h| {
            let (name, data) = h;
            if name == "X-Token" {
                let sp: Vec<Vec<u8>> = data
                    .split(|&v| v == 44)
                    .map(|chunk| chunk.to_vec())
                    .collect();

                sp.iter().for_each(|token| {
                    if token.len() == 32 {
                        if self.channels.len() < 32 {
                            self.channels.push(token.clone());
                        }
                    }
                })
            }
        });

        if self.channels.len() > 0 {
            let mut s = SUBSCRIPTIONS.write().unwrap();

            self.channels.iter().for_each(|ch| {
                if !s.contains_key(ch) {
                    s.insert(ch.clone(), Vec::new());
                }

                let v = s.get_mut(ch).unwrap();


                v.push(self.out.clone());
            });

            drop(s);
        }

        return Ok(());
    }

    fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
        println!("WS connection {} closed", self.out.connection_id());

        if self.channels.len() > 0 {
            let mut s = SUBSCRIPTIONS.write().unwrap();

            self.channels.iter().for_each(|ch| {
                if s.contains_key(ch) {
                    let v = s.get_mut(ch).unwrap();

                    v.retain(|x| x.connection_id() != self.out.connection_id());

                    if v.len() == 0 {
                        s.remove(ch);
                    }
                }
            });

            drop(s);
        }
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        if msg.is_binary() {
            let parsed_msg = WSMSGMessage::from_message(msg.into_data());

            /*unsafe {
                println!("Theyer angry!! id:{} type:{} data:{}", parsed_msg.clone().unwrap().message_id, parsed_msg.clone().unwrap().message_type, String::from_utf8_unchecked(parsed_msg.clone().unwrap().message_data));
            }*/
            match parsed_msg {
                Ok(v) => match v.message_type {
                    WSMSGMessageTypeC2S::Message => {
                        if v.message_data.len() < 32 {
                            if v.message_id == 0 {
                                return Ok(());
                            }
    
                            let mut response_data: Vec<u8> = vec![];
                            response_data.extend_from_slice(WSMSGMessageError::MessageMalformed.to_string().as_bytes());

                            return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                                message_type: WSMSGMessageTypeS2C::Reply,
                                message_id: Some(v.message_id),
                                message_code: Some(1),
                                message_data: response_data
                            }));
                        }

                        let target_channel: Vec<u8> = v.message_data[0..32].to_vec();

                        let mut s = SUBSCRIPTIONS.write().unwrap();

                        if s.contains_key(&target_channel) {
                            let y = s.get_mut(&target_channel).unwrap();

                            y.iter().for_each(|sender| {
                                if sender.connection_id() != self.out.connection_id() {
                                    sender.send(WSMSGResponse::to_message(&WSMSGResponse {
                                        message_type: WSMSGMessageTypeS2C::Message,
                                        message_id: None,
                                        message_code: None,
                                        message_data: v.message_data.clone()
                                    })).unwrap();    
                                }
                            });
                        }

                        drop(s);

                        if v.message_id == 0 {
                            return Ok(());
                        }

                        return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                            message_type: WSMSGMessageTypeS2C::Reply,
                            message_id: Some(v.message_id),
                            message_code: Some(0),
                            message_data: vec![]
                        }));
                    }
                    WSMSGMessageTypeC2S::Ping => {
                        if v.message_id == 0 {
                            return Ok(());
                        }

                        return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                            message_type: WSMSGMessageTypeS2C::Reply,
                            message_id: Some(v.message_id),
                            message_code: Some(0),
                            message_data: v.message_data,
                        }));
                    }
                    WSMSGMessageTypeC2S::ListSubscribed => {
                        if v.message_id == 0 {
                            return Ok(());
                        }

                        return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                            message_type: WSMSGMessageTypeS2C::Reply,
                            message_id: Some(v.message_id),
                            message_code: Some(0),
                            message_data: self.channels.iter().flat_map(|x| x.clone()).collect()
                        }));
                    }
                    WSMSGMessageTypeC2S::Subscribe => {
                        if v.message_data.len() != 32 {
                            if v.message_id == 0 {
                                return Ok(());
                            }

                            return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                                message_type: WSMSGMessageTypeS2C::Reply,
                                message_id: Some(v.message_id),
                                message_code: Some(1),
                                message_data: Vec::from("Expected token length 32".as_bytes())
                            }));
                        }

                        if self.is_subscribed(&v.message_data) {
                            if v.message_id == 0 {
                                return Ok(());
                            }

                            return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                                message_type: WSMSGMessageTypeS2C::Reply,
                                message_id: Some(v.message_id),
                                message_code: Some(1),
                                message_data: Vec::from("Already subscribed".as_bytes())
                            }));
                        }

                        self.subscribe(v.message_data);
                        
                        if v.message_id == 0 {
                            return Ok(());
                        }

                        return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                            message_type: WSMSGMessageTypeS2C::Reply,
                            message_id: Some(v.message_id),
                            message_code: Some(0),
                            message_data: Vec::new()
                        }));
                    }
                    WSMSGMessageTypeC2S::Unsubscribe => {
                        if v.message_data.len() != 32 {
                            if v.message_id == 0 {
                                return Ok(());
                            }

                            return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                                message_type: WSMSGMessageTypeS2C::Reply,
                                message_id: Some(v.message_id),
                                message_code: Some(1),
                                message_data: Vec::from("Expected token length 32".as_bytes())
                            }));
                        }

                        if !self.is_subscribed(&v.message_data) {
                            if v.message_id == 0 {
                                return Ok(());
                            }

                            return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                                message_type: WSMSGMessageTypeS2C::Reply,
                                message_id: Some(v.message_id),
                                message_code: Some(1),
                                message_data: Vec::from("Not subscribed".as_bytes())
                            }));
                        }

                        self.unsubscribe(v.message_data);
                        
                        if v.message_id == 0 {
                            return Ok(());
                        }

                        return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                            message_type: WSMSGMessageTypeS2C::Reply,
                            message_id: Some(v.message_id),
                            message_code: Some(0),
                            message_data: Vec::new()
                        }));
                    }
                },
                Err(e) => {
                    let mut response_data: Vec<u8> = vec![];
                    response_data.extend_from_slice(e.to_string().as_bytes());

                    return self.out.send(WSMSGResponse::to_message(&WSMSGResponse {
                        message_type: WSMSGMessageTypeS2C::Reply,
                        message_id: Some(u64::MAX),
                        message_code: Some(1),
                        message_data: response_data,
                    }));
                }
            }
        }

        return Ok(());
    }

    fn on_request(&mut self, req: &ws::Request) -> ws::Result<ws::Response> {
        match req.resource() {
            "/ws" => {
                return Response::from_request(req);
            }
            "/csprng" => {
                let mut data: Vec<u8> = vec![];

                let mut rng = RNG.write().unwrap();
                for _ in 0..32 {
                    data.push(rng.gen());
                }
                drop(rng);
                
                return Ok(Response::new(200, "OK", data));
            }
            _ => {
                if req.resource().starts_with("/ws/") {
                    return Response::from_request(req);
                }
                return Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec()));
            }
        }
    }
}

impl WSHandler {
    fn is_subscribed(&self, channel: &Vec<u8>) -> bool {
        for ch in self.channels.iter() {
            let matching = ch.len() == channel.len() && ch.iter().zip(channel).filter(|&(a, b)| a == b).count() == ch.len();
            if matching {
                return true;
            }
        }
        return false;
    }

    fn subscribe(&mut self, channel: Vec<u8>) {
        if self.is_subscribed(&channel) { return };

        let mut s = SUBSCRIPTIONS.write().unwrap();

        if !s.contains_key(&channel) {
            s.insert(channel.clone(), Vec::new());
        }

        let v = s.get_mut(&channel).unwrap();

        v.push(self.out.clone());

        drop(s);

        self.channels.push(channel);
    }

    fn unsubscribe(&mut self, channel: Vec<u8>) {
        if let Some(to_remove) = self.channels.iter().enumerate()
            .find(|&(_i, ch)| ch.len() == channel.len() && ch.iter().zip(&channel).all(|(a, b)| a == b))
            .map(|(i, _ch)| i) {

            let mut s = SUBSCRIPTIONS.write().unwrap();

            if !s.contains_key(&channel) {
                eprintln!("Inconsistent state, handler was listening on channel, but the channel was not found in the subscriptions map");
                self.channels.remove(to_remove);
                return;
            }

            let v = s.get_mut(&channel).unwrap();

            v.retain(|x| x.connection_id() != self.out.connection_id());

            if v.len() == 0 {
                s.remove(&channel);
            }

            drop(s);

            self.channels.remove(to_remove);

        }
    }
}

fn main() {
    let port_str = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port = match port_str.parse::<u16>() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Invalid port: {}", e);
            std::process::exit(1);
        },
    };

    let listen_host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    
    let listen_addr = format!("{}:{}", listen_host, port);

    println!("Listening on {}", &listen_addr);
    if let Err(error) = listen(&listen_addr, |out| WSHandler {
        out: out,
        channels: vec![]
    }) {
        println!("Failed to listen: {:?}", error);
    }
}
