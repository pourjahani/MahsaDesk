use base64::Engine;
use mahsadesk::{config_gen, psiphon, servers, tools};
use serde_json::Value;
use std::path::Path;

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

// ---------------------------------------------------------------------------
// ECH + domain fronting
// ---------------------------------------------------------------------------

#[test]
fn ech_config_list_inserted() {
    let raw = "trojan://pass@a.com:443?sni=a.com&type=tcp#E";
    let p = servers::parse_link(raw).unwrap();
    let mut s = default_settings();
    s.ech_enabled = true;
    s.ech_config = "".into();
    let cfg = config_gen::build_config(&s, &p);
    let out = cfg.pointer("/outbounds/0").unwrap();
    assert_eq!(
        out.pointer("/streamSettings/tlsSettings/echConfigList")
            .unwrap()
            .as_str()
            .unwrap(),
        "https://1.1.1.1/dns-query",
        "empty ech_config must fall back to the DoH URL"
    );
    // explicit base64 config must pass through
    s.ech_config = "dWJxVGVzdA==".into();
    let cfg = config_gen::build_config(&s, &p);
    assert_eq!(
        cfg.pointer("/outbounds/0/streamSettings/tlsSettings/echConfigList")
            .unwrap()
            .as_str()
            .unwrap(),
        "dWJxVGVzdA=="
    );
    // reality profiles never get ECH
    let rp = servers::parse_link("vless://u@r.example.com:443?security=reality&pbk=PUBKEY123&sid=ab01#R").unwrap();
    s.ech_enabled = true;
    let cfg = config_gen::build_config(&s, &rp);
    assert!(cfg
        .pointer("/outbounds/0/streamSettings/realitySettings/echConfigList")
        .is_none());
}

#[test]
fn domain_fronting_swaps_address_and_sni_keeps_host() {
    let raw = "vmess://eyJ2IjoiMiIsInBzIjoiTiIsImFkZCI6InJlYWwtc2VydmVyLmNvbSIsInBvcnQiOiI0NDMiLCJpZCI6IjgxIiwiYWlkIjoiMCIsIm5ldCI6IndzIiwicGF0aCI6Ii9wIiwidGxzIjoidGxzIn0=";
    let p = servers::parse_link(raw).unwrap();
    let mut s = default_settings();
    s.front_enabled = true;
    s.front_host = "cdn.example.net".into();
    let cfg = config_gen::build_config(&s, &p);
    let out = cfg.pointer("/outbounds/0").unwrap();
    assert_eq!(
        out.pointer("/settings/vnext/0/address").unwrap().as_str().unwrap(),
        "cdn.example.net",
        "connect address must become the fronting CDN"
    );
    assert_eq!(
        out.pointer("/streamSettings/tlsSettings/serverName")
            .unwrap()
            .as_str()
            .unwrap(),
        "cdn.example.net",
        "SNI must present the CDN domain"
    );
    assert_eq!(
        out.pointer("/streamSettings/wsSettings/headers/Host")
            .unwrap()
            .as_str()
            .unwrap(),
        "real-server.com",
        "Host header must keep the real destination"
    );
    // off by default
    let cfg = config_gen::build_config(&default_settings(), &p);
    assert_eq!(
        cfg.pointer("/outbounds/0/settings/vnext/0/address")
            .unwrap()
            .as_str()
            .unwrap(),
        "real-server.com"
    );
}

// ---------------------------------------------------------------------------
// Psiphon config builder
// ---------------------------------------------------------------------------

