/// 显式声明 UAC 执行级别：文件管理器始终以标准用户令牌运行（asInvoker）。
/// tauri-build 默认 manifest 只有 Common-Controls 依赖、缺 trustInfo，Windows
/// 会退回 UAC 启发式（exe 名含 setup/install 等词会误弹提权）——这里固化语义。
/// 注意 `app_manifest` 是整体替换，需自带 Common-Controls 依赖。
/// LC-009：声明 longPathAware=true，配合系统 LongPathsEnabled 让 >260 字符
/// 长路径在传输/重命名中不再报「name too long」。
const APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>"#;

fn main() {
    // Icon swaps must rebuild the Windows PE resource even when only binary assets change.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/32x32.png");
    println!("cargo:rerun-if-changed=icons/128x128.png");
    println!("cargo:rerun-if-changed=icons/128x128@2x.png");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(APP_MANIFEST)),
    )
    .expect("failed to run tauri-build");
}
