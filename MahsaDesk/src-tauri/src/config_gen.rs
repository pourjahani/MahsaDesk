use crate::servers::ServerProfile;
use crate::settings::Settings;
use serde_json::{json, Value};

const API_PORT: u16 = 10130;

fn to_kv_list(list: &str) -> Vec<String> {
    list.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn sniffing_value(settings: &Settings) -> Value {
    json!({
        "enabled": settings.sniffing,
        "destOverride": ["http", "tls", "quic"],
        "routeOnly": false
    })
}

/// Build the full Xray JSON config for the given active server.
pub fn build_config(settings: &Settings, server: &ServerProfile) -> Value {
    let listen = if settings.proxy_sharing { "0.0.0.0" } else { "127.0.0.1" };
    let sniffing = sniffing_value(settings);

    let mut rules: Vec<Value> = Vec::new();

    // 1. custom user rules first
    let block: Vec<String> = to_kv_list(&settings.custom_block);
    if !block.is_empty() {
        rules.push(json!({"type": "field", "domain": block, "outboundTag": "block"}));
    }
    let direct: Vec<String> = to_kv_list(&settings.custom_direct);
    if !direct.is_empty() {
        rules.push(json!({"type": "field", "domain": direct, "outboundTag": "direct"}));
    }
    let proxy: Vec<String> = to_kv_list(&settings.custom_proxy);
    if !proxy.is_empty() {
        rules.push(json!({"type": "field", "domain": proxy, "outboundTag": "proxy"}));
    }

    // 2. preset rules
    match settings.routing_preset.as_str() {
        "ir" => {
            rules.push(json!({"type": "field", "domain": ["geosite:category-ads-all"], "outboundTag": "block"}));
            rules.push(json!({"type": "field", "domain": ["geosite:category-ir"], "outboundTag": "direct"}));
            rules.push(json!({"type": "field", "ip": ["geoip:private", "geoip:ir"], "outboundTag": "direct"}));
        }
        "ir_cn" => {
            rules.push(json!({"type": "field", "domain": ["geosite:category-ads-all"], "outboundTag": "block"}));
            rules.push(json!({"type": "field", "domain": ["geosite:category-ir", "geosite:cn"], "outboundTag": "direct"}));
            rules.push(json!({"type": "field", "ip": ["geoip:private", "geoip:ir", "geoip:cn"], "outboundTag": "direct"}));
        }
        _ => { /* global: everything proxied */ }
    }
    rules.push(json!({"type": "field", "ip": ["geoip:private"], "outboundTag": "direct"}));

    let outbound = build_outbound(settings, server);

    json!({
        "log": {"loglevel": settings.loglevel},
        "dns": {
            "servers": [settings.remote_dns],
            "queryStrategy": if settings.ipv6 { "UseIP" } else { "UseIPv4" }
        },
        "inbounds": [
            {
                "tag": "socks",
                "listen": listen,
                "port": settings.socks_port,
                "protocol": "socks",
                "settings": {"udp": true, "ip": "127.0.0.1"},
                "sniffing": sniffing
            },
            {
                "tag": "http",
                "listen": listen,
                "port": settings.http_port,
                "protocol": "http",
                "settings": {},
                "sniffing": sniffing_value(settings)
            },
            {
                "tag": "api",
                "listen": "127.0.0.1",
                "port": API_PORT,
                "protocol": "dokodemo-door",
                "settings": {"address": "127.0.0.1"}
            }
        ],
        "outbounds": [
            outbound,
            {"tag": "direct", "protocol": "freedom", "settings": {}},
            {"tag": "block", "protocol": "blackhole", "settings": {}}
        ],
        "routing": {
            "domainStrategy": settings.domain_strategy,
            "rules": rules
        },
        "stats": {},
        "api": {
            "tag": "api",
            "services": ["HandlerService", "StatsService", "LoggerService"]
        },
        "policy": {
            "levels": {
                "0": {
                    "handshake": 4,
                    "connIdle": 300,
                    "uplinkOnly": 0,
                    "downlinkOnly": 0
                }
            },
            "system": {
                "statsInboundUplink": true,
                "statsInboundDownlink": true
            }
        }
    })
}

/// Build the "proxy" outbound for the given profile.
pub fn build_outbound(settings: &Settings, server: &ServerProfile) -> Value {
    // Mahsa CDN rewrite: connect to the chosen CDN IP instead of the raw address,
    // keeping the TLS SNI / Host header identical to the original config.
    let mut address = server.address.clone();
    let mut host_hdr = server.host.clone();
    let mut sni_override: Option<String> = None;
    if settings.cdn_mode == "manual" && !settings.cdn_ip.is_empty() && is_domain(&server.address) {
        if host_hdr.is_empty() {
            host_hdr = server.address.clone();
        }
        address = settings.cdn_ip.clone();
    }
    // Domain fronting (fake host): connect to the CDN domain instead, keep the
    // real destination in the Host header and present the CDN domain as SNI.
    if settings.front_enabled
        && !settings.front_host.trim().is_empty()
        && is_domain(&address)
    {
        if host_hdr.is_empty() {
            host_hdr = server.address.clone();
        }
        address = settings.front_host.trim().to_string();
        sni_override = settings.front_host.trim().to_string().into();
    }

    let protocol = match server.protocol.as_str() {
        "vmess" => "vmess",
        "vless" => "vless",
        "trojan" => "trojan",
        "shadowsocks" => "shadowsocks",
        "socks" => "socks",
        _ => "http",
    };

    let settings_obj: Value = match protocol {
        "vmess" | "vless" => {
            let mut user = serde_json::Map::new();
            user.insert("id".into(), json!(server.auth));
            user.insert("level".into(), json!(0));
            user.insert("email".into(), json!("mahsadesk"));
            if protocol == "vmess" {
                user.insert("alterId".into(), json!(server.alter_id));
                user.insert("security".into(), json!(if server.encryption.is_empty() { "auto" } else { server.encryption.as_str() }));
            }
            if protocol == "vless" {
                user.insert("encryption".into(), json!(if server.encryption.is_empty() { "none" } else { server.encryption.as_str() }));
                user.insert("flow".into(), json!(server.flow));
            }
            json!({"vnext": [{"address": address, "port": server.port, "users": [Value::Object(user)]}]})
        }
        "trojan" => {
            json!({"servers": [{"address": address, "port": server.port, "password": server.auth}]})
        }
        "shadowsocks" => {
            json!({"servers": [{
                "address": address,
                "port": server.port,
                "method": if server.method.is_empty() { "aes-256-gcm" } else { server.method.as_str() },
                "password": server.auth
            }]})
        }
        "socks" => {
            let mut s = json!({"servers": [{"address": address, "port": server.port}]});
            if let Some((user, pass)) = server.auth.split_once(':') {
                s["servers"][0]["users"] = json!([{"user": user, "pass": pass}]);
            }
            s
        }
        _ => {
            let mut s = json!({"servers": [{"address": address, "port": server.port}]});
            if let Some((user, pass)) = server.auth.split_once(':') {
                s["servers"][0]["users"] = json!([{"user": user, "pass": pass}]);
            }
            s
        }
    };

    let mut settings_obj = settings_obj;
    if settings.mux && matches!(protocol, "vmess" | "vless" | "trojan") {
        settings_obj["mux"] = json!({"enabled": true, "concurrency": 8});
    }

    let stream = build_stream(settings, server, &host_hdr, sni_override.as_deref());

    let mut m = serde_json::Map::new();
    m.insert("tag".into(), json!("proxy"));
    m.insert("protocol".into(), json!(protocol));
    m.insert("settings".into(), settings_obj);
    if !stream.is_null() {
        m.insert("streamSettings".into(), stream);
    }
    Value::Object(m)
}

fn build_stream(settings: &Settings, server: &ServerProfile, host_hdr: &str, sni_override: Option<&str>) -> Value {
    let supported = ["tcp", "ws", "kcp", "grpc", "httpupgrade", "h2", "quic"];
    let net_raw = if server.net.is_empty() { "tcp" } else { server.net.as_str() };
    let network = if supported.contains(&net_raw) { net_raw } else { "tcp" };

    let mut stream = serde_json::Map::new();

    let has_tls_security = !server.security.is_empty() && server.security != "none";
    if has_tls_security {
        let mut sni = if !server.sni.is_empty() {
            server.sni.clone()
        } else if !host_hdr.is_empty() {
            host_hdr.to_string()
        } else {
            server.address.clone()
        };
        if let Some(front) = sni_override {
            sni = front.to_string();
        }
        let mut t = serde_json::Map::new();
        t.insert("serverName".into(), json!(sni));
        t.insert("allowInsecure".into(), json!(settings.allow_insecure));
        t.insert("alpn".into(), json!(split_csv(&server.alpn)));
        let fp = if server.fingerprint.is_empty() { "chrome" } else { server.fingerprint.as_str() };
        t.insert("fingerprint".into(), json!(fp));
        if server.security == "reality" {
            t.insert("publicKey".into(), json!(server.pbk));
            t.insert("shortId".into(), json!(server.sid));
            t.insert("spiderX".into(), json!(server.spx));
        } else if settings.ech_enabled {
            // Encrypted Client Hello (Xray v25+): base64 ECHConfigList, or an
            // https:// DoH URL from which the core fetches the HTTPS record.
            let conf = if settings.ech_config.trim().is_empty() {
                "https://1.1.1.1/dns-query"
            } else {
                settings.ech_config.trim()
            };
            t.insert("echConfigList".into(), json!(conf));
        }
        if settings.fragment_enabled {
            t.insert("fragment".into(), json!({
                "packets": settings.fragment_packets,
                "length": settings.fragment_length,
                "interval": settings.fragment_interval
            }));
        }
        stream.insert("security".into(), json!(server.security));
        let key = if server.security == "reality" { "realitySettings" } else { "tlsSettings" };
        stream.insert(key.into(), Value::Object(t));
    }

    let transport: Value = match network {
        "ws" => json!({
            "path": server.path,
            "headers": if host_hdr.is_empty() { Value::Null } else { json!({"Host": host_hdr}) }
        }),
        "httpupgrade" => json!({
            "path": server.path,
            "host": host_hdr
        }),
        "h2" => json!({
            "path": server.path,
            "host": [host_hdr]
        }),
        "grpc" => json!({
            "serviceName": server.path.trim_start_matches('/'),
            "multiMode": false
        }),
        "kcp" => json!({
            "header": {"type": header_type_or_none(server)}
        }),
        "quic" => json!({
            "security": if server.host.is_empty() { "none" } else { server.host.as_str() },
            "header": {"type": header_type_or_none(server)}
        }),
        _ => json!({
            "header": {"type": header_type_or_none(server)}
        })
    };

    stream.insert("network".into(), json!(network));
    let plain_tcp = network == "tcp"
        && transport.pointer("/header/type").and_then(|v| v.as_str()) == Some("none");
    if !plain_tcp {
        let tname = match network {
            "ws" => "wsSettings",
            "httpupgrade" => "httpupgradeSettings",
            "h2" => "httpSettings",
            "grpc" => "grpcSettings",
            "kcp" => "kcpSettings",
            "quic" => "quicSettings",
            _ => "tcpSettings",
        };
        stream.insert(tname.into(), transport);
    }
    Value::Object(stream)
}

fn header_type_or_none(server: &ServerProfile) -> String {
    if server.header_type.is_empty() { "none".into() } else { server.header_type.clone() }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

fn is_domain(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphabetic())
}
