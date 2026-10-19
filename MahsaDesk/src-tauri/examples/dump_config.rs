use mahsadesk::{config_gen, servers, settings};

fn main() {
    // a realistic vmess+ws+tls config, the shape MahsaNG shares most often
    let raw = "vmess://eyJ2IjoiMiIsInBzIjoiU21va2UiLCJhZGQiOiJhY2Nlc3MuaG91c2Uuby5uZXQud29ya2VyLmRldiIsInBvcnQiOiI0NDMiLCJpZCI6Ijg3YjhmNGFiLWZlYmMtNGY1Ny04ZmY0LWQxYjdmNjM1NzBhYiIsImFpZCI6IjAiLCJuZXQiOiJ3cyIsImhvc3QiOiJhY2Nlc3MuaG91c2Uuby5uZXQud29ya2VyLmRldiIsInBhdGgiOiIvdmFjdCNkLyIsInRscyI6InRscyJ9";
    let p = servers::parse_link(raw).expect("vmess must parse");
    let settings = settings::Settings::default();
    println!("{}", serde_json::to_string_pretty(&config_gen::build_config(&settings, &p)).unwrap());
}
