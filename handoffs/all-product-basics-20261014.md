# HydroMac 公开版四项基本产品功能

Chapter 任务 f0af76232ec95284a07d9cf73b9631a2，2026-10-15；复用派出身份 PID 8198，不释放原 claims。

范围：仅本公开变体 product_basics。写 Sources/HydroMacApp.swift、ContentView.swift、产品设置/生命周期/离屏自检适配器与总部逐字节生命周期副本；scripts/accept/product_basics.py、公开 CLI 薄入口、build.sh、project.yaml 的 product_basics/accept、双语 README、本交接及本轮 build/accept、perf/acceptance 原件。共享资源为本仓 build 目录、Chapter 固定验收锁；不动私有产品和共享生产引擎。

输入：本仓 README/原构建/公开发行记录；固定 PRD-product-basics.md 与 engine/product-basics.json；总部 swift-shared；Initials/MacKit 的实际隔离测试样例，以及 HydroKit 已通过的适配代码。快照白名单为 ProductLifecycle、ProductSettings、ProductBasicsSelfTest 三个已检查的不含客户数据文件；其余计算器、Rust、样本及私有发行源不复制。

完成条件：当前公开身份 io.github.zengtianli.HydroMac 源码真实构建、独立候选 build-receipt；生产原件离屏设置控件/快捷键动作、新进程读回、隔离同步与更新失败恢复完整覆盖固定 14 项；执行 chapter sop accept --app hydro-mac-oss --check product_basics --json，Chapter 自行写正式证据；限定本轮文件提交。预算 30 分钟，失败只在原因/输入改变后重跑。

入场 git dirty：dist/、docs/demo/、perf/acceptance/、perf/delivery-evidence.json；project.yaml 为原本忽略的本机登记，保留原产品说明。没有本产品 CLAUDE 或相关仓内 handoff，沿 README 确定 SwiftUI + Rust 八计算器、自包含 ad-hoc 公开版。

边界：候选不装机、不启动已装实例、不改变真实偏好/账号/iCloud/后台/暂存；UI 离屏、不进 Dock、不合成输入；不推送、公开、部署、公证、发更新源或手机装机。iCloud 本机隔离链路与苹果跨设备传输分开报告。

状态：本轮源码和候选验收完成；未安装，Chapter 后续独立 install 阶段接续。

实现：总部 AppConfiguration/AppLifecycle/AppLifecycleUI/AppLifecycleCLI/AppShortcuts 逐字节 vendor，--check 通过；产品适配启动计算器、可关闭/恢复及修改的应用快捷键和更新选项。⌘R 打开默认计算器，⌘K 搜索，系统保留键及冲突拒绝。可选 iCloud Drive 默认关闭，只同步偏好白名单，凭据/权限/路径/计算数据排除。公开身份/配置/通知与私有版独立，正式更新源为 zengtianli/hydro-mac 的 GitHub Release；公开 ad-hoc 分发提供真实下载入口，沿原 README 手动解压替换，不冒充自动安装。

候选：build/水利工具箱.app，1.0.0 (9)，bundle io.github.zengtianli.HydroMac，主程序 SHA256 72dd62f1f087d7e3f80e77bf1b77f976509cfdb8d8aa7ac772b8c5da54f778ea。独立 build/accept/product-basics-20261015.json 由 app_sop build-receipt 实际包裹 bash build.sh 生成，SHA256 5c29ee7c53d08f0029697f191c6b781cf9f9f85c22fdef3c6c1d6acd9e2dbac0；124 个真实输入均与当前 SHA 一致。构建时仍有本轮未提交源码，因此回执如实记录原 HEAD 和 dirty=true，文件 SHA 才是本候选精确来源；未将回执伪写成装机记录。

必要验证：原 20 个合成 golden 数值回归 20/20；共享 vendor --check、git diff --check、Python/Bash 语法检查通过，包内 hydromac --help 实际运行。Rust/计算器公式与示例未改。浅/深色真实设置与更新离屏截图已读取核对，控件可见、状态与动作一致，全程 prohibited、不显示窗口、不进 Dock、不合成输入。

固定验收默认仍验 /Applications/水利工具箱.app 和 perf/build-receipt.json，本轮显式选择候选：

```sh
HYDROMAC_BASICS_APP="$PWD/build/水利工具箱.app" \
HYDROMAC_BASICS_RECEIPT="$PWD/build/accept/product-basics-20261015.json" \
chapter sop accept --app hydro-mac-oss --check product_basics --json
```

