import SwiftUI

/// 水利工具箱 HydroMac —— 原生 SwiftUI 壳 + vendored Rust hydro-cli 后端(JSON 契约)。
/// macOS 专属原生版;计算核 vendored 自作者上游 hydro 项目。
@main
struct HydroMacApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 900, minHeight: 600)
        }
        .windowResizability(.contentSize)
        .defaultSize(width: 1180, height: 760)
        .commands { PaletteCommands() }
    }
}
