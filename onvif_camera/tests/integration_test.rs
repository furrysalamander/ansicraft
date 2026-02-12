use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_device_service_reqwest() {
    // 1. Start the Server in a background task
    let port = 8081; // Use a different port to avoid conflicts
    tokio::spawn(async move {
        onvif_mc2::run(port, false).await;
    });

    // Give it a moment to start
    sleep(Duration::from_millis(500)).await;

    let base_url = format!("http://127.0.0.1:{}/onvif/device_service", port);
    let client = reqwest::Client::new();

    // 2. Test GetDeviceInformation
    let soap_request = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetDeviceInformation/>
  </s:Body>
</s:Envelope>"#;

    let resp = client
        .post(&base_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request)
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        resp.status().is_success(),
        "Response status: {}",
        resp.status()
    );
    let text = resp.text().await.expect("Failed to get response text");
    println!("GetDeviceInformation Response: {}", text);

    assert!(text.contains("MockModelX1"));
    assert!(text.contains("MockCameraFactory"));

    // 3. Test GetSystemDateAndTime
    let soap_request_date = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetSystemDateAndTime/>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&base_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_date)
        .send()
        .await
        .expect("Failed to send GetSystemDateAndTime");
    let text = resp.text().await.unwrap();
    println!("GetSystemDateAndTime Response: {}", text);
    assert!(text.contains("UTCDateTime"));

    // 4. Test Media GetProfiles
    let media_url = format!("http://127.0.0.1:{}/onvif/media_service", port);
    let soap_request_profiles = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <s:Body>
    <trt:GetProfiles/>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&media_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_profiles)
        .send()
        .await
        .expect("Failed to send GetProfiles");
    let text = resp.text().await.unwrap();
    println!("GetProfiles Response: {}", text);
    assert!(text.contains("Profile1"));
    assert!(text.contains("MainStream"));

    // 5. Test Media GetStreamUri
    let soap_request_uri = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>
    <trt:GetStreamUri>
        <trt:StreamSetup>
            <tt:Stream>RTP-Unicast</tt:Stream>
            <tt:Transport>
                <tt:Protocol>RTSP</tt:Protocol>
            </tt:Transport>
        </trt:StreamSetup>
        <trt:ProfileToken>Profile1</trt:ProfileToken>
    </trt:GetStreamUri>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&media_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_uri)
        .send()
        .await
        .expect("Failed to send GetStreamUri");
    let text = resp.text().await.unwrap();
    println!("GetStreamUri Response: {}", text);
    // Expect proxy URL components
    assert!(text.contains("127.0.0.1:8554"));
    assert!(text.contains("app-8F9K44lJ"));
    assert!(text.contains("rtsp://"));

    // 6. Test PTZ GetConfigurations
    let ptz_url = format!("http://127.0.0.1:{}/onvif/ptz_service", port);
    let soap_request_ptz = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  <s:Body>
    <tptz:GetConfigurations/>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&ptz_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_ptz)
        .send()
        .await
        .expect("Failed to send GetConfigurations");
    let text = resp.text().await.unwrap();
    println!("GetConfigurations Response: {}", text);
    assert!(text.contains("PTZCfg1"));

    // 7. Test Device GetCapabilities
    let soap_request_caps = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetCapabilities>
        <tds:Category>All</tds:Category>
    </tds:GetCapabilities>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&base_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_caps)
        .send()
        .await
        .expect("Failed to send GetCapabilities");
    let text = resp.text().await.unwrap();
    println!("GetCapabilities Response: {}", text);
    assert!(text.contains("Media"));
    assert!(text.contains("PTZ"));

    // 8. Test PTZ ContinuousMove
    let soap_request_move = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>
    <tptz:ContinuousMove>
        <tptz:ProfileToken>Profile1</tptz:ProfileToken>
        <tptz:Velocity>
            <tt:PanTilt x="0.5" y="0.0" space="http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace"/>
            <tt:Zoom x="0.0" space="http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace"/>
        </tptz:Velocity>
    </tptz:ContinuousMove>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&ptz_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_move)
        .send()
        .await
        .expect("Failed to send ContinuousMove");
    // Verify success status, response body might be empty or small
    assert!(resp.status().is_success());
    let text = resp.text().await.unwrap_or_default();
    println!("ContinuousMove Response: {}", text);

    // 9. Test PTZ Stop
    let soap_request_stop = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  <s:Body>
    <tptz:Stop>
        <tptz:ProfileToken>Profile1</tptz:ProfileToken>
        <tptz:PanTilt>true</tptz:PanTilt>
        <tptz:Zoom>true</tptz:Zoom>
    </tptz:Stop>
  </s:Body>
</s:Envelope>"#;
    let resp = client
        .post(&ptz_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_request_stop)
        .send()
        .await
        .expect("Failed to send Stop");
    assert!(resp.status().is_success());
    let text = resp.text().await.unwrap_or_default();
    println!("Stop Response: {}", text);
}
