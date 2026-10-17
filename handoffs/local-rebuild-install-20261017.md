# 本机快装入口与已装验收复验（2026-10-17）

本轮 owner 为 Chapter PID `87139` / `chapter-task:d03106dc3b6449df89cd028cf1c4ddad`；写前沿该精确身份核对 claims，不新增同范围声明、不释放原声明。范围为本产品 `project.yaml` 的 `sop.install`、新增 `scripts/build-local.py`、本交接，以及原构建/安装/固定验收入口产生的 build/perf 证据；保留开工已有 README、旧 handoff、perf、dist、素材、暂存和后台。预算 40 分钟，先完成本次装机及六项必要验收，不追加性能、媒体、网页、发行或其他产品工作。

共享资源仅使用既有 `sim_lane.BuildSlot(wait=0)`、与 test_product 共用的本产品输出锁及原数据保留安装事务；不占模拟器、输入设备、窗口焦点或剪贴板。快装只进行本机 Developer ID/runtime/timestamp 签名，不公证、不发布、不推送更新源；保留 bundle ID、用户数据、登录项和服务配置。

完成条件：Chapter rebuilder 能发现明确入口；入口真实重建并产生原 build-receipt，按既有 installer 装到本机；读回实际版本、来源和运行状态；以 Chapter 固定 accept 复验原五项失败及 installed_icon；仅提交本轮文件，按实际仓库可见性执行本次指令（私有仓推送、公开仓只本地提交），不改其他组件或私有父仓。

开工已装为 1.0.1 (31)，回执与旧候选为 (33)，五项失败均为实际 build 不一致。原后台仅有 deferred 性能作业，保留原队列；没有运行中的已装产品实例。公开产品仓开工领先 5 个完成提交、无暂存；远端 `zengtianli/hydro-mac` 为 PUBLIC。`project.yaml` 被产品仓及私有 Apps 父仓忽略，不能将本机 harness 登记强加到公开仓。

## 本轮结果

1. `project.yaml` 登记 `sop.install: PYTHONDONTWRITEBYTECODE=1 python3 scripts/build-local.py --install`，Chapter `machine_work.rebuilder()` 实际读回同一命令及 `sop.install` 来源。此本机登记沿原 `.gitignore` 保持不进公开 Git；没有为强行提交登记而修改忽略规则、父仓索引或其他产品。
2. 原 `build.sh` 真实 Swift/Rust 构建，本机共享 `distribution_grade.sign()` 签名，由 Chapter 原 `build_receipt()` 在实际构建前后绑定 143 个输入。候选位于 `build/local-fast-_rqv4vxi/水利工具箱.app`，该目录保留原构建 (33) 的回执、新构建回执、签名与全文件 manifest；没有改写旧候选或旧来源。
3. 原 `scripts/install-local.py` 成功把 `/Applications/水利工具箱.app` 从 1.0.1 (31) 对齐到 **1.0.1 (37)**；安装后全部文件与 manifest 一致，Developer ID、designated requirement 及 bundle ID 不变，没有终止既有进程。旧包由原事务保留到废纸篓，用户数据/登录项/常驻配置未修改；四项基本功能各自的用户数据和已装字节未变断言通过。
4. `chapter sop verify-build-receipt --app hydro-mac-oss --artifact '/Applications/水利工具箱.app' --json` 返回 `ok:true`，原因为“装机可执行文件、图标与当前构建输入均匹配 receipt”。新可执行 SHA256 为 `b3b81eb5f7fe41f70b1f8b790384f05330417420964f0a3a91db57694fd78bfe`；回执 SHA256 为 `12f789763c18906199867ce2dbe7ecadb1a027e6ac17a422f4c68ae111d068e6`，源码摘要为 `35fcee73fa366a630e6bf07963c36626b007fef89e17e208df50d35cb777360f`，实际构建上下文为 `ac7784a`。提交交接不会改变实际生产输入，不冒充又重建了一次。
5. 已装 CLI `status --json` 读回 1.0.1 (37)、公开 bundle ID、八计算器及 `background_service:false`。另以真实安装路径 `--background` 静默启动一次，`proc_pidpath(19338)` 读回实际安装可执行文件，SHA 与回执相同；进程生成离屏 ready 快照后仅回收本次自建实例，前后均无遗留产品实例，不抢焦点、不增加常驻。原件为 `build/local-fast-_rqv4vxi/installed-runtime.json`；首次诊断因 macOS `ps` 截断多字节列导致解码失败，实例已清理，改用 `proc_pidpath` 后读取成功。该原始 ready PNG 含透明区域，只作运行探针，不作为界面/推广素材；实际界面视觉核验沿下述固定脚本截图。

