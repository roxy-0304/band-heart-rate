/// <reference types="@tauri-apps/api" />
import { listen } from "@tauri-apps/api/event";

// ===== Type Definitions =====

interface HeartRatePayload {
  heart_rate: number;
  sensor_contact: boolean | null;
  connected: boolean;
  scanning: boolean;
}

interface ReadingPoint {
  time: number;
  value: number;
}

type ZoneName = "warmup" | "fatburn" | "aerobic" | "limit";

// ===== Constants =====

/** Maximum heart rate reference (220 - 30 default age) */
const MAX_HR = 190;
/** Number of points to keep in chart */
const CHART_POINTS = 300;
/** Heart rate zones [min, max) as percentage of MAX_HR */
const ZONES: { name: ZoneName; label: string; min: number; max: number }[] = [
  { name: "warmup", label: "热身", min: 0, max: 0.60 },
  { name: "fatburn", label: "燃脂", min: 0.60, max: 0.70 },
  { name: "aerobic", label: "有氧", min: 0.70, max: 0.80 },
  { name: "limit", label: "极限", min: 0.80, max: 1.00 },
];

const ZONE_COLORS: Record<ZoneName, string> = {
  warmup: "#4FC3F7",
  fatburn: "#66BB6A",
  aerobic: "#FFA726",
  limit: "#EF5350",
};

const ZONE_BG_COLORS: Record<ZoneName, string> = {
  warmup: "rgba(79, 195, 247, 0.06)",
  fatburn: "rgba(102, 187, 106, 0.06)",
  aerobic: "rgba(255, 167, 38, 0.06)",
  limit: "rgba(239, 83, 80, 0.06)",
};

// ===== DOM References =====

const statusDot = document.getElementById("status-dot")!;
const statusText = document.getElementById("status-text")!;
const heartIcon = document.getElementById("heart-icon")!;
const bpmNumber = document.getElementById("bpm-number")!;
const zoneBadge = document.getElementById("zone-badge")!;
const statMin = document.getElementById("stat-min")!;
const statMax = document.getElementById("stat-max")!;
const statAvg = document.getElementById("stat-avg")!;
const canvas = document.getElementById("heart-rate-chart") as HTMLCanvasElement;
const btnRecord = document.getElementById("btn-record")!;
const btnReset = document.getElementById("btn-reset")!;
const recordingIndicator = document.getElementById("recording-indicator")!;

// ===== State =====

let readings: ReadingPoint[] = [];
let isRecording = false;
let chartAnimId: number | null = null;
let hasData = false;

// Stats (computed on the fly)
let statsMin = 0;
let statsMax = 0;
let statsSum = 0;
let statsCount = 0;

// Last known good BPM (for zone display)
let latestBpm = 0;
let isConnected = false;
let isScanning = false;

// ===== Canvas Setup =====

const ctx = canvas.getContext("2d")!;

function resizeCanvas() {
  const rect = canvas.parentElement!.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;
  canvas.style.width = rect.width + "px";
  canvas.style.height = rect.height + "px";
}

window.addEventListener("resize", () => {
  resizeCanvas();
  if (readings.length > 0) drawChart();
});

// ===== Heart Rate Zone Logic =====

function getZone(bpm: number): ZoneName | null {
  if (bpm <= 0) return null;
  const pct = bpm / MAX_HR;
  for (const z of ZONES) {
    if (pct >= z.min && pct < z.max) return z.name;
  }
  return "limit";
}

function getZoneLabel(zone: ZoneName | null): string {
  if (!zone) return "--";
  return ZONES.find(z => z.name === zone)?.label ?? "--";
}

// ===== Update UI =====

function updateStatus(connected: boolean, scanning: boolean) {
  isConnected = connected;
  isScanning = scanning;

  statusDot.className = "status-dot";
  statusText.className = "status-text";

  if (connected) {
    statusDot.classList.add("connected");
    statusText.classList.add("connected");
    statusText.textContent = "已连接";
    heartIcon.classList.remove("paused");
  } else if (scanning) {
    statusDot.classList.add("scanning");
    statusText.classList.add("scanning");
    statusText.textContent = "扫描中...";
    heartIcon.classList.add("paused");
  } else {
    statusDot.classList.add("disconnected");
    statusText.classList.add("disconnected");
    statusText.textContent = "断开连接";
    heartIcon.classList.add("paused");
  }
}

