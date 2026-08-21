//! Build script to embed Windows PE icon resources and administrator execution level manifest.

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/copyfile.ico");

    let is_release = std::env::var("PROFILE").is_ok_and(|p| p == "release");
    let execution_level = if is_release {
        "requireAdministrator"
    } else {
        "asInvoker"
    };

    let manifest = format!(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="{execution_level}" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}" />
      <!-- Windows 8.1 -->
      <supportedOS Id="{{1f676c76-80e1-4239-95bb-83d0f6d0da78}}" />
      <!-- Windows 8 -->
      <supportedOS Id="{{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}}" />
      <!-- Windows 7 -->
      <supportedOS Id="{{35138b9a-5d96-4fbd-8e2d-a2440225f93a}}" />
    </application>
  </compatibility>
</assembly>
"#
    );

    res.set_manifest(&manifest);

    if let Err(e) = res.compile() {
        eprintln!("Aviso: Falha ao compilar recursos do Windows: {e}");
    }
}

#[cfg(not(windows))]
fn main() {}
