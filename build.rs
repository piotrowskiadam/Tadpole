fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::MSResource::new();
        res.set_icon("tadpole.ico");
        res.compile().unwrap();
    }
}