function updateHeartRate(bpm: number) {
  if (bpm <= 0) {
    bpmNumber.textContent = "--";
    bpmNumber.className = "bpm-number no-data";
    zoneBadge.className = "zone-badge no-data";
    zoneBadge.textContent = "--";
    heartIcon.classList.add("paused");
    return;
  }

  bpmNumber.textContent = String(bpm);
  heartIcon.classList.remove("paused");

  const zone = getZone(bpm);
  if (zone) {
    bpmNumber.className = `bpm-number zone-${zone}`;
    zoneBadge.className = `zone-badge zone-${zone}`;
    zoneBadge.textContent = getZoneLabel(zone);
  }
}

function updateStats(value: number) {
  if (!isRecording) return;

  // First data point
  if (statsCount === 0) {
    statsMin = value;
    statsMax = value;
    statsSum = value;
    statsCount = 1;
  } else {
    if (value < statsMin) statsMin = value;
    if (value > statsMax) statsMax = value;
    statsSum += value;
    statsCount++;
  }

  // Update DOM
  const avg = Math.round(statsSum / statsCount);

  statMin.textContent = String(statsMin);
  statMin.className = "stat-value min";
  statMax.textContent = String(statsMax);
  statMax.className = "stat-value max";
  statAvg.textContent = String(avg);
  statAvg.className = "stat-value avg";
}

function resetStats() {
  statsMin = 0;
  statsMax = 0;
  statsSum = 0;
  statsCount = 0;

  statMin.textContent = "--";
  statMin.className = "stat-value no-data min";
  statMax.textContent = "--";
  statMax.className = "stat-value no-data max";
  statAvg.textContent = "--";
  statAvg.className = "stat-value no-data avg";
  readings = [];
  hasData = false;
  latestBpm = 0;
}

function handleData(bpm: number) {
  if (bpm <= 0) return;

  latestBpm = bpm;
  hasData = true;

  // Add to chart buffer
  const now = performance.now();
  readings.push({ time: now, value: bpm });
  if (readings.length > CHART_POINTS) {
    readings = readings.slice(readings.length - CHART_POINTS);
  }

  updateHeartRate(bpm);
  updateStats(bpm);
  drawChart();
}

// ===== Canvas Chart Drawing =====