## 固定验收与原始证据

依次执行 `chapter sop accept --app hydro-mac-oss --check <name> --json`，六项均返回 `passed`、退出 0、`input_stable:true`。以下原始 raw 及固定 JSON SHA 均实读核对；`perf/delivery-evidence.json` 全由 Chapter 写入，没有手写通过。

| 检查 | 实际检查 | 固定证据目录 | raw SHA256 |
| --- | --- | --- | --- |
| functionality | 36/36 | `perf/acceptance/run-13n4ijqk/` | `0c1a90949fdc6272a680e824cf273fbf20fcc20d5c49e7ff88321865cfd0c2fb` |
| recovery | 8/8 | `perf/acceptance/run-q2tmmh65/` | `117aa0acf8d8fee1508909e447f82e749f3e4e80f009956c7993ff90f1ba6024` |
| privacy | 37/37 | `perf/acceptance/run-u52b0gdl/` | `bdc4faa9535fdb5c6fe77acd346318308127e0ea92d240dfd45be6563c94c5d0` |
| native_ui | 44/44 | `perf/acceptance/run-kev1kqbq/` | `a8b7e22dac2e96c642124a3bb09483545a85a969680ca0471f47e3b111af7a66` |
| product_basics | 56/56 | `perf/acceptance/run-60yskj1g/` | `2e97c714ddd59751caaf6ad11ece2e76ebd01a2d1b27417574f63aa86ea2d110` |
| installed_icon | 系统离屏图标比对通过 | `perf/acceptance/run-uy8rvlru/` | 固定 JSON：`10c919a62fdbf29b42d26655993b2ee2289b5cb51cf805ea800860f00bfb9643` |

`product_basics` 分别为设置 18、快捷键 10、iCloud 9、更新 19 项真实检查；同步往返为隔离目录链路，Apple 跨设备投递未测，更新沿隔离传输/真实新版手动安装边界，不扩大为生产发行。已逐图查看本轮固定 native_ui 的 `native-ui-content.png` 与 `native-ui-rainfall-results.png`，侧栏、空态、降雨动作和结果表文字可读。没有新增源码功能缺陷，不重复输入未变的其他通过项，不动性能或媒体原队列。

## 提交边界与接续

本组件唯一产品远端 `zengtianli/hydro-mac` 为 PUBLIC，本次明确要求“公开仓只本地提交”，因此不推送、不对外同步；只提交新增快装脚本和本交接，开工其他改动/暂存保持原样，登记与机器证据保持本地。私有 Apps 父仓不追踪该被忽略的嵌套登记，本轮没有私有组件仓可推送，未修改父仓。远端分支只读回为 `448bfe7599201f34f30b49115d977a956f5fc2b2`；本地领先 5 个原完成提交及本轮提交，不宣称已同步远端。两个要求推送的通用完成谓词与本次明确“公开仓只本地提交”冲突时，执行本次具体边界。

已按两种要求读取 `chapter xcode-cloud status --app hydro-mac-oss --json` 和 `chapter xcode-cloud status --repo . --json`，该仓均为 `push_allowed:true`、workflow 仍全部手动；没有共享前提受阻。仓内没有 `.github/workflows/` 或 `ci_scripts/` 登记。真实提交仍保留凭证扫描和原 Git hooks，仅用原 `GIT_FOLLOW_LOCAL_ONLY=1` 避免跨机跟随；本轮未触发 CI、上传、部署、发版或更新源推送。

以后本组件当前源码过期时，Chapter 可自动调用上述 `sop.install`，共享构建槽/本产品输出锁忙时返回 75 交原队列；本轮不新建并行任务、不释放派出者声明。CLI 诊断用 `chapter sop verify-build-receipt --app hydro-mac-oss --artifact '/Applications/水利工具箱.app' --json`；只有源码或原因变化才沿固定 `chapter sop accept --app hydro-mac-oss --check <name> --json` 复验。本次本机装机与六项复验已完成；签名未公证是本次明确快装边界，不作为正式发行或公开下载包。