#[test]
fn psiphon_config_builder_defaults_and_keys() {
    let mut s = default_settings();
    let cfg = psiphon::build_config(&s, None, Path::new("data"), Path::new("missing.json"));
    assert_eq!(
        cfg.get("LocalSocksProxyPort").unwrap().as_u64().unwrap(),
        7501
    );
    assert_eq!(
        cfg.get("LocalHttpProxyPort").unwrap().as_u64().unwrap(),
        7502
    );
    assert_eq!(cfg.get("PropagationChannelId").unwrap(), "FFFFFFFFFFFFFFFF");
    assert_eq!(cfg.get("SponsorId").unwrap(), "FFFFFFFFFFFFFFFF");
    assert!(cfg.get("EmitDiagnosticNotices").unwrap().as_bool().unwrap());
    assert!(cfg.get("UpstreamProxyURL").is_none(), "only mode: no upstream");

    // psiphon-after: upstream to xray's socks
    s.psiphon_protocol = "normal".into();
    s.psiphon_region = "de".into();
    s.psiphon_aggressive = true;
    let cfg = psiphon::build_config(&s, Some(10808), Path::new("data"), Path::new("missing.json"));
    assert_eq!(
        cfg.get("UpstreamProxyURL").unwrap().as_str().unwrap(),
        "socks5://127.0.0.1:10808"
    );
    assert_eq!(cfg.get("EgressRegion").unwrap().as_str().unwrap(), "DE");
    assert_eq!(cfg.get("AggressiveEstablishment").unwrap().as_bool().unwrap(), true);
    let protos = cfg
        .get("LimitTunnelProtocols")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(protos.iter().any(|p| p.as_str().unwrap() == "TLS-OSSH"));
}

// ---------------------------------------------------------------------------
// DNS scanner primitives
// ---------------------------------------------------------------------------

#[test]
fn dns_query_packet_shape() {
    let pkt = tools::dns_query_packet("a.example.co");
    assert_eq!(pkt[2], 0x01, "QR=0 RD=1");
    assert_eq!(&pkt[4..6], &[0x00, 0x01], "one question");
    let body = String::from_utf8_lossy(&pkt[12..]);
    assert!(body.contains("example"));
    assert!(pkt.ends_with(&[0x00, 0x01, 0x00, 0x01]), "A/IN");
}

#[test]
fn parse_dns_answer_reads_a_records() {
    // hand-built response: header + question + one A answer
    let mut pkt: Vec<u8> = vec![0xAB, 0xCD, 0x81, 0x80, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00];
    for label in ["e", "x", "co"] {
        pkt.push(label.len() as u8);
        pkt.extend_from_slice(label.as_bytes());
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]); // A IN
    // answer 1: compressed name pointer to offset 12
    pkt.extend_from_slice(&[0xC0, 0x0C, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x04]);
    pkt.extend_from_slice(&[142, 250, 74, 14]);
    // answer 2: literal name (example.co\x00), then A record rdata
    pkt.extend_from_slice(&[0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e']);
    pkt.extend_from_slice(&[0x02, b'c', b'o', 0x00]);
    pkt.extend_from_slice(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x04]);
    pkt.extend_from_slice(&[142, 250, 74, 99]);
    // answer 3: non-A (CNAME) must be skipped
    pkt.extend_from_slice(&[0xC0, 0x0C, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x03]);
    pkt.extend_from_slice(&[b'w', b'w', b'w']);
    let ips = tools::parse_dns_answer(&pkt);
    assert_eq!(ips.len(), 2, "A records parsed, CNAME skipped: {:?}", ips);
    assert_eq!(ips[0], "142.250.74.14");
    assert_eq!(ips[1], "142.250.74.99");
}

#[test]
fn ip_presets_expand_to_valid_ips() {
    let ips = tools::expand_candidate_ips("cloudflare", "", 8);
    assert!(!ips.is_empty());
    assert!(ips.len() <= 64);
    for ip in &ips {
        ip.parse::<std::net::Ipv4Addr>()
            .unwrap_or_else(|e| panic!("bad ip {} : {}", ip, e));
    }
    // custom list: raw IPs pass through
    let custom = "1.2.3.4\n5.6.7.8";
    let ips = tools::expand_candidate_ips("custom", custom, 8);
    assert_eq!(ips, vec!["1.2.3.4", "5.6.7.8"]);
}
