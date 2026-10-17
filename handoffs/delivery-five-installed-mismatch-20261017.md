# 五项固定已装验收：同一实际产物差异（2026-10-17）

本轮任务为 Chapter `chapter-task:c83409cd70b94cd7977e08eb288921ec`，沿派出者 PID `35170` 核对 claims；不另声明或释放。范围仅 delivery-recovery、delivery-privacy、delivery-product_basics、delivery-native_ui、delivery-functionality，预算上限 30 分钟。禁止推送、发版、装机、部署或改授权，保留他人 README、旧 handoff、perf、dist、素材、暂存及后台。

本轮先核原日志及其 detail/raw、共用 bundle_binding、product_basics、Chapter 原只读生产绑定与回执核验器。实际原因确定后，具体写入范围收敛为本交接；没有可据实修复的实现或验收脚本缺陷，不为凑修改而放宽校验。完成条件为核实五项共同原因、保存真实版本/来源/签名结论和接续命令、本地提交交接。五项正式通过仍须实际装机一致后由原 jobs 固定复验，不将本轮排查完成写成验收通过。

共享资源只有只读文件、Chapter 查询和 codesign 验签；无构建、模拟器、GUI 启动或性能采样，不新增锁或队列。本轮无代码变化，复用输入未变的原失败证据及候选证据，不重复测试或执行固定失败命令。未手写或改写任何通过证据。

## 原失败原件

原日志位于 `~/Library/Application Support/app-sop/workbench/logs/`。五项均为 `ValueError: Product/receipt mismatch (build): expected 33, found 31`，在产品用例启动之前被拒绝；不是 Finder/Safari 假设、私有 claims 环境变量、验收文件算入生产来源或环境类 75。

| 项目 | 日志文件 | detail 文件（相对本仓） | raw 的实核 SHA256 |
| --- | --- | --- | --- |
| recovery | `9e271e1feb5140d38ba5c06bd67da9b3.log` | `perf/acceptance/run-tcl4n2c8/recovery.detail.json` | `771674639cd48c90f6b0b33371a909fb8bc5066089c439f3bc7af3ff1ba2ce17` |
| privacy | `ba8e448e38174ba0b24857e86657983a.log` | `perf/acceptance/run-9yp9sl3b/privacy.detail.json` | `e5884103fb84852d9ece95d695c30bc8897f1e739d17476e56d926846db46df9` |
| product_basics | `46e60949154b4ec9ae272a1c8def592b.log` | `perf/acceptance/run-b_tlxra8/product_basics.detail.json` | `c6c8c46aacccfc6e4922518786f43d2e529f78835c1dcdeafeccf5bf72740e7b` |
| native_ui | `59ef8ea86f7145b8ab5eba33b42a0efa.log` | `perf/acceptance/run-k81204k_/native_ui.detail.json` | `ca91a750816c6f13c38e8cfac7fad0b21ad56fd3230d8d8b83b3caf32fd28600` |
| functionality | `1ebae37e519f457888f53a7fda00e85b.log` | `perf/acceptance/run-a4czy8bt/functionality.detail.json` | `58b53fb8fdb1c2c8ca81c261dee04bd62b8c12dd7f65e4b4186ea173121e8dab` |

所有 detail 指向的 raw 均实读并按 SHA256 核对。product_basics 的 4/8 来自四项用户数据未变检查通过，四项 execution_completed 失败；不表示其基本功能已实际执行通过。

## 当前只读核验

- 已装 `/Applications/水利工具箱.app` 为 1.0.1 (31)，可执行 SHA256 `4e05a5408cd4eb66cce14d60080a6a0cf4ddfcd7ea850df0441605a54f56fd6e`。
- 当前真实回执 `perf/build-receipt.json` 的 SHA256 为 `5e46eb4c15519b1badb0a4b2f40e1d45504d16d3f5ee7712fb3a539a873c6e74`，绑定 1.0.1 (33)，可执行 SHA256 `3030558412e6873d7c58da689b85783e661f99371d010cc462cf161a6fe93035`。
- `build/release/水利工具箱.app` 正是上述 (33) 候选；Chapter 原 `verify_build_receipt(..., code_only=True)` 对候选返回 true，对已装返回 false，原因是“装机可执行文件/版本/图标与构建 receipt 不符”。
- 对已装和候选分别运行 `codesign --verify --deep --strict`，均退出 0；有效签名不消除版本和可执行文件差异。
- Chapter 原 `acceptance_code_environment` 返回 `SOP_BUILD_CURRENT=0`、`SOP_CODE_BINDING_SHA256=74468440841efde83dbad3195d3b0b8caed55b92c85d4dc7a755ffb4a130190e`。当前验收脚本已消费这两个变量，无队列环境时复用原只读 verifier；不应改为忽略实际产物，或把此失败归类为 75。
- `scripts/accept/product_checks.py`、`product_basics.py` 和已有回归测试没有未提交变更；上一轮 `handoffs/delivery-artifact-mismatch-20261017.md` 已记录相同已装差异。本轮不再制造候选/已装混淆。

输入未变的候选证据已复核 raw 和 SHA：functionality 36/36、native_ui 44/44 对应构建 (33)，见上一轮交接；recovery 8/8、privacy 37/37、product_basics 56/56 对应构建 (32)，见 `handoffs/delivery-checks-repair-20261017.md`。五份候选记录的生产摘要均为上面的当前摘要，但构建 (32) 三项只作为原范围证据保留，不能冒充当前 (33) 或已装 (31) 新测。

## 受阻与原入口接续

五项受阻于实际已装 (31) 与当前来源回执 (33) 不一致；本轮禁止装机，所以不能在本轮消除这个前提，也不修改验收目标或回执来消除错误。没有需要本人决定的产品取舍；装机确认沿 Chapter 原有按钮，不在本轮新增发版/装机决策。

