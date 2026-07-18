# Eyezen 用户痛点与产品约束：一手证据研究

- 日期：2026-07-15
- 范围：成人桌面工作者的数字视觉疲劳、久坐与休息、通知打断、习惯形成、可访问性、隐私与本地优先
- 产物性质：系统审计的需求研究输入，不是医疗建议、法律意见或真实用户研究的替代品
- 外部证据边界：仅采用官方机构/标准/平台文档和原始同行评审研究；不把产品博客、营销材料或二手综述作为关键主张依据

## 结论先行

Eyezen 的核心工作不应定义为“按时强制用户执行 20-20-20”，而应定义为：

> 在不破坏关键工作、不扩大隐私采集、不制造健康焦虑的前提下，帮助桌面工作者形成可持续、可解释、可自主调整的屏幕休息节律。

这一定位带来五个直接结论：

1. **20-20-20 是可配置的行为提示，不是已被充分证明的治疗方案。** AOA 与 NIOSH 将其列为实用建议，但一项 30 人交叉实验未发现 20 秒定时休息能降低短时任务后的症状；另有小型前瞻干预报告主观症状改善而客观眼表指标未同步改善。现有证据不足以证明精确的“20 分钟/20 秒/20 英尺”具有独特疗效。
2. **20 秒远眺不等于打断久坐。** WHO 要求减少并以身体活动替代久坐；NIOSH 把眼部远眺与离屏、起身、伸展、步行的 5 分钟休息分别描述。Eyezen 可以在长休息中提示活动，但不能把完成 20 秒休息表述为满足身体活动建议。
3. **打断成本是产品的第一性约束。** 原始 HCI 研究表明，中断会增加压力、挫折感、时间压力或恢复成本；在任务断点交付通知比任意时刻打断更可取。Eyezen 的全屏/AFK/进程白名单与预提醒方向正确，但仍需补齐时段静默、系统 Focus/DND 语义和明确的延后路径。
4. **健康评分只能描述行为，不得暗示健康结果。** 当前 `Eye-Care Index` 的权重和阈值没有找到临床验证来源。它应被解释为本地、可审计的“休息节律/执行情况”指标；分数、streak 和跳过记录不得制造道德判断。
5. **本地优先必须包含透明、删除和最小采集。** “数据只在本机”并不自动等于隐私完成；用户仍需知道记录了什么、为何记录、导出包含什么，并能完成删除。前台进程名、窗口标题、症状自评和持续活动历史都应按敏感数据对待。

## 1. 方法与证据标签

本文使用四类标签，避免把外部事实、仓库事实和产品判断混在一起：

- **[仓库事实]**：直接来自当前仓库文件或源码。
- **[来源事实]**：由就近链接的一手来源直接支持。
- **[产品推断]**：把来源事实与 Eyezen 当前能力结合后得到的、仍需验证的设计判断。
- **[未知]**：当前仓库和公开一手来源不能回答，必须通过目标用户或运行验证补齐。

证据优先级为：原始实验/正式标准与指南 > 官方平台设计文档 > 专业协会面向公众的建议。专业协会“推荐某规则”只能证明该建议存在，不能证明精确参数或医疗疗效。

### 1.1 本次访问核验状态

- **已在本次会话直接打开正文核验**：AOA Computer Vision Syndrome、Johnson/Rosenfield 的 PubMed 原始摘要、WHO 指南主页、NIOSH Working from Home、OSHA Computer Workstations eTool，以及下列仓库证据文件。
- **已定位到原始出版物/官方规范，但正文抓取受检索服务高负载影响**：Talens-Estarelles、McLean、Mark、Iqbal/Bailey、Lally，以及 W3C、Microsoft、Apple、NIST、EUR-Lex 的部分页面。它们均链接到作者机构、DOI 落地页或规范所有者，不使用二手文章替代；涉及这些来源的精确产品文案在公开发布前仍应逐页复核。
- 检索故障不会被解释为反证；本文因此避免给出数字视觉疲劳患病率、长期疗效百分比或未能核验的竞品事实。

## 2. Eyezen 的仓库事实基线

### 2.1 定位、平台与当前能力

**[仓库事实]** `README.md` 将 Eyezen 定位为“Cross-platform desktop eye care app — quiet, smart, out of your way”，目标平台为 Windows、macOS、Linux。当前公开功能包括：

