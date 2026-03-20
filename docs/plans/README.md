# Implementation Plans

实现计划存放目录。每个计划对应一个功能切片或阶段性目标。

## 命名规范

```
<序号>-<scope>.md
```

- **序号**：三位数字，按创建顺序递增（`001`, `002`, ...）
- **scope**：kebab-case，简短描述计划范围

示例：
```
001-config-service.md
002-timer-core.md
003-timer-loop.md
004-sound-service.md
005-platform-detect.md
006-window-service.md
007-tray-service.md
008-integration.md
009-frontend-prototype.md
```

## 计划模板

每个计划文件应包含：

```markdown
# <标题>

## 目标
一句话说明这个计划要达成什么。

## 前置依赖
- 依赖哪些已完成的计划或模块

## 切片
按实现顺序列出任务：
1. ...
2. ...

## 接口定义
涉及的类型、command、event 签名。

## 测试要求
需要覆盖的测试场景。

## 验收标准
怎样算完成。
```

## 索引

| 序号 | 计划 | 状态 | 说明 |
|------|------|------|------|
| 001 | [config-service](001-config-service.md) | Implemented | 模块骨架 + AppError + Service trait + tracing + ConfigService |
| 002 | [timer-core](002-timer-core.md) | Implemented | Timer 纯函数状态机 (types + 3 pure functions) |
| 003 | [timer-loop](003-timer-loop.md) | Implemented | TimerService struct + tokio tick loop + stub effects |
| 004 | [sound-service](004-sound-service.md) | Implemented | rodio 独立线程 + mpsc + 嵌入式音频 |
| 005 | [platform-detect](005-platform-detect.md) | Implemented | PlatformApi trait + 3 平台全屏检测 + DetectorService |
| 006 | [window-service](006-window-service.md) | Implemented | 多显示器 tip-window 生命周期管理 |
| 007 | [tray-service](007-tray-service.md) | Implemented | 托盘菜单 + tooltip + 面板切换 |
| 008 | [integration](008-integration.md) | Implemented | AppServices 接线 + commands + events + 优雅关闭 |
| 009 | [frontend-prototype](009-frontend-prototype.md) | Implemented | 前端最小闭环：ts-rs 类型桥接 + IPC stores + tray-panel + tip-window |
| 010 | [main-window-settings-about](010-main-window-settings-about.md) | Implemented | Settings UI + About 页面，完成 MVP 前端 |