本轮只读 `chapter jobs --app hydro-mac-oss --active --json` 返回 1 个 deferred 性能作业 `5eeaf3da50104f17beca72bc2b96962a`，没有装机作业。该性能作业与本轮五项失败无关，保留其状态、next_attempt_at 和原调度；不以该锁忙解释五项失败，不创建替代作业。

用户或后续执行者 cd 进本仓后，先用原 CLI 实核差异：

```sh
chapter sop verify-build-receipt --app hydro-mac-oss --artifact '/Applications/水利工具箱.app' --json
chapter jobs --app hydro-mac-oss --active --json
```

待 Chapter 在独立装机授权中对齐实际产物和回执后，由原 jobs 自动复验本轮五项；本轮不手工新增或重派作业。需要单项诊断时使用 `chapter sop accept --app hydro-mac-oss --check recovery --json`，同一入口的 check 可依次换成 privacy、product_basics、native_ui、functionality；对齐前不重复执行相同失败命令。不要使用 `scripts/delivery/resume.py` 接手本轮，它还会装机并执行范围外验收及素材动作。

本轮仅交接文本有变，本地提交限定本文件，未推送；未构建、装机、发版、部署、改变授权、运行本人已装 App 或释放 Chapter claims。仓内原因核验和交接工作已完成，可以关会话；五项正式验收仍受上述真实装机差异阻塞。

## 本次固定失败复核：410c59ce（2026-10-17）

本次 owner 为派出者 PID `9024` / `chapter-task:410c59ce0ae84952bb19e3708e6ca676`；写前以该精确身份检查本文件 claims，不另声明或释放。任务仍仅限五项 delivery 失败，保留其他会话改动、暂存和后台。具体写入范围为本节；共享资源仅只读文件、验签和 Chapter 查询，无构建、模拟器、GUI 或性能采样。完成条件为核对本次五份原件、当前产物与回执，记录授权范围内的结果和接续方式，并本地提交；五项通过不在此次核验结果中。

本次提供的日志与证据如下，均在产品用例开始前报同一错误 `Product/receipt mismatch (build): expected 33, found 31`：

| 检查 | 原日志文件（app-sop/workbench/logs） | 本仓固定证据目录 |
| --- | --- | --- |
| recovery | `23148a91ea814eb1a54a4564b64f673b.log` | `perf/acceptance/run-cpjng_dd/` |
| privacy | `adf770b49c5f40bdb686467f9f29f62a.log` | `perf/acceptance/run-5ae1joyk/` |
| product_basics | `9dfd190e2596484a8def34f37cd9b948.log` | `perf/acceptance/run-kdqjb55m/` |
| native_ui | `e0556929ff8344b09905c0322d8ab5ab.log` | `perf/acceptance/run-8tkc3jsh/` |
| functionality | `e02263bd16a041b39435e7e31fd1b735.log` | `perf/acceptance/run-japvm1ft/` |

已读取五份 detail 指向的 raw 并逐份比对 SHA256，全部一致；其中 product_basics raw SHA256 为 `1f66d1097f4239905ad30f1324c2cac9ad0063d2ff49646598f50a35eedc0b3d`，其余四份与上表旧轮 raw SHA 相同。product_basics 的 4/8 仍只是四项数据/安装字节未变，不是四项功能执行成功。

本次实读已装 Info.plist、可执行文件和当前回执：已装仍为 `1.0.1 (31)` / `4e05a5408cd4eb66cce14d60080a6a0cf4ddfcd7ea850df0441605a54f56fd6e`，候选仍为 `1.0.1 (33)` / `3030558412e6873d7c58da689b85783e661f99371d010cc462cf161a6fe93035`，回执 SHA256 仍为 `5e46eb4c15519b1badb0a4b2f40e1d45504d16d3f5ee7712fb3a539a873c6e74`。原只读 CLI 对已装返回 `ok: false`、退出 1，原因为“装机可执行文件/版本/图标与构建 receipt 不符”；对 `build/release/水利工具箱.app` 返回 `ok: true`、退出 0，原因为“装机可执行文件、图标与当前构建输入均匹配 receipt”。两包 `codesign --verify --deep --strict` 均退出 0。

共用 `bundle_binding` 的真实产物比较正确；`code_binding` 已消费 `SOP_CODE_BINDING_SHA256` / `SOP_BUILD_CURRENT`，直接 CLI 则复用 Chapter 原生产来源 verifier。此次错误发生在该来源检查前的实际 build 比较，不能归因于验收脚本被计入生产源码。五项入口也不依赖 Finder/Safari 常驻、私有 claims 变量或额外构建锁。无需为此修改实现或验收脚本；不能更换默认验收目标、伪造回执、忽略 build/SHA、改判 75 或手写通过来代替装机。

本次 `chapter jobs --app hydro-mac-oss --active --json` 仍只返回原 deferred 性能作业 `5eeaf3da50104f17beca72bc2b96962a`，无装机作业；未改其状态或调度。它不解释此次五项失败，且不属于此次修复范围。仅补本节，不重跑输入未变的五项失败命令，不追加测试、构建或候选验收。

五项仍受真实已装产物与回执不一致阻塞，本轮明确禁止装机，故无法在授权范围内消除前提。CLI 接手先执行 `chapter sop verify-build-receipt --app hydro-mac-oss --artifact '/Applications/水利工具箱.app' --json` 核实际差异；装机沿 Chapter 的独立既有装机入口处理，不新增用户决策或替代作业。实际对齐后由原 jobs 固定脚本复验，不手写证据。不要使用会执行装机及范围外动作的 `scripts/delivery/resume.py` 接续本轮。