- 可配置 20-20-20 与 Pomodoro 两种计时节律；
- 全屏、AFK、进程白名单、工作日的智能抑制；
- 多显示器休息覆盖层、预提醒、开始/跳过、暂停/恢复；
- 中英文、明暗主题、全局热键、托盘面板；
- 本地 SQLite 日/周/月趋势、休息结果拆分、`Eye-Care Index` 与 rhythm/streak；
- 本地 TOML 配置与 SQLite 备份导出。

**[仓库事实]** `src-tauri/src/models/config.rs` 的默认值为工作 20 分钟、休息 20 秒、预提醒 15 秒、提醒超时 60 秒；Pomodoro 默认为 25/5/15 分钟。`src/pages/tip/TipApp.svelte` 提供“开始休息”和“跳过”，`src/pages/tray/TrayApp.svelte` 提供暂停/恢复。

**[仓库事实]** `src-tauri/src/services/stat/health.rs` 的 Beta `Eye-Care Index` 由执行率 70% 与最长连续工作段 30% 加权；streak 阈值取最近有数据日期的中位数或启发式回退。该公式是实现事实，不是医学效度证据。

### 2.2 真实产品张力

**[产品推断]** Eyezen 已经不是“缺一个提醒功能”的项目。真正风险是三组互相冲突的目标：

| 想获得的价值     | 同时产生的风险                        | 应守住的约束                                                 |
| ---------------- | ------------------------------------- | ------------------------------------------------------------ |
| 提醒足够可见     | 抢焦点、打断深度工作、用户彻底关闭    | 从低强度预提醒逐级升级；关键上下文可抑制；始终有完整退出操作 |
| 统计更有解释力   | 把行为代理分数误读为健康结果          | 公式透明、Beta、非诊断；不用医学化颜色/等级                  |
| 更智能地判断场景 | 收集前台应用、窗口标题或行为历史      | 只采完成当前决策所需的最小信号；默认不保留上下文历史         |
| 形成长期习惯     | streak 压力、一次失败归零、被分数控制 | 强调滚动节律和恢复，不发送 streak 丢失通知                   |
| 扩展到久坐健康   | 定位膨胀成综合健康平台                | 仅在长休息提供可选活动提示；不宣称运动达标                   |

## 3. 痛点与一手证据

### 3.1 数字视觉疲劳：症状真实，精确处方不确定

