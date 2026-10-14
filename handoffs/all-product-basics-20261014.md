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

状态：本机稳定开发者签名安装及已装版四项验收完成。

实际安装 /Applications/水利工具箱.app，1.0.0 (13)，bundle io.github.zengtianli.HydroMac，主程序 SHA256 84f43b97379697361e90d7a3d95666baa82f6a2d9805b0aec5204264b2f896b3。原 Developer ID Application: tianli Zeng (B9LJH93LA4)、hardened runtime、安全时间戳、deep/strict 签名全部通过；不是 ad-hoc 或新开发身份，未公证。公开版先前没有安装，previous/retired_app 为 null；未把私有 HydroKit 当作旧公开包替换，私有包逐文件 SHA 保持相同。

新增安装适配提交 ab9f714、签名要求路径归一修复 1ba156a。原 build.sh --install 新增 --prepared 当前签名原件入口，不再按执行名 pkill 或删除旧目录；先校验公开 bundle、稳定签名、完整 designated requirement、真实编译回执与全部文件，复制核对后同卷切换，旧包保留回滚并归废纸篓。签名工具只接本公开包，沿总部 login_run.py 的 Air 登录会话，私钥不搬运、不改 Air 源码/装机；输入及返回 ZIP SHA 均验证。只做一次真实安装，没有重复装机。

首次 12 已签成功，安装前读取 codesign 原件发现 -dr- 含绝对 Executable 路径，不能把整个输出作为复制后签名要求；尚未执行真实安装。修为完整 designated => 行比较后，用真正的签名复制包验证要求和每个文件一致，再当前源码重新编译、签成 13。原 12 签名产物/ZIP 与 preflight-build-receipt-12.json 保留；没有修改共享生产签名/分发门，也没有借公证开关绕发行限制。

正式 perf/build-receipt.json 由 app_sop build-receipt 包裹实际 bash build.sh && python3 scripts/sign-transfer.py 生成，源码提交 1ba156a552bde7eb9c7bd7eed5f79d2c94cefb23、dirty=false、127 个输入全部匹配；回执 SHA256 c1a1167531cea2c0f353c6ec428b00232c699421a2c46f0a8497b9d577ed6aa2。签名原件在 build/accept/installed-source-13/水利工具箱.app，正式已装包所有文件与该原件一致。签名/安装来源、前后读回在 build/install-product-basics-20261015/，实际 Air 缓存为 login-run/hydrooss-basics-f0af7623-13，仅该一回任务，helper 已按原接口结束/卸载；未停其他后台。

实际固定入口：

```sh
bash build.sh --install --prepared "$PWD/build/accept/installed-source-13/水利工具箱.app"
env -u HYDROMAC_BASICS_APP -u HYDROMAC_BASICS_RECEIPT chapter sop accept --app hydro-mac-oss --check product_basics --json
```

已装版固定验收最终退出 0，scope=installed，四项 passed，固定 14 项必测行为完整、56/56 真实断言通过；不是候选覆盖。最后原件 perf/acceptance/product-basics-23849ebeca8a42158408e91dee58584d/raw.json，SHA256 8e2367eb3addacdc6e4be4b9fb654771f3928040dbaf7b524828e4ec4c9a2925；spec digest a2eb9d53b41091848eed0e8ca82732016b2a1a8d13d7dd4a16516aaa348dae32。原生离屏设置/快捷键动作、独立进程持久化、隔离同步推拉/冲突/坏数据恢复/本地回落、GitHub 真正解码与错误恢复、手动升级配置保留仍使用真实已装原件的隔离副本。浅深色 PNG 原件在同目录；新设置深色/更新浅色截图再次读取核对。此前 121 个 Sources/cli/icon 输入逐 SHA 未变，沿用已通过 20/20 数值回归，不重复未变输入。Chapter 自行写入 perf/acceptance/product_basics.json/detail/log 与 delivery-evidence.json；其原运行中的另一个 app_sop 报告锁未抢夺，fresh chapter basics 回读已是 passed、无 problems。

生产读回：包内 preferences status 为 annual、默认 ⌘R 开、⌘K 搜索；config status 显示有可迁移偏好但当前 keys 空、同步关闭、app_running=false。源码/主程序/图标/签名及实际文件清单全部匹配，未合成输入、上屏或进 Dock；生产 App 未启动，装前装后生产 HydroMac/hydro-cli 进程仍为空，不重启后台/登录项。

数据保留的精确边界：隔离固定验收自己的 owner-before/after 一致；安装前后私有安装字节、公开配置目录及真实云端配置一致，用户偏好仍全部未设且同步/自动检查未开启。更长的装后观察发现公开默认域从空字典新增 appUpdateState（最新版本 1.0.0 的本机检查历史，多次时间更新），因此完整 plist 字节不相等；本会话未直接执行生产 update check，不能证实该记录由哪个外部后台写入，不把它归给隔离夹具，也不删除/覆盖记录或停止现有后台。原 before/after-install、data-preservation-readback.json 如实保留这个差异，未把不相等写成相等。

本轮无本人待决定项或产品受阻。iCloud 仍只证明本机隔离目录链路，苹果服务器/跨设备传输未验；本机快装不等于公证/发行，GitHub 公开 Release 与更新源未改。未推送、公开发布、部署、发更新源或手机装机；已装私有版、其他 dirty/暂存及后台保留，原 claims PID 36288 保持不释放。本轮仅处理已授权四项及本机安装，不展开缓存中推广页旧缺项。
