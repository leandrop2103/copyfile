#[cfg(windows)]
extern "system" {
    fn MessageBeep(uType: u32) -> i32;
}

/// Sends a native Windows notification with optional completion sound.
pub fn send_completion_notification(title: &str, message: &str, play_sound: bool) {
    let title_owned = title.to_string();
    let message_owned = message.to_string();

    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            if play_sound {
                // Play subtle Windows Asterisk/Notification chime
                unsafe {
                    let _ = MessageBeep(0x0000_0040);
                }
            }

            // Dispatch modern Windows Toast Notification via PowerShell WinRT XML
            let script = format!(
                r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = @"
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>{title}</text>
            <text>{message}</text>
        </binding>
    </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = New-Object Windows.UI.Notifications.ToastNotification $xml
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("CopyFile").Show($toast)
"#,
                title = title_owned.replace('"', "`\""),
                message = message_owned.replace('"', "`\"")
            );

            let _ = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
        }

        #[cfg(not(windows))]
        {
            let _ = (title_owned, message_owned, play_sound);
        }
    });
}

#[cfg(windows)]
trait CommandExt {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl CommandExt for std::process::Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt as WindowsCommandExt;
        WindowsCommandExt::creation_flags(self, flags)
    }
}
