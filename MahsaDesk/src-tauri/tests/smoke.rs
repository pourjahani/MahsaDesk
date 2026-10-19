use base64::Engine;
use mahsadesk::{config_gen, servers};
use serde_json::Value;

#[test]
fn parses_hysteria2_link() {
    let link = "hy2://secretpass@h.example.com:8443?sni=h.example.com&obfs=salamander&obfs-password=obfsbw&insecure=1#Hy2Node";
    let p = servers::parse_link(link).expect("hy2 must parse");
    assert_eq!(p.protocol, "hysteria2");
    assert_eq!(p.address, "h.example.com");
    assert_eq!(p.port, 8443);
    assert_eq!(p.auth, "secretpass");
    assert_eq!(p.method, "salamander");
    assert_eq!(p.encryption, "obfsbw");
    assert_eq!(p.security, "tls");
}

#[test]
fn build_link_roundtrip() {
    for link in [
        "vless://b831381d-6324-4d53-ad4f-8cda48b30811@real.example.com:443?encryption=none&security=reality&sni=www.example.org&fp=chrome&pbk=PUBKEY123&sid=ab01&flow=xtls-rprx-vision&type=tcp#Reality-Node",
        "trojan://pass123@t.example.com:443?sni=t.example.com&type=ws&path=%2Ftrojan#T",
        "hysteria2://secretpass@h.example.com:8443?sni=h.example.com&obfs=salamander&obfs-password=obfsbw&insecure=1#Hy2Node",
        "vmess://eyJ2IjoiMiIsInBzIjoiU21va2UiLCJhZGQiOiJhY2Nlc3MuaG91c2Uuby5uZXQud29ya2VyLmRldiIsInBvcnQiOiI0NDMiLCJpZCI6Ijg3YjhmNGFiLWZlYmMtNGY1Ny04ZmY0LWQxYjdmNjM1NzBhYiIsImFpZCI6IjAiLCJuZXQiOiJ3cyIsImhvc3QiOiJhY2Nlc3MuaG91c2Uuby5uZXQud29ya2VyLmRldiIsInBhdGgiOiIvdmFjdCNkLyIsInRscyI6InRscyJ9",
    ] {
        let p = servers::parse_link(link).expect("must parse");
        let rebuilt = servers::build_link(&p);
        let p2 = servers::parse_link(&rebuilt).expect("rebuilt link must parse");
        assert_eq!(p.protocol, p2.protocol);
        assert_eq!(p.address, p2.address);
        assert_eq!(p.port, p2.port);
        assert_eq!(p.auth, p2.auth);
    }
}

fn default_settings() -> mahsadesk::settings::Settings {
    mahsadesk::settings::Settings::default()
}

#[test]
fn parses_vmess_link() {
    let json = serde_json::json!({
        "v": "2", "ps": "Test VMess", "add": "server.example.com", "port": "443",
        "id": "8c042e26-3f9a-4c72-a72f-e1e8bbdd0f85", "aid": "0",
        "net": "ws", "type": "none", "host": "cdn.example.com", "path": "/wspath",
        "tls": "tls", "sni": "", "scy": "auto"
    });
    let raw = format!("vmess://{}", base64::engine::general_purpose::STANDARD.encode(json.to_string()));
    let p = servers::parse_link(&raw).expect("vmess must parse");
    assert_eq!(p.protocol, "vmess");
    assert_eq!(p.address, "server.example.com");
    assert_eq!(p.port, 443);
    assert_eq!(p.net, "ws");
    assert_eq!(p.security, "tls");
    assert_eq!(p.host, "cdn.example.com");
    assert_eq!(p.path, "/wspath");
    assert_eq!(p.remark, "Test VMess");
}

#[test]
fn parses_vless_reality_link() {
    let link = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@real.example.com:443?encryption=none&security=reality&sni=www.example.org&fp=chrome&pbk=PUBKEY123&sid=ab01&flow=xtls-rprx-vision&type=tcp#Reality-Node";
    let p = servers::parse_link(link).expect("vless must parse");
    assert_eq!(p.protocol, "vless");
    assert_eq!(p.address, "real.example.com");
    assert_eq!(p.port, 443);
    assert_eq!(p.security, "reality");
    assert_eq!(p.flow, "xtls-rprx-vision");
    assert_eq!(p.sni, "www.example.org");
    assert_eq!(p.pbk, "PUBKEY123");
}

