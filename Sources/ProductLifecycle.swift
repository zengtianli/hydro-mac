import AppKit

/// 「配置与更新…」窗口与 `hydromac config` / `hydromac update` 共用的一处定义：产品名、更新渠道、可迁移配置。
/// 只迁移启动计算器、应用快捷键及更新选项；高德凭据、权限、文件路径和计算数据不迁移。
/// 公开变体独立绑定自己的产品身份与 GitHub 正式发行渠道。
enum Lifecycle {
    static let name = "HydroMac · 水利工具箱"
    static let command = "hydromac"
    static let productID = "io.github.zengtianli.HydroMac"
    static let updateSource = AppUpdateSource.github(repository: "zengtianli/hydro-mac")
    static func makeConfiguration() -> AppConfiguration? {
        AppConfiguration(productID: productID, defaultsKeys: [HydroPreferences.calculatorKey, "appUpdateOptions"] + AppShortcuts.portableKeys,
                         defaults: HydroPreferences.defaults, validateDefault: HydroPreferences.validate)
    }

    /// App 侧接线：菜单入口与窗口，运行中的窗口跟随命令的改动（没有可迁移配置时 follow 不做任何事）。
    /// 生命周期自检调的也是这个函数。
    @MainActor static func installApp() {
        let configuration = makeConfiguration()
        AppLifecycleUI.install(name: name, configuration: configuration, updateSource: updateSource)
        HydroSettings.shared.install(configuration: configuration!)
        AppLifecycleCLI.follow(configuration)
    }

    /// 命令侧：本可执行文件就在 .app 里，版本、构建号与 bundle id 取自本包，与窗口显示的是同一处。
    /// 包路径先解析软链：`update install` 替换的是这个路径上的包，而共用层不替换软链本身。
    /// （hydromac 转调时给的已是解析后的路径；这里兜住直接经软链调 App 可执行文件的情形。）
    static func product() -> AppLifecycleCLI.Product {
        var product = AppLifecycleCLI.Product(command: command, name: name, configuration: makeConfiguration(), updateSource: updateSource)
        product.changed = { HydroSettings.notifyChanged() }
        let resolved = Bundle.main.bundleURL.resolvingSymlinksInPath()
        if resolved.path != Bundle.main.bundleURL.path, let bundle = Bundle(url: resolved) { product.bundle = bundle }
        return product
    }
}
