const zhCN = {
  'tab.settings': '设置',
  'tab.about': '关于',

  'settings.timer.title': '计时器',
  'settings.timer.workMinutes': '工作时间',
  'settings.timer.workMinutes.desc': '工作时长，到时提醒休息',
  'settings.timer.workMinutes.unit': 'min',
  'settings.timer.restSeconds': '休息时间',
  'settings.timer.restSeconds.desc': '每次休息时长',
  'settings.timer.restSeconds.unit': 'sec',
  'settings.timer.preAlertSeconds': '预提醒',
  'settings.timer.preAlertSeconds.desc': '休息前预提醒时间',
  'settings.timer.preAlertSeconds.unit': 'sec',
  'settings.timer.alertTimeout': '提醒超时',
  'settings.timer.alertTimeout.desc': '提醒等待上限，超时自动休息',
  'settings.timer.alertTimeout.unit': 'sec',

  'settings.behavior.title': '行为',
  'settings.behavior.sound': '提示音',
  'settings.behavior.sound.desc': '休息提醒时播放音效',
  'settings.behavior.fullscreenSkip': '全屏跳过',
  'settings.behavior.fullscreenSkip.desc': '全屏应用时跳过提醒',
  'settings.behavior.autoStart': '开机启动',
  'settings.behavior.autoStart.desc': '系统启动时自动运行',

  'settings.display.title': '显示',
  'settings.display.language': '语言',
  'settings.display.language.desc': '界面显示语言',
  'settings.display.theme': '主题',
  'settings.display.theme.desc': '外观主题',
  'settings.display.theme.light': '浅色',
  'settings.display.theme.dark': '深色',

  'about.version': '版本',
  'about.description': '基于 20-20-20 规则的跨平台桌面护眼工具',
  'about.rule': 'Every 20 min, look 20 feet away, for 20 seconds',
  'about.checkUpdate': '检查更新',
  'about.platform': '平台',
  'about.license': '开源协议',
  'about.source': '源代码',

  'tray.working': '工作中',
  'tray.resting': '休息中',
  'tray.paused': '已暂停',
  'tray.alerting': '该休息了',
  'tray.preAlert': '即将休息',
  'tray.pause': '暂停',
  'tray.resume': '继续',
  'tray.skipRest': '跳过',
  'tray.startRest': '开始休息',
  'tray.settings': '设置',

  'tip.alerting.title': '该让眼睛休息一下了',
  'tip.alerting.subtitle': '看向远处，让眼睛放松',
  'tip.resting.title': '休息中',
  'tip.resting.subtitle': '看向远处，让眼睛放松',
  'tip.startRest': '开始休息',
  'tip.skip': '跳过',
  'tip.waiting': '等待中...',

  'tipMinimal.resting': '休息中... 请看向远处',
  'tipMinimal.alerting': '休息一下... 请看向远处',
} as const;

export type TranslationKey = keyof typeof zhCN;
export type TranslationDict = { [K in TranslationKey]: string };

export default zhCN satisfies TranslationDict;