function drawChart() {
  const rect = canvas.parentElement!.getBoundingClientRect();
  const w = rect.width;
  const h = rect.height;
  const dpr = window.devicePixelRatio || 1;

  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  if (readings.length < 2) {
    // Draw empty state
    ctx.fillStyle = "#8b949e";
    ctx.textAlign = "center";
    ctx.font = "13px 'Segoe UI', sans-serif";
    ctx.fillText("等待心率数据...", w / 2, h / 2);
    ctx.restore();
    return;
  }

  const padLeft = 36;
  const padRight = 8;
  const padTop = 16;
  const padBottom = 20;
  const chartW = w - padLeft - padRight;
  const chartH = h - padTop - padBottom;

  // Determine Y range (add 10% padding above max)
  const values = readings.map(p => p.value);
  const vMin = Math.min(...values);
  const vMax = Math.max(...values);
  const vRange = Math.max(vMax - vMin, 20);
  const vPad = vRange * 0.1;
  const yMin = Math.max(0, vMin - vPad);
  const yMax = vMax + vPad;

  function yToPixel(val: number): number {
    return padTop + chartH - ((val - yMin) / (yMax - yMin)) * chartH;
  }

  function xToPixel(idx: number): number {
    if (readings.length <= 1) return padLeft;
    return padLeft + (idx / (readings.length - 1)) * chartW;
  }

  // --- Draw zone background bands ---
  for (const z of ZONES) {
    const zoneMin = Math.max(z.min * MAX_HR, yMin);
    const zoneMax = Math.min(z.max * MAX_HR, yMax);
    if (zoneMin >= zoneMax) continue;

    const yTop = Math.min(yToPixel(zoneMin), yToPixel(zoneMax));
    const yBot = Math.max(yToPixel(zoneMin), yToPixel(zoneMax));
    const bandH = yBot - yTop;

    ctx.fillStyle = ZONE_BG_COLORS[z.name];
    ctx.fillRect(padLeft, yTop, chartW, bandH);
  }

  // --- Y axis labels ---
  const yTicks = 4;
  ctx.fillStyle = "#8b949e";
  ctx.textAlign = "right";
  ctx.font = "10px 'Cascadia Code', 'Fira Code', monospace";
  for (let i = 0; i <= yTicks; i++) {
    const val = yMin + (yMax - yMin) * (i / yTicks);
    const y = yToPixel(val);
    ctx.fillText(String(Math.round(val)), padLeft - 4, y + 3);
    // Grid line
    ctx.strokeStyle = "rgba(48, 54, 61, 0.5)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padLeft, y);
    ctx.lineTo(padLeft + chartW, y);
    ctx.stroke();
  }

  // --- Draw heart rate curve ---
  const linePoints = readings.map((p, i) => ({ x: xToPixel(i), y: yToPixel(p.value) }));

  // Fill under curve
  ctx.beginPath();
  ctx.moveTo(linePoints[0].x, padTop + chartH);
  for (const pt of linePoints) {
    ctx.lineTo(pt.x, pt.y);
  }
  ctx.lineTo(linePoints[linePoints.length - 1].x, padTop + chartH);
  ctx.closePath();

  const gradient = ctx.createLinearGradient(0, padTop, 0, padTop + chartH);
  gradient.addColorStop(0, "rgba(255, 59, 48, 0.2)");
  gradient.addColorStop(1, "rgba(255, 59, 48, 0.02)");
  ctx.fillStyle = gradient;
  ctx.fill();

  // Stroke the curve
  ctx.beginPath();
  ctx.moveTo(linePoints[0].x, linePoints[0].y);
  for (let i = 1; i < linePoints.length; i++) {
    ctx.lineTo(linePoints[i].x, linePoints[i].y);
  }
  ctx.strokeStyle = "#FF3B30";
  ctx.lineWidth = 2;
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  ctx.shadowColor = "rgba(255, 59, 48, 0.4)";
  ctx.shadowBlur = 6;
  ctx.stroke();
  ctx.shadowBlur = 0;

  // --- Current value dot (last point) ---
  const last = linePoints[linePoints.length - 1];
  const gradientDot = ctx.createRadialGradient(last.x, last.y, 0, last.x, last.y, 6);
  gradientDot.addColorStop(0, "rgba(255, 59, 48, 1)");
  gradientDot.addColorStop(1, "rgba(255, 59, 48, 0)");
  ctx.fillStyle = gradientDot;
  ctx.beginPath();
  ctx.arc(last.x, last.y, 6, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#FF3B30";
  ctx.beginPath();
  ctx.arc(last.x, last.y, 2.5, 0, Math.PI * 2);
  ctx.fill();

  ctx.restore();
}

// ===== Animation Loop =====

let lastBpm = 0;

function animationLoop() {
  if (latestBpm !== lastBpm) {
    lastBpm = latestBpm;
    drawChart();
  }
  chartAnimId = requestAnimationFrame(animationLoop);
}

// ===== Tauri Event Listener =====

async function init() {
  resizeCanvas();

  // Start animation loop (optimized: only redraws when data changes)
  animationLoop();

  // Listen for heart rate updates from Rust backend
  await listen<HeartRatePayload>("hr-update", (event) => {
    const data = event.payload;

    // Update connection status
    updateStatus(data.connected, data.scanning);

    // Update heart rate display
    if (data.connected && data.heart_rate > 0) {
      handleData(data.heart_rate);
    } else if (!data.connected && !data.scanning) {
      // Disconnected — reset display but keep stats
      if (latestBpm !== 0) {
        bpmNumber.textContent = "--";
        bpmNumber.className = "bpm-number no-data";
        zoneBadge.className = "zone-badge no-data";
        zoneBadge.textContent = "--";
        latestBpm = 0;
      }
    }
  });

  // ===== Button Handlers =====

  btnRecord.addEventListener("click", () => {
    isRecording = !isRecording;

    if (isRecording) {
      btnRecord.classList.add("recording");
      btnRecord.innerHTML = "<span>⏹</span> 停止记录";
      recordingIndicator.classList.add("active");

      // Reset stats when starting a new recording session
      resetStats();
      // Keep existing chart data if any
      statsCount = 0;
    } else {
      btnRecord.classList.remove("recording");
      btnRecord.innerHTML = "<span>▶</span> 开始记录";
      recordingIndicator.classList.remove("active");
    }
  });

  btnReset.addEventListener("click", () => {
    resetStats();
    readings = [];
    hasData = false;
    latestBpm = 0;
    drawChart();

    // If not recording, also reset the heart display
    if (!isRecording) {
      bpmNumber.textContent = "--";
      bpmNumber.className = "bpm-number no-data";
      zoneBadge.className = "zone-badge no-data";
      zoneBadge.textContent = "--";
      heartIcon.classList.add("paused");
    }
  });
}

// ===== Entry Point =====

document.addEventListener("DOMContentLoaded", () => {
  init().catch(console.error);
});
