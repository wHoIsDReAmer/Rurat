extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // res.set_icon("resource/icon.ico");
        res.compile().unwrap()
    }
}