**[来源事实]** AOA 将数字视觉疲劳描述为长时间使用电脑、平板、电子阅读器和手机后出现的一组眼部与视觉问题，列出的常见症状包括眼疲劳、头痛、视物模糊、眼干和颈肩痛；照明、眩光、观看距离、坐姿和未矫正视力问题都会影响症状。[AOA — Computer vision syndrome](https://www.aoa.org/healthy-eyes/eye-and-vision-conditions/computer-vision-syndrome)

**证据限制：** AOA 页面属于专业协会的公众指导，能支持“问题与建议存在”，不能单独证明干预疗效或人群患病率。

**[来源事实]** AOA 与 NIOSH 都把 20-20-20 列为可尝试的实用建议；NIOSH 同时建议调整字体、亮度/对比度、眩光和显示器位置，并强调定期离开屏幕。[AOA](https://www.aoa.org/healthy-eyes/eye-and-vision-conditions/computer-vision-syndrome)；[NIOSH — Working from Home](https://www.cdc.gov/niosh/bulletin/2020/working-from-home.html)

**[来源事实]** Johnson 与 Rosenfield 的交叉实验让 30 名 21–31 岁参与者完成四次 40 分钟平板阅读任务，分别每 5、10、20 或 40 分钟允许 20 秒休息；四种条件下症状都显著上升，休息频率对症状、阅读速度和准确率均无显著影响。作者结论是不支持把 20 秒定时休息作为数字视觉疲劳的治疗干预。[PubMed PMID 36473088](https://pubmed.ncbi.nlm.nih.gov/36473088/)；[DOI 10.1097/OPX.0000000000001971](https://doi.org/10.1097/OPX.0000000000001971)

**证据限制：** 样本小、年龄窄、任务仅 40 分钟且是平板阅读；它否定不了长期、不同工作环境或更长休息的收益，只能否定“精确规则已经被充分证明”的强断言。

**[来源事实]** Talens-Estarelles 等人的前瞻提醒干预报告了部分主观数字眼疲劳/干眼症状改善，但未观察到相应的客观眼表或双眼视觉体征改善，说明提醒可能有主观收益，但仍不能证明精确参数具有特异疗效。[DOI 10.1016/j.clae.2022.101744](https://doi.org/10.1016/j.clae.2022.101744)

**[产品推断]** 对 Eyezen 的约束：

- 默认 20/20/20 可以保留为易记模板，但界面必须明确“可配置的休息节律”，不用“保护视力”“预防眼病”“降低风险 X%”等疗效语言。
- 提醒内容不只应重复“20 英尺”，还可轮换低风险、来源可解释的环境提示：远眺、眨眼、检查眩光/字号/显示器距离；不得推荐药物、滴眼液或诊断。
- `Eye-Care Index` 不应使用临床风险、健康等级或治疗效果叙事。更准确的名字是“休息节律指数”或“今日执行情况”。
- 若症状持续或明显，应提示咨询合格眼科/视光专业人员，而不是让用户提高提醒强度自我治疗。

**可验证产品启示：** 对所有 UI、README、发布说明做一次健康声明清单测试：每一句必须能被分类为“规则说明”“行为记录”或“专业转介”，任何结果性医疗动词均应阻断发布。

### 3.2 久坐与身体活动：眼休息和身体休息是两条不同路径

**[来源事实]** WHO 2020 指南建议成人限制久坐时间，并尽可能用任何强度的身体活动替代久坐；指南没有给出适用于所有人的单一久坐上限或统一打断频率。[WHO 指南主页](https://www.who.int/publications/i/item/9789240015128)；[指南方法与建议论文](https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/)

**[来源事实]** OSHA 说明，即使工位设计良好，长时间保持静态姿势或执行重复任务仍会造成局部疲劳；持续注视显示器会使支撑头部的颈肩肌肉疲劳。[OSHA Computer Workstations eTool — Work Process](https://www.osha.gov/etools/computer-workstations/work-process)

**[来源事实]** NIOSH 把“20/20/20 远眺”与“每小时 5 分钟离屏并离开坐姿，伸展、步行或补水”分开描述，并称其研究发现，在传统休息之外增加每小时 5 分钟休息可降低肌肉骨骼不适和眼疲劳。[NIOSH — Working from Home](https://www.cdc.gov/niosh/bulletin/2020/working-from-home.html)

**[来源事实]** McLean 等人对 15 名电脑终端工作者的现场研究比较了约 30 秒微休息的不同频率，报告更频繁微休息可降低不适且未造成生产率损失；样本很小且人群单一，不能据此制定通用处方。[DOI 10.1016/S0003-6870(00)00071-5](<https://doi.org/10.1016/S0003-6870(00)00071-5>)

**[产品推断]** 对 Eyezen 的约束：

- 20 秒远眺仍处于坐姿且能量消耗极低，不能记为“活动达标”或“久坐已打断”。
- Pomodoro 短/长休息可以提供可选的起身/伸展/补水提示，但这是内容层扩展，不应演变为步数、卡路里、姿态识别或综合健康平台。
- 统计中必须分开“眼部距离休息”和“活动休息”；在没有传感器或明确用户确认时，不推断用户真的站立或运动。

**可验证产品启示：** 检查产品内每一个 `rest` 指标的中文/英文名称，保证它只描述“计时器休息已完成”，不偷换为“身体活动完成”或“健康收益”。

### 3.3 通知打断：被看到不等于被接受

**[来源事实]** Mark、Gudith 与 Klocke 的 48 人实验发现，被中断的参与者会通过加快工作补偿，完成时间未必更差，但报告了更高压力、挫折感、时间压力和努力程度。实验环境与真实长期办公不同，但足以说明“没有生产率下降”不能推出“中断无成本”。[原始论文 PDF（UC Irvine）](https://www.ics.uci.edu/~gmark/chi08-mark.pdf)；[DOI 10.1145/1357054.1357072](https://doi.org/10.1145/1357054.1357072)

**[来源事实]** Iqbal 与 Bailey 的 CHI 研究以任务断点管理通知，结果支持在较合适的任务转换点交付通知，以降低即时、任意时刻打断带来的成本；该研究不是休息提醒器的长期随机试验，产品迁移仍需验证。[Microsoft Research 论文页](https://www.microsoft.com/en-us/research/publication/effects-of-intelligent-notification-management-on-users-and-their-tasks/)；[DOI 10.1145/1357054.1357193](https://doi.org/10.1145/1357054.1357193)

**[来源事实]** Apple 的通知设计指导要求通知提供及时且高价值的信息，并给用户管理通知的控制；Microsoft 的通知 UX 指导同样把通知视为用户可关闭、可管理的系统 Surface，而不是保证送达的强制通道。[Apple HIG — Notifications](https://developer.apple.com/design/human-interface-guidelines/notifications)；[Microsoft — Toast UX guidance](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-ux-guidance)

**[产品推断]** Eyezen 的提醒 Interface 应具备以下完整语义，而不是继续增加特殊分支：

1. **预告**：低强度、不中断输入，说明距离休息还有多久。
2. **现在休息**：用户显式接受后进入休息。
3. **延后到一个明确时间**：语义不同于跳过；仅在用户研究证明有需求后加入，且必须作为独立状态转换，而不是布尔 flag。
4. **跳过本轮**：不追罚、不连续弹回；统计说明这是用户选择，不是失败。
5. **暂停/静默时段**：用户能预测何时恢复。
6. **上下文抑制**：全屏、AFK、白名单或系统 Focus/DND 触发时，说明“为什么没有提醒”，但不泄露不必要的应用历史。

当前已有预提醒、开始、跳过、暂停和三类上下文抑制；缺口主要是工作时段/静默时段、系统 Focus/DND 的明确政策，以及“跳过”和“延后”是否应该区分的用户证据。

**可验证产品启示：** 不用“通知点击率”证明提醒成功。至少同时观察本地的接受、跳过、抑制、从提醒到选择的时间，以及用户是否在试用后关闭功能；这些指标只能在明确同意的可用性研究中收集，不能默认上传。

### 3.4 习惯形成：稳定情境与可恢复性比 streak 压力更可信

**[来源事实]** Lally 等人让 96 名参与者选择一个日常健康行为并连续 12 周报告，在稳定情境中重复行为时，自动化程度呈渐近增长；达到 95% 自动化平台的中位估计约为 66 天，个体差异很大，一次错过执行机会没有显著破坏形成过程。[DOI 10.1002/ejsp.674](https://doi.org/10.1002/ejsp.674)

**证据限制：** 这是自选饮食/饮水/活动行为的纵向研究，不是屏幕休息提醒试验；“66 天”是样本中位估计，不是产品承诺或通用期限。

**[产品推断]** 对 Eyezen 的约束：

- 让用户选择与真实工作情境匹配的节律、工作日和静默时段，比强迫统一模板更符合稳定情境重复的机制。
- 不宣称“21 天养成习惯”，也不把 66 天变成新的倒计时。
- 一次跳过不应让 streak 归零后出现损失提醒；优先显示 7/30 天滚动节律、个人趋势和“下一次可继续”。
- 当前基于“有数据日期中位数”的 streak 是行为展示，不是健康信号；界面应保持弱化、可隐藏，不发送保 streak 的提醒。

**可验证产品启示：** 若保留 streak，用户测试必须比较“连续天数”与“滚动一致性”两种呈现，观察羞耻/压力、理解偏差和关闭提醒意愿，而不只测哪一种更能增加完成次数。

### 3.5 可访问性：提醒器不能用视觉疲劳换取“护眼”

**[来源事实]** WCAG 2.2 要求键盘可操作（2.1.1）、普通文本至少 4.5:1 对比度（1.4.3）、关键 UI/图形至少 3:1 非文本对比度（1.4.11）、可见焦点（2.4.7）、不小于 24×24 CSS px 的最低目标尺寸（2.5.8，AA）、对时间限制提供关闭/调整/延长能力（2.2.1）、对自动开始且持续超过 5 秒的移动/闪烁/滚动内容提供暂停/停止/隐藏（2.2.2），并为状态消息提供程序化可感知方式（4.1.3）。[W3C WCAG 2.2](https://www.w3.org/TR/WCAG22/)

**适用边界：** Eyezen 的 Svelte 内容运行在 WebView，WCAG 可作为可自动化的最低基线；窗口抢焦点、系统通知、屏幕阅读器多窗口行为仍需按各桌面平台验证，不能只跑 Web 可访问性扫描。

**[来源事实]** Microsoft 与 Apple 都要求应用尊重系统辅助功能偏好，例如高对比度/颜色、减少运动、键盘与辅助技术语义。[Microsoft accessibility overview](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-overview)；[Apple HIG — Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)

**[仓库事实]** `TipMinimalApp.svelte` 有一个无限循环的 5 秒 opacity “breathing” 动画，未见 `prefers-reduced-motion` 分支；`TipApp.svelte` 多处使用 0.30–0.35 的白色透明度显示 15 px 文本或按钮。这里只能判定为高风险候选，实际是否合格仍须按最终背景色计算对比度并运行平台测试。

**[产品推断]** 对 Eyezen 的约束：

- 所有动画尊重 `prefers-reduced-motion`/平台 Reduce Motion；装饰动画关闭后不能损失状态信息。
- 状态不能只靠绿/黄/灰颜色表达；当前托盘同时有文字标签，应保留这种冗余编码。
- 多显示器可以都显示视觉覆盖层，但只应有一个可交互/可被屏幕阅读器宣告的主窗口，避免重复朗读和焦点跳转。
- 自动弹出的提醒不应吞掉用户正在输入的焦点；开始、跳过、暂停必须全键盘完成且焦点顺序稳定。
- 60 秒提醒超时虽可配置，仍需验证屏幕阅读器用户是否有足够时间理解并操作；超时不能把人锁入循环提醒。
- 高对比度、200% 文本缩放、Windows Narrator、macOS VoiceOver 和 Linux 至少一个主流桌面辅助技术应进入发布验收矩阵。

### 3.6 隐私与本地优先：最小化、透明、可删除

**[来源事实]** W3C Privacy Principles 要求把数据最小化、透明、目的限制和用户控制放入系统设计；GDPR 第 5 条也明确了目的限制、数据最小化和存储限制原则。这里引用它们作为工程约束，不对 Eyezen 是否受特定法律管辖作结论。[W3C Privacy Principles](https://www.w3.org/TR/privacy-principles/)；[EUR-Lex GDPR 正文](https://eur-lex.europa.eu/eli/reg/2016/679/oj)

**[来源事实]** NIST Privacy Framework 将识别数据处理、治理、向用户沟通以及让个人管理数据处理列为持续的隐私风险管理活动；它不是一次性的“本地存储”复选框。[NIST Privacy Framework](https://www.nist.gov/privacy-framework)

**[仓库事实]** `README.md` 表明配置保存在本地 `config.toml`，统计保存在同目录 `data.db`，并支持 SQLite 备份导出。进程白名单功能要求处理前台进程 basename；当前公开说明未形成一个面向用户的、可核验的“收集/保留/导出/删除”清单。

**[产品推断]** 隐私 Interface 应收敛为一项完整操作：

- 展示本地数据位置、记录字段、用途、保留周期以及哪些数据进入导出；
- 支持导出后清空统计，并对删除范围作明确确认；
- 进程白名单只授权“当前是否抑制提醒”的判断，不自动授权保存长期应用使用历史；
- 如果未来保存 `process_hint`，必须独立、默认关闭、按事件采样、不记录窗口标题，并给出一键删除与导出说明；
- 更新检查等必要联网行为应与遥测明确区分并披露；不把“未发现遥测”扩大成“永不联网”承诺；
- 在没有明确威胁模型前，不用“加密数据库”掩盖权限、密钥和备份 Surface；先保证最小数据、操作系统用户目录权限、删除和透明。

**可验证产品启示：** 用干净安装运行网络观察和文件系统快照，产出“输入事件 → 本地字段 → 读取者 → 保留/导出/删除”的数据流表；任何没有产品用途或 owner 的持久字段均应删除，而不是等待隐私政策解释。

## 4. 痛点优先级与产品验收假设

| 优先级 | 痛点                                     | 当前证据                           | Eyezen 假设                                          | 最小验证                                                           |
| ------ | ---------------------------------------- | ---------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------ |
| P0     | 提醒破坏专注，最终被关闭                 | 原始中断研究 + 当前全屏提醒路径    | 可预测预告、上下文抑制和完整退出操作能提高长期接受度 | 目标用户两周对照日记；比较关闭率、跳过原因与主观打断度             |
| P0     | 用户把 `Eye-Care Index` 当医疗分数       | 当前名称/公式 + 无对应临床效度来源 | 改为“节律/执行情况”，展示公式和限制可降低误解        | 盲测：让用户解释分数含义；不得出现“眼睛更健康”的回答               |
| P1     | 低对比度、运动、抢焦点使部分用户无法操作 | WCAG/平台规范 + 源码高风险候选     | 系统偏好、键盘和单一可访问主窗口可闭合主路径         | 三平台辅助功能矩阵与真实屏幕阅读器操作                             |
| P1     | 本地统计不可理解、不可删除               | 隐私原则 + README 仅描述位置/导出  | 一个数据中心式完整操作可建立信任                     | 用户能在 60 秒内回答“存了什么/在哪/如何删”，删除后文件与查询均验证 |
| P1     | 工作日配置不能表达会议、夜间和个人时段   | 中断证据 + 当前仅 weekday schedule | 明确静默时段比继续扩充应用白名单更易预测             | 访谈确认最常见冲突是否是时段，而不是应用                           |
| P2     | 眼休息被误当身体活动                     | WHO/NIOSH                          | 长休息给可选活动提示、统计分开命名可消除误导         | 文案审计与理解测试                                                 |
| P2     | streak 让一次跳过变成失败                | 习惯形成研究 + 当前 rhythm/streak  | 滚动一致性比归零 streak 更可恢复                     | 比较压力、理解、继续使用意愿；不以完成次数单指标决策               |

这些都是**验收假设**，不是已经由真实 Eyezen 用户验证的需求。公开研究能说明风险机制，不能替代目标人群的工作流证据。

## 5. 建议的最小产品切片

### 5.1 立即收敛：低复杂度、高信任收益

1. **健康声明与命名收敛**
   - 把 `Eye-Care Index` 改为非医学化的“休息节律指数/执行情况”；保留 Beta、公式解释和“不是健康诊断”。
   - README 的 “protects your eyes” 改成“帮助你安排屏幕休息”一类行为表述。
   - 不新增算法、表或 seam；先修正产品契约。

2. **减少运动与对比度验收**
   - 为 `breathing` 和其他装饰动画接入系统减少运动偏好；对提醒页文本、ghost button、焦点态做可复现的对比度检查。
   - 这是同一提醒 Interface 的可访问实现，不建立平行的“无障碍模式”。

3. **本地数据说明**
   - 在统计/关于页展示数据位置、记录内容、导出范围和联网行为；来源应由代码生成或测试守卫，避免文档与实现形成两份真相。

4. **streak 去惩罚化**
   - 不发送 streak 丢失通知；文案强调个人节律而非达标，不把休息日计为失败。

### 5.2 短期深化：先验证，再扩大 Surface

1. **工作时段/静默时段**
   - 当前 weekday schedule 只表达“哪天”，不能表达“什么时候”。若访谈确认会议、下班后提醒是主要痛点，增加一个明确的工作时间段 Module，而不是给每个场景继续加 flag。
   - 验收：跨午夜、时区/DST、暂停恢复、应用重启和手动临时静默都只有一条可解释路径。

2. **“将当前应用加入白名单”快捷操作**
   - 复用已有前台进程判断，在托盘提供完整操作，避免用户手输 basename；操作前显示将保存的确切名称。
   - 不记录窗口标题，不创建连续应用历史。

3. **本地数据删除**
   - 与导出并列提供“清空统计”完整操作；数据库删除、内存缓存刷新、统计空状态和错误语义一次闭合。
   - 删除前确认，删除后可通过查询和重启验证；不做“软删除兼容层”。

4. **长休息的可选活动提示**
   - 20 秒模式只提示远眺/眨眼；Pomodoro 短/长休息可提示起身/伸展/补水。
   - 不记录用户是否真的完成活动，不增加身体健康评分。

### 5.3 研究后再做：收益可能高，跨平台成本也高

- **尊重系统 Focus/DND**：先逐平台确认可读能力、权限和失败语义；若三平台能力不对称，界面必须公开 capability，而不是静默兜底。
- **“延后”操作**：先确认用户是否真的区分“本轮跳过”和“几分钟后再提醒”；若加入，它应是状态机中的独立事件和结果，不是 `skip + delay` 的调用者编排。
- **个性化建议**：只根据本地、可解释的跳过/抑制结果提出设置建议，用户确认后才改配置；不用黑箱 AI 自动改变节律。

## 6. 明确不建议的功能扩张

| 不建议做                                   | 理由                                                               |
| ------------------------------------------ | ------------------------------------------------------------------ |
| 摄像头眼动、眨眼或姿态识别                 | 权限、误判和隐私成本远高于计时提醒的核心价值；会把产品变成监控系统 |
| 持续记录窗口标题、键盘鼠标或完整应用时间线 | 超出抑制提醒所需的最小数据；标题可能包含客户、文档和个人信息       |
| 云账户、排行榜、社交 streak                | 增加身份、同步、滥用与数据治理 Surface，且没有当前核心任务证据     |
| “AI 预测眼疲劳”或自动医疗建议              | 当前信号只能描述休息行为，没有症状、诊断或临床效度                 |
| 蓝光过滤疗效、风险百分比、视力保护承诺     | 证据和产品能力都不支持结果性医疗声明                               |
| 把姿势、睡眠、心理健康、运动都纳入一个总分 | 概念与数据 Surface 无限制增长，稀释桌面休息主路径                  |
| 为每个平台另建一套计时/提醒业务流          | 平台差异应停留在能力 Adapter；核心状态与结果保持单一事实源         |
| 更多默认强提醒、连续补弹或跳过惩罚         | 短期提高可见度，长期提高关闭率和不信任；与中断/习惯证据冲突        |

## 7. 研究驱动的验收清单

### 健康与需求边界

- [ ] 所有健康文案只描述规则、行为或转介，不承诺治疗、预防或风险降低。
- [ ] 20-20-20 与 Pomodoro 均可配置，并明确是模板而非唯一科学处方。
- [ ] 眼部距离休息与身体活动休息在 UI、事件和统计中不混用。
- [ ] 分数公式、输入、未知与 Beta 状态可见；用户不能把它合理解释成临床健康分。

### 打断与控制

- [ ] 预提醒不抢输入焦点；提醒只产生一次可预测升级。
- [ ] 开始、跳过、暂停/恢复、静默与上下文抑制各有单一、可解释语义。
- [ ] 全屏、AFK、白名单、schedule 与未来 DND 同时出现时有明确优先级和唯一原因记录。
- [ ] 用户跳过后不会在同一轮连续补弹，streak 不产生损失提醒。

### 可访问性

- [ ] WCAG 2.2 AA 相关规则自动检查；三平台高对比度、减少运动、200% 缩放人工检查。
- [ ] 仅键盘可完成主路径，焦点可见且不被覆盖层意外夺取。
- [ ] Narrator/VoiceOver 至少各验证一次完整提醒；多屏只宣告一个交互 Surface。
- [ ] 倒计时与状态变化可被辅助技术理解，但不会每秒制造冗余朗读。

### 隐私与本地优先

- [ ] 有从事件到字段、读取者、保留、导出、删除的数据流表。
- [ ] 默认运行的网络请求与本地文件写入都能通过干净安装复现并与说明一致。
- [ ] 前台进程只用于当前决策；任何持久上下文采集均独立 opt-in，且不含窗口标题。
- [ ] 导出内容可预览/说明；清空后重启不能恢复旧统计。

## 8. 仍不确定、必须向真实用户或运行环境求证的事项

1. **目标用户**：当前 README 没有证明主要用户是开发者、普通办公室人员、学生、远程工作者还是有既往眼部症状的人群。
2. **最主要打断冲突**：不知道用户最常在会议、演示、游戏、视频、编码、辅助技术使用还是下班后关闭提醒。
3. **提醒强度偏好**：不知道多屏全屏覆盖是核心价值还是卸载原因；需要定性访谈与两周使用记录。
4. **分数理解**：没有证据说明用户能把 `Eye-Care Index` 理解为行为代理，而不是健康结果。
5. **平台真实能力**：系统 Focus/DND、Wayland 前台进程、macOS 辅助功能权限的可用性和错误语义仍需逐平台实测。
6. **数据敏感度**：不知道用户是否接受保存进程 basename、跳过原因或长期 streak；不能用“本地”替代同意。
7. **真实长期收益**：公开研究不能证明 Eyezen 的留存、症状改善或工作表现；发布前不得声称这些结果。

## 9. 一手来源索引

### 官方健康与职业安全资料

- [WHO — Guidelines on physical activity and sedentary behaviour](https://www.who.int/publications/i/item/9789240015128)
- [WHO guideline development paper — PMC 7719906](https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/)
- [NIOSH — Working from Home: How to Optimize Your Work Environment and Stay Healthy](https://www.cdc.gov/niosh/bulletin/2020/working-from-home.html)
- [OSHA — Computer Workstations eTool: Work Process and Recognition](https://www.osha.gov/etools/computer-workstations/work-process)
- [AOA — Computer vision syndrome](https://www.aoa.org/healthy-eyes/eye-and-vision-conditions/computer-vision-syndrome)

### 原始同行评审研究

- Johnson S, Rosenfield M. _20-20-20 Rule: Are These Numbers Justified?_ [PubMed](https://pubmed.ncbi.nlm.nih.gov/36473088/) · [DOI](https://doi.org/10.1097/OPX.0000000000001971)
- Talens-Estarelles C, et al. _The effects of breaks on digital eye strain, dry eye and binocular vision: Testing the 20-20-20 rule._ [DOI](https://doi.org/10.1016/j.clae.2022.101744)
- McLean L, et al. _Computer terminal work and the benefit of microbreaks._ [DOI](<https://doi.org/10.1016/S0003-6870(00)00071-5>)
- Mark G, Gudith D, Klocke U. _The Cost of Interrupted Work: More Speed and Stress._ [Author-hosted PDF](https://www.ics.uci.edu/~gmark/chi08-mark.pdf) · [DOI](https://doi.org/10.1145/1357054.1357072)
- Iqbal ST, Bailey BP. _Effects of Intelligent Notification Management on Users and Their Tasks._ [Microsoft Research](https://www.microsoft.com/en-us/research/publication/effects-of-intelligent-notification-management-on-users-and-their-tasks/) · [DOI](https://doi.org/10.1145/1357054.1357193)
- Lally P, et al. _How are habits formed: Modelling habit formation in the real world._ [DOI](https://doi.org/10.1002/ejsp.674)

### 可访问性、平台与隐私

- [W3C — Web Content Accessibility Guidelines (WCAG) 2.2](https://www.w3.org/TR/WCAG22/)
- [Microsoft — Accessibility overview for Windows apps](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-overview)
- [Microsoft — Toast UX guidance](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-ux-guidance)
- [Apple Human Interface Guidelines — Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)
- [Apple Human Interface Guidelines — Notifications](https://developer.apple.com/design/human-interface-guidelines/notifications)
- [W3C — Privacy Principles](https://www.w3.org/TR/privacy-principles/)
- [NIST — Privacy Framework](https://www.nist.gov/privacy-framework)
- [EUR-Lex — Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj)

## 10. 仓库证据索引

- `README.md`：产品定位、平台、功能、架构概览、本地配置与数据库路径。
- `package.json`：v0.7.1、Svelte/Vite/Tauri 前端依赖与本地检查脚本。
- `src-tauri/Cargo.toml`：Tauri/Rust/sqlx/SQLite、平台依赖与 release profile。
- `CHANGELOG.md`：功能演进与已有 hardening 声明。
- `src-tauri/src/models/config.rs`：默认节律、Pomodoro、schedule、AFK 与进程白名单配置。
- `src/pages/tip/TipApp.svelte`：全屏提醒的开始/跳过交互与视觉样式。
- `src/pages/tip-minimal/TipMinimalApp.svelte`：最小提醒与无限 breathing 动画。
- `src/pages/tray/TrayApp.svelte`：托盘状态、暂停/恢复、开始/跳过路径。
- `src-tauri/src/services/stat/health.rs`：执行率、抑制原因、`Eye-Care Index` 与 rhythm/streak 公式。