最终退出 0，Chapter product_basics passed，四项全部 passed，固定 14 项完整覆盖，56/56 个实际断言通过。真实控件 target/action、菜单快捷键动作、禁用恢复/保留键/新组合读回、新进程保存/导入拒绝、隔离配置推拉/远端变化/冲突合并/坏文件恢复/不可用回落/关闭保留，均运行真实产品代码。更新在重识别副本内向原 GitHub API 路由注入隔离 URLProtocol，仍经原共享临时 URLSession、GitHub 解码、UI/CLI 检查；缺文件断网、坏 JSON、好源恢复、选项持久化和手动获取边界实际核验。随后对同候选生成的签名新版本 ZIP 真正解压并替换隔离副本，重开 CLI 核配置与隔离凭据保留，不触真实安装。

首轮失败如实保留：macOS 临时网络会话不使用全局 URLProtocol 登记，检查误到真实 GitHub 返回 HTTP 403，48/56。修复测试身份内的 ephemeral 配置注入后重构建，后续成功；未修改总部生产检查器、放宽标准或同因循环。生产网络不可用/HTTP 错误会明确报告失败，不伪称最新版。

最终原件：perf/acceptance/product-basics-1cd35b1740574890875d22d43a396261/raw.json，SHA256 93c4d28796ee75baa63e8f93baf726d236418ca9e5bf0641a13e1140ba6b9755；spec digest 8e832265a1aa330b44c2c412a84e726016b829c3f4e4842a98568043757e5eac。native/native.json、实际 CLI cases、4 张 PNG 和 owner-before/after 在同目录。正式 product_basics.json/detail/log 及 perf/delivery-evidence.json 由 Chapter 原引擎写入，未手写通过记录。

数据与边界：真实公开偏好、支持目录、iCloud 配置/发行条目与安装字节前后相等；实际扫描 /Applications 和 ~/Applications 未发现该公开 bundle，候选未安装。已装私有 HydroKit 不修改、不重启。iCloud 仅本机隔离目录链路通过，苹果服务器/跨设备传输未验证；正式 GitHub 实网成功未宣称已验，受限网络错误已如实观察。无用户决定/本产品受阻，不公证、发布、推送、部署或写更新源，claims 保留。project.yaml 原为忽略的本机登记，沿原边界留本机，不强制加入公开源码；入场 dist/docs/demo/perf 原件保留。

源码提交 c93714b9fc2ac9fdaf4572d86c8d22bed577aa67（18 个本轮文件）；暂存为空，原 untracked 的发行/演示/验收文件保留，未推送。提交后逐个重算 124 个构建输入与候选主程序仍匹配回执；chapter basics --app hydro-mac-oss --json 的 fresh 回读为 passed、无 problems，未重复运行相同验收。候选后续安装入口仍是原 build.sh --install，但本轮不执行；Chapter 独立 install 阶段应先核同 bundle 旧包/真实来源及既有发行签名边界，装机后用默认固定脚本验已装版，不携带候选覆盖环境变量。

## 本机安装阶段 · 2026-10-15 接续

Chapter 同任务，复用 PID 36288。本次用户明确授权本机迭代快装一次、稳定开发者签名，不公证/发布。范围仅 build.sh 原安装段的身份/签名/回滚适配、scripts/sign-local.sh、sign-transfer.py、install-local.py、project.yaml 必要输入登记、perf/build-receipt 与安装/验收原件和本交接；不改四项已通过实现、不展开推广页旧 SOP。共享资源是公开 bundle 的 /Applications 目标、原 Chapter 验收锁与持证机 login_run 任务缓存；不改私有产品、持证机仓库/装机、偏好/账号/iCloud/登录项/后台/他人暂存。

完成条件：Mini 当前源码真实编译，Air 原 Developer ID B9LJH93LA4 登录会话仅签隔离包并 SHA 回传，原 build/install 入口安装一次并逐文件/版本/签名/源回执读回，默认已装版固定验收覆盖 14 项；安装前后配置及其他产品/后台不变，旧版可回溯。有限预算 30 分钟，未知先读原件，同因不循环。

入场现场：公开 bundle 在 /Applications 和 ~/Applications 均未安装；私有 HydroKit 1.0.1 (85) 保留，Developer ID B9LJH93LA4；无 HydroMac/hydro-cli 运行实例。Mini 仅有 CJZ357HGD3 Apple Development，Air 现场有原 B9LJH93LA4 Developer ID；只用原 login_run.py，不转移钥匙。原 public build.sh --install 按 HydroMac 可执行名 pkill、无备份直接删除目录，必须先改为按公开 bundle 校验、保留旧包且不结束其他产品。入场 dirty 仍只有 dist/docs/demo/perf 原 untracked，暂存为空。

状态：安装适配和签名接续中。
