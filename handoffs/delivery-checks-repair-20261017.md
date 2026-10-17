# 本轮四项 delivery 验收修复（2026-10-17）

本轮 owner 沿 Chapter 的 PID 48276 / `chapter-task:ac9e0a4d7c3e4bdd8ad580eb55a81afa`，写前用该身份核 claims，不另声明或释放。只修 delivery-functionality、delivery-recovery、delivery-privacy、delivery-product_basics；源码范围为 `scripts/accept/product_checks.py` 的证据目录与相对路径、`scripts/accept/product_basics.py` 的同类路径及生产绑定、新增 `Tests/test_delivery_acceptance.py` 和本交接。保留其他会话的 README、历史 handoff、perf、dist、素材、暂存和后台。

原四份失败日志分别为 `da72038258d7491c8e47f4ce879673fb`、`0082daa70ad44be983c87366e5b0bdab`、`a7d65d5bb1264ecd95063432144a8bb6`、`bae40f25e5a94bc8a983e88c2737bebb`（均在 `~/Library/Application Support/app-sop/workbench/logs/`）。逐项验收日志证明：代码从 `~/Library/Caches/chapter/worktrees/…` 执行，而 Chapter 合同将 SOP_OUT_DIR 指向本产品主仓；四项均在原目录断言处失败，未运行产品用例。product_basics 还沿用自行逐文件比较 receipt 的旧实现。

完成条件：沿已登记产品主仓保存证据，固定脚本在工作树与直接 CLI 两种布局都能读回，越界/其他产品目录仍拒绝；product_basics 复用 Chapter 的生产绑定与实际 bundle/版本/二进制/图标/签名校验；回归和四项固定候选验收通过后本地提交。实物与输出优先复用 `build/accept/native-ui-repair-20261017/` 的当前候选与原真实构建回执，本轮证据写 `build/accept/delivery-checks-repair-20261017/`。不改生产源码，不运行构建、装机、发布、推送或部署；若候选不再有效，构建须走 BuildSlot，忙/环境推迟返回 75 给原队列，不自建重试循环；无模拟器与性能采样。

本轮总预算 30 分钟，阶段限定为原件定位、最小补丁、必要回归/候选四项、交接与本地提交；不展开全产品旧欠项。

已核验：

- `PYTHONDONTWRITEBYTECODE=1 /Users/tianli/Dev/.venv/bin/python -m pytest -n auto -p no:cacheprovider --basetemp=build/accept/delivery-checks-repair-20261017/pytest Tests/test_native_ui_acceptance.py Tests/test_delivery_acceptance.py -q`：34 passed，7.90 秒；第一次 pytest 因输出父目录未建立而未开始测试，建立本轮目录后执行上述有效回归。覆盖主仓与工作树布局、精确产品/工作树身份、其他产品/父目录/软链越界拒绝、四入口失败证据的相对路径和真实 SHA、过期生产绑定/无效签名拒绝。
- 四个固定入口从本轮 `workspace/scripts/accept/` 运行，`SOP_REPO` 指向该隔离副本，`SOP_APP_ID=hydro-mac-oss`，`SOP_OUT_DIR` 指向主仓本轮证据目录；`--app` 与 `--receipt` 复用上一轮真实构建候选及回执，不编造生产绑定变量；脚本调用 Chapter 原只读 verifier 后通过才执行隔离副本。没有新构建、装机或产品源码变更。
- recovery 固定脚本退出 0，8/8；product_basics 退出 0，56/56（settings、shortcuts、icloud、updates 均 passed），证据在 `build/accept/delivery-checks-repair-20261017/evidence/` 对应 detail/raw。原生 JSON 和四张离屏截图的 SHA、全部证据相对主仓的路径均已读回核验；owner-before/after 完全相等，未改变本人偏好、配置、云文件和已装字节。
- 首轮功能/隐私各有两个 golden 失败，raw 明确为 Excel 样例路径不存在；这是本轮隔离目录准备时只复制 golden、漏复制 `cli/data/` 的夹具缺失。补齐原样例后仅重跑功能/隐私，首轮失败证据保留，后续输出另写 `complete-fixture-evidence/`；没有改 golden 预期、忽略断言或重试业务错误。

最终四项固定候选验收均退出 0：functionality 36/36、recovery 8/8、privacy 37/37、product_basics 56/56；执行副本与本轮当前脚本逐字节一致，四份 raw 的全部检查、绑定和 SHA 已读回核实。原始证据如下（路径均相对本仓）：

| 项 | raw 路径 | SHA256 |
| --- | --- | --- |
| functionality | `build/accept/delivery-checks-repair-20261017/complete-fixture-evidence/functionality-bf7df0e3b83847aaac272928adfd742d/raw.json` | `001053adee90d1d637f015bd38136f86f8a6123cac5401817e7e41a5230e3e01` |
| recovery | `build/accept/delivery-checks-repair-20261017/evidence/recovery-ac3047ba85fc4287a2608b42d73d703d/raw.json` | `9c153ed2412249eeb42d006593d76380e4f93adbaa8755e0c9c65e3563778172` |
| privacy | `build/accept/delivery-checks-repair-20261017/complete-fixture-evidence/privacy-202a8370e3cb4896a2670df473cdc3ed/raw.json` | `bda5aefc25dc93acac53da21d38fd79a8ddfd628ec3dca38752e2d8a34284d4f` |
| product_basics | `build/accept/delivery-checks-repair-20261017/evidence/product-basics-a71102994bd24204a1a9416f6bd3bbcb/raw.json` | `ec2169d4b2f5a27958d8cf6b636c055597c5e94ec21de27bdd1e289e29e0e70b` |

三份 Python 文件语法检查和 `git diff --check` 通过。本地提交仅含两个验收脚本、新增测试和本交接，不带他人的 README、历史交接或 perf 等改动与产物；保留原 claims、暂存和后台。

固定已装复验由原 jobs 队列自动接续，不手写通过证据。只读回读时已装版仍为 1.0.1 (31)，二进制 SHA256 `4e05a5408cd4eb66cce14d60080a6a0cf4ddfcd7ea850df0441605a54f56fd6e`；当时默认 `perf/build-receipt.json` 已为构建 33，沿用其他后台原件，不覆盖或伪报二者一致。候选通过不表示已装包通过；本轮无装机授权，留 Chapter 原队列处理实际产物对齐。必要时在本仓先用 `chapter jobs --app hydro-mac-oss --active --json` 查看原任务，产物一致后由原任务固定复验，不新建并行任务。

本轮授权的仓内修复和候选验证已全部完成，可以关会话；已装结果以原 jobs 后续真实回执为准。