#[test]
fn parses_trojan_and_ss() {
    let t = servers::parse_link("trojan://pass123@t.example.com:443?sni=t.example.com&type=ws&path=%2Ftrojan#T").unwrap();
    assert_eq!(t.protocol, "trojan");
    assert_eq!(t.auth, "pass123");
    assert_eq!(t.security, "tls");
    assert_eq!(t.path, "/trojan");

    let s = servers::parse_link("ss://YWVzLTI1Ni1nY206c2VjcmV0@1.2.3.4:8388#SSNode").unwrap();
    assert_eq!(s.protocol, "shadowsocks");
    assert_eq!(s.method, "aes-256-gcm");
    assert_eq!(s.auth, "secret");

    let s2 = servers::parse_link("ss://YWVzLTI1Ni1nY206c2VjcmV0@1.2.3.4:8388#SSNode")
        .unwrap();
    assert_eq!(s2.address, "1.2.3.4");
}

#[test]
fn parses_base64_subscription_payload() {
    let links = "vmess://eyJ2IjoiMiIsInBzIjoiU3ViTm9kZSIsImFkZCI6InMuY29tIiwicG9ydCI6IjQ0MyIsImlkIjoiODEiLCJhaWQiOiIwIiwibmV0IjoidGNwIiwidGxzIjoiIn0=\nvless://u@s.example.com:443?security=tls\n";
    let payload = base64::engine::general_purpose::STANDARD.encode(links);
    let out = servers::parse_payload(&payload);
    assert_eq!(out.len(), 2, "base64 sub with two links must parse");
    assert_eq!(out[0].remark, "SubNode");
}

#[test]
fn generated_config_shape() {
    let raw = "vmess://eyJ2IjoiMiIsInBzIjoiTiIsImFkZCI6ImEuY29tIiwicG9ydCI6IjQ0MyIsImlkIjoiODEiLCJhaWQiOiIwIiwibmV0Ijoid3MiLCJob3N0IjoiYS5jb20iLCJwYXRoIjoiL3BhdGgiLCJ0bHMiOiJ0bHMifQ==";
    let p = servers::parse_link(raw).expect("must parse");
    let mut s = default_settings();
    s.fragment_enabled = true;
    let cfg = config_gen::build_config(&s, &p);

    assert!(cfg.get("inbounds").unwrap().is_array());
    assert_eq!(cfg.pointer("/stats").unwrap().as_object().unwrap().len(), 0);
    let out = cfg.pointer("/outbounds/0").unwrap();
    assert_eq!(out.get("protocol").unwrap().as_str().unwrap(), "vmess");
    assert_eq!(
        out.pointer("/settings/vnext/0/address").unwrap().as_str().unwrap(),
        "a.com"
    );
    assert_eq!(
        out.pointer("/streamSettings/security").unwrap().as_str().unwrap(),
        "tls"
    );
    assert_eq!(
        out.pointer("/streamSettings/wsSettings/headers/Host").unwrap().as_str().unwrap(),
        "a.com"
    );
    assert!(out.pointer("/streamSettings/tlsSettings/fragment").is_some());
}

#[test]
fn cdn_rewrite_swaps_address_keeps_host() {
    let raw = "vmess://eyJ2IjoiMiIsInBzIjoiTiIsImFkZCI6ImEuY29tIiwicG9ydCI6IjQ0MyIsImlkIjoiODEiLCJhaWQiOiIwIiwibmV0Ijoid3MiLCJwYXRoIjoiL3AiLCJ0bHMiOiJ0bHMifQ==";
    let p = servers::parse_link(raw).unwrap();
    let mut s = default_settings();
    s.cdn_mode = "manual".into();
    s.cdn_ip = "104.18.0.25".into();
    let cfg = config_gen::build_config(&s, &p);
    let out = cfg.pointer("/outbounds/0").unwrap();
    assert_eq!(
        out.pointer("/settings/vnext/0/address").unwrap().as_str().unwrap(),
        "104.18.0.25"
    );
    assert_eq!(
        out.pointer("/streamSettings/wsSettings/headers/Host").unwrap().as_str().unwrap(),
        "a.com"
    );
}

#[test]
fn routing_presets_present() {
    let p = servers::parse_link("socks://u:p@127.0.0.1:1080#L").unwrap();
    let mut s = default_settings();
    s.routing_preset = "ir".into();
    let cfg = config_gen::build_config(&s, &p);
    let rules: &Value = cfg.pointer("/routing/rules").unwrap();
    match rules.as_array().unwrap().len() {
        x if x >= 2 => {}
        n => panic!("expected >=2 rules, got {}", n),
    }
}
