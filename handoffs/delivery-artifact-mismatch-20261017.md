# 本轮两项已装验收差异核验（2026-10-17）

本轮 owner 为 Chapter PID `15667` / `chapter-task:bbda1434b2bf4edc8eae278535fcf2a1`；所有写入前使用此身份核对 claims，不新增或释放声明。范围只有 `delivery-native_ui` 与 `delivery-functionality`，总预算 30 分钟；只允许本产品仓内实现/验收修复、本地提交及本交接，不允许推送、发版、装机、部署或改授权。

先读原日志、对应 detail/raw 和原实现后，确定无需修改实现或验收脚本。具体写入范围收敛为本交接和 `build/accept/delivery-artifact-mismatch-20261017/` 的原脚本输出；保留其他会话的 README、历史交接、perf、dist、素材、暂存和后台。共享资源仅为隔离副本的离屏验收，不构建、不启动模拟器、不采样性能、不启动本人已装 App；没有增加锁或新队列。

本轮完成条件是核实两项失败根因、用固定入口检验现有候选、确认真实已装差异仍被拒绝并本地提交交接。两项正式已装验收的通过条件仍是实际装机产物与生产来源及回执一致后，经 Chapter 原 jobs 复验；本轮不能完成这项装机前提，也不把候选通过写成已装通过。

## 原件与实际原因

- native_ui 原日志：`/Users/tianli/Library/Application Support/app-sop/workbench/logs/b57a7dabf4fa4d468545ae34819c7b63.log`；原明细 `perf/acceptance/run-dqfqwbtt/native_ui.detail.json` 的 error 为 `ValueError: Product/receipt mismatch (build): expected 33, found 31`。
- functionality 原日志：`perf/acceptance/run-3hfb0_td/functionality.log`；对应明细 error 相同。
- `scripts/accept/product_checks.py` 默认检查 `/Applications/水利工具箱.app`；Chapter 的 `acceptance_code_environment` 明确绑定实际已装产物，不提供候选替代合同。现场返回 `SOP_BUILD_CURRENT=0`，生产摘要为 `74468440841efde83dbad3195d3b0b8caed55b92c85d4dc7a755ffb4a130190e`。
- 已装包为 1.0.1 (31)，可执行 SHA256 `4e05a5408cd4eb66cce14d60080a6a0cf4ddfcd7ea850df0441605a54f56fd6e`；当前 `perf/build-receipt.json` 绑定 1.0.1 (33)，可执行 SHA256 `3030558412e6873d7c58da689b85783e661f99371d010cc462cf161a6fe93035`。两者签名均实际验证成功，但版本和可执行字节不同。
- `build/release/水利工具箱.app` 是该回执的真实构建 33 候选；Chapter 原 `verify_build_receipt(..., code_only=True)` 返回 true，且实际 `codesign --verify --deep --strict` 退出 0。候选可以复用，不能因此替换固定已装验收的目标。
- 当前回执原件 SHA256 为 `5e46eb4c15519b1badb0a4b2f40e1d45504d16d3f5ee7712fb3a539a873c6e74`，built_at 为 `2026-10-17T08:38:10+08:00`，构建源提交为 `e607aaba8c51a4b3501a7c1a497a683b3653a99e`；本轮未修改回执或任何 App 字节。

这是实际装机差异，既不是夹具错误，也不是环境类 75。不能忽略版本/二进制、把失败改成 75、篡改回执、手写通过，或让默认入口悄悄检查候选来消除失败。此前两次验收脚本修复已经提交，本轮没有新的仓内代码缺陷可修。

## 固定脚本实测与证据

执行的都是原 `scripts/accept/functionality.py` / `scripts/accept/native_ui.py`，使用 `--app` 和 `--receipt` 指明实际目标，`SOP_OUT_DIR` 分别指向本轮独立目录。候选检查清除继承的 `SOP_CODE_BINDING_SHA256` / `SOP_BUILD_CURRENT`，让脚本调用 Chapter 原只读 verifier；不编造生产绑定变量。两项候选各用独立副本、随机偏好域和目录并行运行，无构建或性能资源竞争。

| 目标/检查 | 实际退出/检查数 | raw（相对本仓） | SHA256 |
| --- | --- | --- | --- |
| 候选 functionality | 0 / 36/36 | `build/accept/delivery-artifact-mismatch-20261017/candidate-functionality/functionality-0e49c155b147489ea6b45f01da9301ac/raw.json` | `bd8f1469ca036c931da790a1ff817ac6bb4fa09d0bd8239fbc24ae72146288af` |
| 候选 native_ui | 0 / 44/44 | `build/accept/delivery-artifact-mismatch-20261017/candidate-native-ui/native_ui-ecec0c3c4fc2402c813f4cf25cdc43f5/raw.json` | `06c95a6558472b6bc588ec9ad26dd36c326c61374db4dea47f6ea19f34ef3dc4` |
| 已装 functionality | 1 / 0/1 | `build/accept/delivery-artifact-mismatch-20261017/installed-functionality/functionality-ada81b2cf79d4bee9f90df4b78ce8d85/raw.json` | `58b53fb8fdb1c2c8ca81c261dee04bd62b8c12dd7f65e4b4186ea173121e8dab` |
| 已装 native_ui | 1 / 0/1 | `build/accept/delivery-artifact-mismatch-20261017/installed-native-ui/native_ui-ed73968e55a3473097e2d77850d7644b/raw.json` | `ca91a750816c6f13c38e8cfac7fad0b21ad56fd3230d8d8b83b3caf32fd28600` |

四份 detail/raw 的实际内容和 SHA 已读回。候选两项的生产绑定均为当前且全部 checks=true；已装两项均只记录 execution_completed=false 和原版本不匹配错误，未启动产品用例。已目视候选离屏 `ui/native-ui-content.png` 和 `ui/native-ui-rainfall-results.png`，侧栏与降雨管线/结果表文字可读。最终回读已装仍为构建 31 及上述原 SHA；没有改动本人偏好或已装产物。无实现/测试变更，不追加无关测试或全量 SOP。

## 剩余前提与 CLI 接续

两项固定已装验收仍受阻：需要 Chapter 原装机流程在另外的装机授权范围内，把已装包对齐到当前已验证候选及完整回执；本轮禁止装机，不能从修复任务推导此授权。本轮不新增 jobs、不启动 installer、不运行涵盖其他检查/媒体的 `scripts/delivery/resume.py`。

在本仓 CLI 接手可先只读核原队列和实际差异：

```sh
chapter jobs --app hydro-mac-oss --active --json
chapter sop verify-build-receipt --app hydro-mac-oss --artifact '/Applications/水利工具箱.app' --json
```

现场原队列只有性能作业 `5eeaf3da50104f17beca72bc2b96962a`，因构建槽占用 deferred；这是本轮范围之外的已有作业，保留其 next_attempt_at、预算和自动接续，不将其等待列为这两项的失败原因。没有正在排队的装机作业可供本轮直接接续；不能假称 jobs 已自动完成装机前提。

Chapter 完成实际装机对齐后，由原 jobs 按两项固定入口自动复验；手动诊断时可分别运行 `chapter sop accept --app hydro-mac-oss --check functionality --json` 与 `chapter sop accept --app hydro-mac-oss --check native_ui --json`。候选通过与已装失败原件均保留，不覆盖 perf 下的固定历史证据。

本地提交仅含本交接，不改变他人暂存和工作树改动。仓内核验与交接已完成，两项正式已装验收尚未通过；其必要下一步是原装机流程对齐实际产物，不是继续更改验收断言。
