/**
 * 终端会话「迁移到独立窗口」的标记集合（模块级单例，跨组件/窗口共享）。
 *
 * 用途：detach 把 tab 移出 store 后，面板的 Terminal 组件会 unmount，
 * 但此时后端 PTY session 要交给独立窗口接管，不能被 ptyKill。
 * popout 时 markMigrating 登记、独立窗口 onMounted 接管后 unmarkMigrating 清除，
 * Terminal.vue 的 onBeforeUnmount 据此跳过 kill。
 */
const migrating = new Set<string>()

/** 标记某 sessionId 正在迁移到独立窗口（面板 unmount 时跳过 ptyKill）。 */
export function markMigrating(sessionId: string) {
  migrating.add(sessionId)
}

/** 独立窗口已接管，清除迁移标记。 */
export function unmarkMigrating(sessionId: string) {
  migrating.delete(sessionId)
}

/** 查询某 sessionId 是否正在迁移中。 */
export function isMigrating(sessionId: string) {
  return migrating.has(sessionId)
}
