use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};

use crate::client;

fn get_client(mut stream: TcpStream) -> Vec<String>{
    stream.write(b"r1\n----------ENDOFCONTENT----------\n").unwrap();

    let received = read_buffer(stream.try_clone().unwrap()).unwrap();
    let value = String::from_utf8_lossy(received.as_slice()).to_string();

    value.split("::").collect::<Vec<&str>>()
        .iter().map(|x| x.to_string()).collect()
    // println!("{}", value);
}

#[derive(Default, Clone)]
pub struct Server {
    pub clients: Arc<Mutex<Vec<client::Client>>>,
    port_list: Vec<String>,

    cmd_output: Arc<Mutex<Vec<String>>>,
    file_list: Arc<Mutex<Vec<String>>>,
    folder_path: Arc<Mutex<String>>
}

impl Server {
    pub fn new() -> Self {
        Server::default()
    }

    pub fn set_cout(&mut self, cmd_output: Arc<Mutex<Vec<String>>>) {
        self.cmd_output = cmd_output;
    }
    pub fn set_fl(&mut self, file_list: Arc<Mutex<Vec<String>>>) {
        self.file_list = file_list;
    }
    pub fn set_folder_path(&mut self, fp: Arc<Mutex<String>>) {
        self.folder_path = fp;
    }

    pub fn listen_port(&mut self, port: String) -> bool {
        let listener = TcpListener::bind("0.0.0.0:".to_string() + port.as_str());

        if let Ok(stream) = listener {
            let vec = Arc::clone(&self.clients);
            let cout = Arc::clone(&self.cmd_output);
            let fl = Arc::clone(&self.file_list);
            let fp = Arc::clone(&self.folder_path);
            tokio::task::spawn(async move {
                for i in stream.incoming() {
                    println!("New Client!");

                    if let Err(_) = i {
                        break;
                    }

                    let stream = i.unwrap();
                    let ip = stream.try_clone().unwrap()
                        .peer_addr().unwrap().ip().to_string();
                    let info = get_client(stream.try_clone().unwrap());

                    let mut client = client::Client::new(stream.try_clone().unwrap()
                        , info[1].clone()
                        , ip.clone()
                        , Arc::clone(&cout)
                        , Arc::clone(&fl)
                        , Arc::clone(&fp));

                    vec.lock().unwrap().push(client);
                }
            });
            true
        } else {
            false
        }
    }
}

// FN
fn read_buffer(mut stream: TcpStream) -> Result<Vec<u8>, ()> {
    let mut buffer = [0 as u8; 1024]; // 1MB Buffer
    match stream.read(&mut buffer) {
        Ok(size) => {
            if size <= 0 {
                return Err(())
            }

            let mut vect: Vec<u8> = Vec::new();
            for v in &buffer {
                if *v > 0 as u8 {
                    vect.push(*v);
                }
            }

            if size > 1024 {
                let repeat = size / 1024;
                for i in 0..repeat {
                    stream.read(&mut buffer).unwrap();
                    for v in &buffer {
                        if *v > 0 as u8 {
                            vect.push(*v);
                        }
                    }
                }
            }
            Ok(vect)
        },
        Err(_) => {
            Err(())
        }
    }
}