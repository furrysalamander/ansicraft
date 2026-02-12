use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;
use uuid::Uuid;

pub async fn run_discovery_service(service_port: u16, device_uuid: String) {
    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port = 3702;
    // Bind to all interfaces
    let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);

    println!("Binding discovery socket to {}", addr);
    
    // We use std::net::UdpSocket builder to set options before binding to tokio
    let socket = match std::net::UdpSocket::bind(addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind discovery socket: {}", e);
            return;
        }
    };
    
    if let Err(e) = socket.set_nonblocking(true) {
        eprintln!("Failed to set nonblocking: {}", e);
        return;
    }

    // Join multicast group on all interfaces (or default)
    if let Err(e) = socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED) {
        eprintln!("Failed to join multicast group: {}", e);
        // Continue anyway, maybe it works (?) No, discovery won't work without multicast join.
        // But maybe we are in a container where this is tricky.
    }

    let socket = match UdpSocket::from_std(socket) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to convert to tokio socket: {}", e);
            return;
        }
    };

    println!("WS-Discovery service running on 239.255.255.250:3702");

    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, remote_addr)) => {
                let msg = String::from_utf8_lossy(&buf[..len]);
                // Basic check if it is a Probe
                if (msg.contains("Probe") || msg.contains(":Probe")) && 
                   (msg.contains("NetworkVideoTransmitter") || msg.contains("Device")) {
                    
                    println!("Received Probe from {}", remote_addr);
                    
                    // Send ProbeMatch
                    if let Some(response) = build_probe_match(&msg, service_port, &device_uuid) {
                        if let Err(e) = socket.send_to(response.as_bytes(), remote_addr).await {
                            eprintln!("Failed to send ProbeMatch: {}", e);
                        } else {
                            println!("Sent ProbeMatch to {}", remote_addr);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Discovery socket error: {}", e);
            }
        }
    }
}

fn build_probe_match(probe_msg: &str, service_port: u16, device_uuid: &str) -> Option<String> {
    // Extract MessageID from Probe to use as RelatesTo
    // Look for <wsa:MessageID>...</wsa:MessageID> or <MessageID>...</MessageID>
    let msg_id = extract_tag_content(probe_msg, "MessageID")?;

    let host_ip = std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    
    let message_uuid = Uuid::new_v4();

    // Standard ONVIF ProbeMatch response
    let response = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
               xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
               xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
    <soap:Header>
        <wsa:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatch</wsa:Action>
        <wsa:MessageID>urn:uuid:{}</wsa:MessageID>
        <wsa:RelatesTo>{}</wsa:RelatesTo>
    </soap:Header>
    <soap:Body>
        <d:ProbeMatch>
            <d:EndpointReference>
                <wsa:Address>urn:uuid:{}</wsa:Address>
            </d:EndpointReference>
            <d:Types>dn:NetworkVideoTransmitter</d:Types>
            <d:Scopes>onvif://www.onvif.org/type/video_encoder onvif://www.onvif.org/type/audio_encoder onvif://www.onvif.org/hardware/MockCamera onvif://www.onvif.org/name/MockCamera</d:Scopes>
            <d:XAddrs>http://{}:{}/onvif/device_service</d:XAddrs>
            <d:MetadataVersion>1</d:MetadataVersion>
        </d:ProbeMatch>
    </soap:Body>
</soap:Envelope>"#,
        message_uuid,
        msg_id,
        device_uuid,
        host_ip,
        service_port
    );

    Some(response)
}

fn extract_tag_content(xml: &str, tag_name: &str) -> Option<String> {
    // Simple naive extraction, ignoring namespaces prefix variations in tag matching if possible
    // Try with namespace
    if let Some(start) = xml.find(&format!(":{}", tag_name)) {
        // found :TagName, look for closing >
        if let Some(content_start) = xml[start..].find('>').map(|i| start + i + 1) {
            if let Some(end) = xml[content_start..].find(&format!(":{}", tag_name)) {
                 // find the < before the end tag
                 if let Some(close_bracket) = xml[..content_start+end].rfind("</") {
                     return Some(xml[content_start..close_bracket].to_string());
                 }
            }
        }
    }
    
    // Try without namespace (unlikely for proper soap but possible)
    if let Some(start) = xml.find(&format!("<{}", tag_name)) {
         if let Some(content_start) = xml[start..].find('>').map(|i| start + i + 1) {
             if let Some(end) = xml[content_start..].find(&format!("</{}", tag_name)) {
                 return Some(xml[content_start..content_start+end].to_string());
             }
         }
    }

    None
}
