<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import * as echarts from "echarts/core";
import { LineChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import * as api from "@/api";
import { useHostsStore } from "@/store/hosts";
import { useStatsStore } from "@/store/stats";
import type { Container, Image, StatsSample } from "@/api/types";

echarts.use([
  LineChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  CanvasRenderer,
]);

const route = useRoute();
const store = useHostsStore();
const hostId = computed(() => route.params.id as string);

const containers = ref<Container[]>([]);
const images = ref<Image[]>([]);
const loading = ref(false);

const probe = computed(() => store.probeMap[hostId.value]);

const runningCount = computed(
  () => containers.value.filter((c) => c.state === "running").length,
);
const projectCount = computed(
  () =>
    new Set(
      containers.value
        .map((c) => c.compose_project)
        .filter((p): p is string => !!p),
    ).size,
);

// ===== 资源监控聚合 =====
// docker stats 按容器逐条推送，这里维护「最近一次各容器采样」，
// 再用独立的 tick（与 stats interval 对齐）把所有容器汇总成一个整机点。
// 历史数据放 store，切走再回来能恢复曲线、网络速率也能正确算出第一点。
const latestByContainer = new Map<string, StatsSample>();
const statsStore = useStatsStore();
// 指向 store 内该主机的 stats 对象（响应式），直接读写即可
const stats = computed(() => statsStore.get(hostId.value));

// 当前展示值（卡片大数字）
const cpuNow = ref(0);
const memNow = ref(0);
const netRxRate = ref(0); // bytes/s
const netTxRate = ref(0); // bytes/s
const netRxTotal = ref(0); // bytes
const netTxTotal = ref(0); // bytes

// 百分比（多容器汇总可能超 100，环形限制在 100 内）
const cpuPct = computed(() => Math.min(Math.max(cpuNow.value, 0), 100));
const memPct = computed(() => Math.min(Math.max(memNow.value, 0), 100));

// 按阈值着色：<60 正常 / <85 警告 / >=85 危险
function thresholdColor(pct: number): string {
  if (pct >= 85) return "#f56c6c";
  if (pct >= 60) return "#e6a23c";
  return "#67c23a";
}
const cpuColor = computed(() => thresholdColor(cpuPct.value));
const memColor = computed(() => thresholdColor(memPct.value));

// ===== 图表 =====
const netEl = ref<HTMLDivElement>();
let netChart: echarts.ECharts | null = null;

// 首次加载才显示全屏 loading；轮询刷新静默更新，避免遮挡已渲染内容
let loadedOnce = false;
async function loadAll() {
  // 守卫：hostId 为空（keep-alive 切换瞬间）时跳过，避免 invalid args
  if (!hostId.value) return;
  if (!loadedOnce) loading.value = true;
  try {
    const [c, i] = await Promise.all([
      api.listContainers(hostId.value),
      api.listImages(hostId.value),
    ]);
    containers.value = c;
    images.value = i;
    loadedOnce = true;
  } finally {
    loading.value = false;
  }
}

// ---- 大小解析/格式化 ----

/** "1.23kB" / "4.56MB" / "789B" -> 字节数 */
function parseSize(s: string): number {
  const m = s.trim().match(/^([\d.]+)\s*([kKMGTP]?B?)$/i);
  if (!m) return 0;
  const val = parseFloat(m[1]);
  if (isNaN(val)) return 0;
  const unit = m[2].toUpperCase();
  const factor: Record<string, number> = {
    "": 1,
    B: 1,
    KB: 1000,
    MB: 1_000_000,
    GB: 1_000_000_000,
    TB: 1_000_000_000_000,
    PB: 1_000_000_000_000_000,
  };
  return val * (factor[unit] ?? 1);
}

/** 字节数 -> 人类可读 */
function formatBytes(b: number): string {
  if (!isFinite(b) || b <= 0) return "0 B";
  if (b < 1000) return `${b.toFixed(0)} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let i = -1;
  let v = b;
  do {
    v /= 1000;
    i++;
  } while (v >= 1000 && i < units.length - 1);
  return `${v.toFixed(1)} ${units[i]}`;
}

/** "1.23kB / 4.56MB" -> [rxBytes, txBytes] */
function parseNetIO(net: string): [number, number] {
  const parts = net.split("/");
  const rx = parts[0] ? parseSize(parts[0]) : 0;
  const tx = parts[1] ? parseSize(parts[1]) : 0;
  return [rx, tx];
}

function netChartOption() {
  return {
    backgroundColor: "transparent",
    grid: { left: 52, right: 16, top: 36, bottom: 28 },
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number) => `${formatBytes(v)}/s`,
    },
    legend: {
      data: ["接收 RX", "发送 TX"],
      textStyle: { color: "#8b949e" },
      top: 6,
    },
    xAxis: {
      type: "category",
      boundaryGap: false,
      data: [],
      axisLabel: { color: "#8b949e", fontSize: 10 },
      axisLine: { lineStyle: { color: "#30363d" } },
    },
    yAxis: {
      type: "value",
      axisLabel: {
        color: "#8b949e",
        fontSize: 10,
        formatter: (v: number) => formatBytes(v) + "/s",
      },
      splitLine: { lineStyle: { color: "#30363d" } },
    },
    series: [
      {
        name: "接收 RX",
        type: "line",
        smooth: true,
        showSymbol: false,
        data: [],
        itemStyle: { color: "#1d9bf0" },
        areaStyle: { opacity: 0.15 },
      },
      {
        name: "发送 TX",
        type: "line",
        smooth: true,
        showSymbol: false,
        data: [],
        itemStyle: { color: "#39d0d8" },
        areaStyle: { opacity: 0.12 },
      },
    ],
  } as echarts.EChartsCoreOption;
}

function initCharts() {
  if (netEl.value) {
    netChart = echarts.init(netEl.value, "dark");
    const opt = netChartOption();
    const s = stats.value;
    // 若 store 已有历史（切回来的情况），直接灌进去恢复曲线
    if (s.labels.length) {
      (opt as any).xAxis.data = s.labels;
      (opt as any).series[0].data = s.netRxRateHistory;
      (opt as any).series[1].data = s.netTxRateHistory;
    }
    netChart.setOption(opt);
  }
}

let unlisten: (() => void) | null = null;
let pollTimer: number | null = null;
let tickTimer: number | null = null;
const STATS_INTERVAL = 2; // 秒，与后端 start_stats 对齐

/** 切回来时若 store 有历史，用最后一个点恢复圆环/网络的实时数值。 */
function restoreLiveFromHistory() {
  const s = stats.value;
  const n = s.labels.length;
  if (n === 0) return;
  cpuNow.value = s.cpuHistory[n - 1];
  memNow.value = s.memHistory[n - 1];
  netRxRate.value = s.netRxRateHistory[n - 1];
  netTxRate.value = s.netTxRateHistory[n - 1];
  netRxTotal.value = s.prevRxTotal ?? 0;
  netTxTotal.value = s.prevTxTotal ?? 0;
}

async function startStats() {
  // 先挂监听再启动流：docker stats 打开瞬间会突发推送第一批采样，
  // 若监听器还没注册，Tauri 事件会被丢弃 → 开头几秒无数据。
  unlisten = await api.onStats(hostId.value, (s) => {
    // 仅缓存，聚合在 tick 内完成
    latestByContainer.set(s.container_id || s.name, s);
  });
  try {
    await api.startStats(hostId.value, STATS_INTERVAL);
  } catch (e) {
    console.error("[stats] start_stats 失败:", e);
  }
}

/** 与 stats interval 对齐：把当前各容器最新采样汇总成一个整机点。 */
function aggregateTick() {
  if (latestByContainer.size === 0) return;

  let cpu = 0;
  let mem = 0;
  let rxTotal = 0;
  let txTotal = 0;
  latestByContainer.forEach((s) => {
    cpu += s.cpu_percent;
    mem += s.mem_percent;
    const [rx, tx] = parseNetIO(s.net_io || "");
    rxTotal += rx;
    txTotal += tx;
  });

  const now = Date.now();
  const s = stats.value;
  let rxRate = 0;
  let txRate = 0;
  if (s.prevTickTs != null && s.prevRxTotal != null && s.prevTxTotal != null) {
    const elapsed = Math.max((now - s.prevTickTs) / 1000, 0.001);
    // 容器重启/移除会导致累计值回退，回退时不计入速率
    rxRate = rxTotal >= s.prevRxTotal ? (rxTotal - s.prevRxTotal) / elapsed : 0;
    txRate = txTotal >= s.prevTxTotal ? (txTotal - s.prevTxTotal) / elapsed : 0;
  }
  s.prevTickTs = now;
  s.prevRxTotal = rxTotal;
  s.prevTxTotal = txTotal;

  cpuNow.value = cpu;
  memNow.value = mem;
  netRxRate.value = rxRate;
  netTxRate.value = txRate;
  netRxTotal.value = rxTotal;
  netTxTotal.value = txTotal;

  statsStore.pushPoint(
    hostId.value,
    new Date().toLocaleTimeString(),
    cpu,
    mem,
    rxRate,
    txRate,
  );
  netChart?.setOption({
    xAxis: { data: s.labels },
    series: [{ data: s.netRxRateHistory }, { data: s.netTxRateHistory }],
  });
}

function stopStats() {
  unlisten?.();
  unlisten = null;
  latestByContainer.clear();
  // 注意：不清 store 历史，切回来要恢复曲线
  if (hostId.value) api.stopStats(hostId.value).catch(() => {});
}

function resizeCharts() {
  netChart?.resize();
}

function startTimers() {
  if (tickTimer != null || pollTimer != null) return;
  tickTimer = window.setInterval(aggregateTick, STATS_INTERVAL * 1000);
  pollTimer = window.setInterval(loadAll, 8000);
}

function stopTimers() {
  if (tickTimer) {
    clearInterval(tickTimer);
    tickTimer = null;
  }
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

let inited = false;
onMounted(async () => {
  // 守卫：keep-alive 场景下若路由 param 尚未就绪，跳过初始化
  if (!hostId.value) {
    console.warn("[dashboard] hostId 为空，跳过初始化");
    return;
  }
  await store.ensureConnected(hostId.value);
  await loadAll();
  initCharts();
  // 若 store 有历史（切回来），先用最后一个点恢复实时数值，避免空白
  restoreLiveFromHistory();
  await startStats();
  startTimers();
  window.addEventListener("resize", resizeCharts);
  inited = true;
});

// keep-alive 激活/失活：恢复/暂停 stats 流和轮询，避免后台空跑 + hostId 漂移报错
onActivated(() => {
  if (!inited) return;
  restoreLiveFromHistory();
  if (hostId.value) startStats();
  startTimers();
});

onDeactivated(() => {
  stopTimers();
  stopStats();
});

onBeforeUnmount(() => {
  stopTimers();
  stopStats();
  window.removeEventListener("resize", resizeCharts);
  netChart?.dispose();
});
</script>

<template>
  <div class="page" v-loading="loading">
    <!-- 第 1 行：左 主机信息 / 右 Docker 健康 -->
    <el-row :gutter="16" class="row-top">
      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="fill">
          <template #header>
            <div class="card-title">
              <el-icon><Monitor /></el-icon>
              <span>系统信息</span>
            </div>
          </template>
          <el-descriptions :column="1" border class="sys-info-table" label-class-name="sys-label" v-if="probe">
            <el-descriptions-item label="主机名">
              <span class="mono">{{ probe.hostname || "—" }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="操作系统">
              <span class="mono">{{ probe.os }}</span>
              <el-tag
                v-if="probe.is_wsl2"
                size="small"
                type="info"
                effect="dark"
                style="margin-left: 8px"
                >WSL2</el-tag
              >
              <el-tag
                v-if="probe.is_windows_native"
                size="small"
                type="warning"
                effect="dark"
                style="margin-left: 8px"
                >Windows 原生</el-tag
              >
            </el-descriptions-item>
            <el-descriptions-item label="Docker 版本">
              <span class="mono">{{ probe.docker_version || "未知" }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="Compose">
              <el-tag
                size="small"
                :type="probe.has_compose ? 'success' : 'danger'"
                effect="dark"
              >
                {{ probe.has_compose ? "已安装 v2" : "未安装" }}
              </el-tag>
            </el-descriptions-item>
          </el-descriptions>
          <el-empty v-else description="未获取到主机信息" :image-size="60" />
        </el-card>
      </el-col>

      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="fill health-card">
          <template #header>
            <div class="card-title">
              <el-icon><Cpu /></el-icon>
              <span>健康</span>
              <el-tag
                v-if="probe"
                size="small"
                type="success"
                effect="dark"
                style="margin-left: auto"
                >在线</el-tag
              >
            </div>
          </template>
          <div class="health-grid">
            <div class="health-item">
              <div class="health-num">{{ projectCount }}</div>
              <div class="health-label">Compose 项目</div>
            </div>
            <div class="health-item">
              <div class="health-num">{{ images.length }}</div>
              <div class="health-label">镜像</div>
            </div>
            <div class="health-item">
              <div class="health-num">{{ containers.length }}</div>
              <div class="health-label">容器</div>
            </div>
          </div>
          <div class="health-running">
            <span class="dot" />
            运行中
            <strong>{{ runningCount }}</strong> /
            {{ containers.length }}
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 第 2 行：监控 - CPU / 内存 -->
    <el-row :gutter="16" class="row-monitor">
      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="metric-card">
          <template #header>
            <div class="card-title">
              <el-icon><Cpu /></el-icon>
              <span>CPU 使用率</span>
            </div>
          </template>
          <div class="ring-wrap">
            <el-progress
              type="circle"
              :percentage="cpuPct"
              :width="120"
              :stroke-width="10"
              :color="cpuColor"
            >
              <template #default="{ percentage }">
                <div class="ring-inner">
                  <el-icon class="ring-icon" :style="{ color: cpuColor }">
                    <Cpu />
                  </el-icon>
                  <div class="ring-num">{{ percentage.toFixed(1) }}%</div>
                </div>
              </template>
            </el-progress>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="metric-card">
          <template #header>
            <div class="card-title">
              <el-icon><Coin /></el-icon>
              <span>内存使用率</span>
            </div>
          </template>
          <div class="ring-wrap">
            <el-progress
              type="circle"
              :percentage="memPct"
              :width="120"
              :stroke-width="10"
              :color="memColor"
            >
              <template #default="{ percentage }">
                <div class="ring-inner">
                  <el-icon class="ring-icon" :style="{ color: memColor }">
                    <Coin />
                  </el-icon>
                  <div class="ring-num">{{ percentage.toFixed(1) }}%</div>
                </div>
              </template>
            </el-progress>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 第 3 行：监控 - 网络实时状态（整宽） -->
    <el-card shadow="never" class="net-card">
      <template #header>
        <div class="card-title">
          <el-icon><Connection /></el-icon>
          <span>网络实时状态</span>
          <span class="net-summary">
            <span class="net-rx">
              ↓ {{ formatBytes(netRxRate) }}/s
              <em>累计 {{ formatBytes(netRxTotal) }}</em>
            </span>
            <span class="net-tx">
              ↑ {{ formatBytes(netTxRate) }}/s
              <em>累计 {{ formatBytes(netTxTotal) }}</em>
            </span>
          </span>
        </div>
      </template>
      <div ref="netEl" class="net-chart" />
    </el-card>
  </div>
</template>

<script lang="ts">
import { Monitor, Cpu, Coin, Connection } from "@element-plus/icons-vue";
export default { name: "DashboardView", components: { Monitor, Cpu, Coin, Connection } };
</script>

<style scoped>
.page {
  padding: 16px 24px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

/* 行内卡片等高 */
.row-top,
.row-monitor {
  margin-bottom: 0;
  flex-shrink: 0;
}
.row-top :deep(.el-col),
.row-monitor :deep(.el-col) {
  display: flex;
}
.row-top :deep(.el-card),
.row-monitor :deep(.el-card) {
  width: 100%;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
}

/* ===== 健康卡 ===== */
.health-card {
  display: flex;
  flex-direction: column;
}
/* 系统信息：固定 label 列宽度，避免主机名长短不同导致切换时错位。
   el-descriptions 是 table 布局，必须 table-layout:fixed 才能让列宽严格生效 */
:deep(.sys-info-table table) {
  table-layout: fixed;
}
:deep(.sys-label) {
  width: 96px;
}
:deep(.health-card .el-card__body) {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}
.health-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  text-align: center;
}
.health-num {
  font-size: 28px;
  font-weight: 600;
  color: var(--el-color-primary);
  line-height: 1.2;
}
.health-label {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin-top: 4px;
}
.health-running {
  margin-top: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.health-running strong {
  color: var(--el-color-success);
  font-size: 16px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--el-color-success);
  box-shadow: 0 0 6px var(--el-color-success);
}

/* ===== 监控卡（CPU/内存 圆环 + 网络） ===== */
.metric-card,
.net-card {
  display: flex;
  flex-direction: column;
}
/* 网络卡占据页面剩余高度 */
.net-card {
  flex: 1;
  min-height: 0;
}
:deep(.metric-card .el-card__body),
:deep(.net-card .el-card__body) {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* CPU/内存：圆环居中 */
.ring-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 0;
}
.ring-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  line-height: 1.1;
}
.ring-icon {
  font-size: 22px;
  margin-bottom: 2px;
}
.ring-num {
  font-size: 18px;
  font-weight: 600;
}
/* 网络：折线图填充剩余空间 */
.net-chart {
  flex: 1;
  min-height: 0;
  width: 100%;
}
/* 压缩 el-card header 高度，节省首屏空间 */
:deep(.el-card__header) {
  padding: 10px 16px;
}
:deep(.el-card__body) {
  padding: 12px 16px;
}

.net-summary {
  margin-left: auto;
  display: flex;
  gap: 18px;
  font-size: 13px;
  font-weight: 400;
}
.net-rx {
  color: #1d9bf0;
}
.net-tx {
  color: #39d0d8;
}
.net-summary em {
  color: var(--el-text-color-secondary);
  font-style: normal;
  margin-left: 4px;
  font-size: 12px;
}

@media (max-width: 768px) {
  .net-chart {
    height: 180px;
    flex: none;
  }
}
</style>
