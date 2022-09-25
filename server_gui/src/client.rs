use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::default::Default;

pub struct Client {
    write_stream: TcpStream,
    read_stream: Arc<Mutex<TcpStream>>,
    pc_name: String,
    ip: String,
    disconnected: Arc<Mutex<bool>>,

    pub is_read: bool,

    cmd_output: Arc<Mutex<Vec<String>>>,
    file_list: Arc<Mutex<Vec<String>>>,
    folder_path: Arc<Mutex<String>>
}

impl Client {
    pub fn new(write_stream: TcpStream, pc_name: String, ip: String,
               cmd_output: Arc<Mutex<Vec<String>>>,
               file_list: Arc<Mutex<Vec<String>>>,
                folder_path: Arc<Mutex<String>>) -> Self {
        Client {
            write_stream: write_stream.try_clone().unwrap(),
            read_stream: Arc::new(Mutex::new(write_stream.try_clone().unwrap())),
            pc_name,
            ip,
            disconnected: Arc::new(Mutex::new(false)),
            is_read: false,
            cmd_output,
            file_list,
            folder_path
        }
    }

    pub fn write(&mut self, msg: &str) -> bool {
        let mut vec = msg.as_bytes().to_vec();
        "\n----------ENDOFCONTENT----------\n".as_bytes().iter().for_each(|b| vec.push(*b));
        let rst = self.write_stream.write(vec.as_slice());
        if let Ok(_) = rst {
            true
        } else {
            false
        }
    }

    pub fn read(&mut self) {
        let mut stream_clone = Arc::clone(&self.read_stream);
        let cout = Arc::clone(&self.cmd_output);
        let fl = Arc::clone(&self.file_list);
        let folder_path = Arc::clone(&self.folder_path);
        let disconnected = Arc::clone(&self.disconnected);
        std::thread::spawn(move || {
        // tokio::task::spawn(async move {
            loop {
                let rst = read_buffer(stream_clone.clone());
                match rst {
                    Ok(ref array) => {
                        let lines = String::from_utf8_lossy(array.as_slice())
                            .split("\n").collect::<Vec<&str>>()
                            .iter().map(|f| f.to_string())
                            .collect::<Vec<String>>();

                        for t in &lines {
                            let text = t.trim().to_string();

                            if text.starts_with("shellout:") {
                                let arr = text.split("shellout:").collect::<Vec<&str>>();
                                cout.lock().unwrap().push(arr[1].to_string());
                            }

                            if text.starts_with("fm||") {
                                let arr: Vec<&str> = text.split("||").collect();
                                fl.lock().unwrap().push(format!("{}||{}", arr[2], arr[1]))
                            }

                            if text.starts_with("disks||") {
                                let arr: Vec<&str> = text.split("||").collect();
                                fl.lock().unwrap().push(format!("{}||dir", arr[1].to_string()));
                            }

                            if text.starts_with("current_folder||") {
                                let arr: Vec<&str> = text.split("||").collect();
                                *folder_path.lock().unwrap() = arr[1].to_string();
                            }

                            if text.starts_with("downloadfile||") {
                                let arr: Vec<&str> = text.split("||").collect();
                                let file_name = arr[1].clone();
                                let len = 14 + file_name.len() + 2;
                                let file = &array[len..array.len()];

                                std::fs::write(file_name, file);

                                // let arr = &array[14..array.len()];
                            }
                        }
                        // println!("Received: {}", std::str::from_utf8(array.as_slice()).unwrap());
                    },
                    Err(_) => {
                        println!("Disconnected!");
                        break;
                    }
                }
            }
            *disconnected.lock().unwrap() = true;
        });
    }

    pub fn get_name(&self) -> String {
        self.pc_name.clone()
    }

    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }

    pub fn is_disconnect(&self) -> bool {
        *self.disconnected.lock().unwrap()
    }
}

fn read_buffer(mut stream: Arc<Mutex<TcpStream>>) -> Result<Vec<u8>, ()> {
    let mut buffer = [0 as u8; 1024 * 1024]; // 1MB Buffer
    let mut vect: Vec<u8> = Vec::new();
    let mut _stream = stream.lock().unwrap();
    match _stream.read(&mut buffer) {
        Ok(size) => {
            let buf_str = String::from_utf8_lossy(&buffer[0..size]);

            if size <= 0 {
                return Err(())
            }

            for v in &buffer[0..size] {
                vect.push(*v);
            }

            if !buf_str.ends_with("\n----------ENDOFCONTENT----------\n") {
                loop {
                    let _size = _stream.read(&mut buffer).unwrap();

                    for v in &buffer[0.._size] {
                        vect.push(*v);
                    }

                    if String::from_utf8_lossy(vect.as_slice()).ends_with("\n----------ENDOFCONTENT----------\n") {
                        vect = vect[0..(vect.len()-34)].to_vec();
                        break;
                    }
                }
            } else {
                vect = vect[0..vect.len()-34].to_vec();
            }
            Ok(vect)
        },
        Err(_) => {
            Err(())
        }
    }
}