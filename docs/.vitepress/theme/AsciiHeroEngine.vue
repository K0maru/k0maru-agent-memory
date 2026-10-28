<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRoute } from 'vitepress'

const props = withDefaults(
  defineProps<{
    mode?: 'all' | 'bg' | 'showcase'
  }>(),
  {
    mode: 'all',
  }
)

// --- Types & Constants ---
type FilterType = 'ascii' | 'scanline' | 'dither'
type PaletteName = 'meadow' | 'mono' | 'amber' | 'blue'

interface PaletteStops {
  name: PaletteName
  label: string
  stops: [string, string, string, string] // 0, 0.4, 0.74, 1
  rgb: [[number, number, number], [number, number, number], [number, number, number], [number, number, number]]
}

const PALETTES: Record<PaletteName, PaletteStops> = {
  meadow: {
    name: 'meadow',
    label: 'Meadow',
    stops: ['#2B361F', '#677D45', '#AEBA78', '#E5E1BD'],
    rgb: [
      [43, 54, 31],
      [103, 125, 69],
      [174, 186, 120],
      [229, 225, 189],
    ],
  },
  mono: {
    name: 'mono',
    label: 'Mono',
    stops: ['#151515', '#575757', '#A6A6A6', '#ECECEC'],
    rgb: [
      [21, 21, 21],
      [87, 87, 87],
      [166, 166, 166],
      [236, 236, 236],
    ],
  },
  amber: {
    name: 'amber',
    label: 'Amber',
    stops: ['#3C200F', '#A74C15', '#EF9A46', '#F7E7CA'],
    rgb: [
      [60, 32, 15],
      [167, 76, 21],
      [239, 154, 70],
      [247, 231, 202],
    ],
  },
  blue: {
    name: 'blue',
    label: 'Blue',
    stops: ['#192A4D', '#326AC5', '#A2C2F1', '#EDF3FC'],
    rgb: [
      [25, 42, 77],
      [50, 106, 197],
      [162, 194, 241],
      [237, 243, 252],
    ],
  },
}

// 17-step character ramp from empty to dense (starts with two spaces)
const ASCII_CHARS = "  .,:;-=+xX08S#&@"

// Bayer 4x4 Ordered Matrix
const BAYER_4X4 = [
  [0, 8, 2, 10],
  [12, 4, 14, 6],
  [3, 11, 1, 9],
  [15, 7, 13, 5],
]

// --- State & Calibrated Constants ---
const activeFilter = ref<FilterType>('scanline')
const activePalette = ref<PaletteName>('meadow')
const MASK_VEIL = 0.90
const DENSITY = 0.88
const isMounted = ref(false)
const heroContainer = ref<HTMLElement | null>(null)
const bgCanvasRef = ref<HTMLCanvasElement | null>(null)
const showcaseCanvasRef = ref<HTMLCanvasElement | null>(null)

// --- Color Helpers ---
function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * Math.max(0, Math.min(1, t))
}

function samplePaletteColor(
  pal: PaletteStops,
  tone: number,
  alpha = 1
): string {
  const t = Math.max(0, Math.min(1, tone))
  let r: number, g: number, b: number

  if (t <= 0.4) {
    const u = t / 0.4
    r = lerp(pal.rgb[0][0], pal.rgb[1][0], u)
    g = lerp(pal.rgb[0][1], pal.rgb[1][1], u)
    b = lerp(pal.rgb[0][2], pal.rgb[1][2], u)
  } else if (t <= 0.74) {
    const u = (t - 0.4) / 0.34
    r = lerp(pal.rgb[1][0], pal.rgb[2][0], u)
    g = lerp(pal.rgb[1][1], pal.rgb[2][1], u)
    b = lerp(pal.rgb[1][2], pal.rgb[2][2], u)
  } else {
    const u = (t - 0.74) / 0.26
    r = lerp(pal.rgb[2][0], pal.rgb[3][0], u)
    g = lerp(pal.rgb[2][1], pal.rgb[3][1], u)
    b = lerp(pal.rgb[2][2], pal.rgb[3][2], u)
  }

  if (alpha >= 0.999) {
    return `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`
  }
  return `rgba(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}, ${alpha.toFixed(3)})`
}

function smoothstep(x: number): number {
  const c = Math.max(0, Math.min(1, x))
  return c * c * (3 - 2 * c)
}

// --- Offscreen Luminance Sampler ---
interface LuminanceMap {
  width: number
  height: number
  data: Float32Array
  getL: (x: number, y: number) => number
}

function buildLuminanceMap(
  width: number,
  height: number,
  drawFn: (ctx: CanvasRenderingContext2D, w: number, h: number) => void
): LuminanceMap {
  const offCanvas = document.createElement('canvas')
  offCanvas.width = width
  offCanvas.height = height
  const ctx = offCanvas.getContext('2d')
  if (!ctx) {
    const empty = new Float32Array(width * height)
    return { width, height, data: empty, getL: () => 0 }
  }

  ctx.fillStyle = '#000000'
  ctx.fillRect(0, 0, width, height)
  drawFn(ctx, width, height)

  const imgData = ctx.getImageData(0, 0, width, height)
  const d = imgData.data
  const len = width * height
  const data = new Float32Array(len)

  for (let i = 0; i < len; i++) {
    const idx = i * 4
    const r = d[idx]
    const g = d[idx + 1]
    const b = d[idx + 2]
    // Standard perceptual luminance L
    data[i] = (0.299 * r + 0.587 * g + 0.114 * b) / 255
  }

  return {
    width,
    height,
    data,
    getL: (x: number, y: number) => {
      const ix = Math.floor(Math.max(0, Math.min(width - 1, x)))
      const iy = Math.floor(Math.max(0, Math.min(height - 1, y)))
      return data[iy * width + ix]
    },
  }
}

// --- Sculptural 3D-Faceted Architectural "K" + Particle Wake ---
// Designed with continuous directional luminance gradients (0.0 -> 1.0)
// so Particle Scanline, Volumetric ASCII, and Bayer 4x4 Dither all render
// genuine 3D chiseled facets, bevel highlights, and trailing particle dissolution.
function drawHeroEmblem(ctx: CanvasRenderingContext2D, w: number, h: number) {
  const cx = w * 0.48
  const cy = h * 0.50

  ctx.save()
  ctx.translate(cx, cy)

  // 1. Subtle asymmetric ambient field behind the K (gives ASCII/Dither a soft penumbra)
  const aura = ctx.createRadialGradient(-10, -10, 12, 10, 0, Math.min(w, h) * 0.46)
  aura.addColorStop(0, 'rgba(255, 255, 255, 0.16)')
  aura.addColorStop(0.55, 'rgba(255, 255, 255, 0.06)')
  aura.addColorStop(1, 'rgba(0, 0, 0, 0)')
  ctx.fillStyle = aura
  ctx.beginPath()
  ctx.arc(0, 0, Math.min(w, h) * 0.46, 0, Math.PI * 2)
  ctx.fill()

  // =========================================================================
  // 2. ARCHITECTURAL "K" — THREE CHISELED 3D FACETED PLANES
  // =========================================================================

  // --- PLANE A: Vertical Monolith Stem (x: -88..-40, y: -106..106) ---
  // Directional bevel gradient: bright top-left ridge (1.0) -> mid-tone lower-right (0.48)
  const stemGrad = ctx.createLinearGradient(-88, -106, -36, 106)
  stemGrad.addColorStop(0, 'rgb(255, 255, 255)')
  stemGrad.addColorStop(0.45, 'rgb(215, 215, 215)')
  stemGrad.addColorStop(0.85, 'rgb(140, 140, 140)')
  stemGrad.addColorStop(1, 'rgb(105, 105, 105)')
  ctx.fillStyle = stemGrad
  ctx.beginPath()
  ctx.moveTo(-88, -106)
  ctx.lineTo(-40, -106)
  ctx.lineTo(-40, -14)
  // Subtle architectural inner chamfer notch at the nexus
  ctx.lineTo(-46, 0)
  ctx.lineTo(-40, 14)
  ctx.lineTo(-40, 106)
  ctx.lineTo(-88, 106)
  ctx.closePath()
  ctx.fill()

  // Bright left-edge specular highlight bar on stem
  const stemSpec = ctx.createLinearGradient(-88, -106, -74, 106)
  stemSpec.addColorStop(0, 'rgba(255, 255, 255, 0.95)')
  stemSpec.addColorStop(0.6, 'rgba(255, 255, 255, 0.45)')
  stemSpec.addColorStop(1, 'rgba(255, 255, 255, 0.0)')
  ctx.fillStyle = stemSpec
  ctx.fillRect(-88, -106, 11, 212)

  // --- PLANE B: Upper Diagonal Arm (Sweeping from nexus (-36, -6) to (46, -106)..(96, -106)) ---
  // Chiseled into two longitudinal facets (upper sunlit ridge + lower shaded bevel)
  const upperRidgeGrad = ctx.createLinearGradient(-36, -10, 72, -106)
  upperRidgeGrad.addColorStop(0, 'rgb(165, 165, 165)')
  upperRidgeGrad.addColorStop(0.35, 'rgb(235, 235, 235)')
  upperRidgeGrad.addColorStop(1, 'rgb(255, 255, 255)')
  ctx.fillStyle = upperRidgeGrad
  ctx.beginPath()
  ctx.moveTo(-36, -8)
  ctx.lineTo(44, -106)
  ctx.lineTo(94, -106)
  ctx.lineTo(-6, 14)
  ctx.closePath()
  ctx.fill()

  // Lower bevel shadow plane on Upper Arm for 3D depth
  const upperShadowGrad = ctx.createLinearGradient(-36, 4, 88, -98)
  upperShadowGrad.addColorStop(0, 'rgb(75, 75, 75)')
  upperShadowGrad.addColorStop(0.5, 'rgb(125, 125, 125)')
  upperShadowGrad.addColorStop(1, 'rgb(185, 185, 185)')
  ctx.fillStyle = upperShadowGrad
  ctx.beginPath()
  ctx.moveTo(-22, 2)
  ctx.lineTo(68, -106)
  ctx.lineTo(94, -106)
  ctx.lineTo(-6, 14)
  ctx.closePath()
  ctx.fill()

  // --- PLANE C: Lower Diagonal Leg (Emerging from beneath upper arm with cast shadow) ---
  const lowerLegGrad = ctx.createLinearGradient(-24, -2, 92, 106)
  lowerLegGrad.addColorStop(0, 'rgb(70, 70, 70)')     // Deep cast shadow under upper arm
  lowerLegGrad.addColorStop(0.28, 'rgb(155, 155, 155)')
  lowerLegGrad.addColorStop(0.68, 'rgb(230, 230, 230)')
  lowerLegGrad.addColorStop(1, 'rgb(255, 255, 255)')    // Bright foot terminal
  ctx.fillStyle = lowerLegGrad
  ctx.beginPath()
  ctx.moveTo(-12, -10)
  ctx.lineTo(18, -26)
  ctx.lineTo(104, 106)
  ctx.lineTo(52, 106)
  ctx.lineTo(-36, 8)
  ctx.closePath()
  ctx.fill()

  // Specular ridge along the outer edge of the lower leg
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.92)'
  ctx.lineWidth = 3.5
  ctx.beginPath()
  ctx.moveTo(16, -24)
  ctx.lineTo(101, 105)
  ctx.stroke()

  // =========================================================================
  // 3. RETRO BAUHAUS / IBM ARCHITECTURAL SCAN GROOVES & PARTICLE DISSOLUTION
  // =========================================================================
  // Subtle dark horizontal precision grooves cut into the top & bottom of the stem
  // and right terminals (classic 1970s IBM / Braun hardware motif)
  ctx.fillStyle = '#000000'
  const grooveYs = [-94, -84, 84, 94]
  for (const gy of grooveYs) {
    ctx.fillRect(-89, gy, 50, 2.2)
  }

  // Particle erosion / crystallization along the right edge of the K arms:
  // Deterministic cluster of luminous squares & micro-bars trailing to the right
  for (let i = 0; i < 64; i++) {
    // Golden-ratio quasi-random sequence for balanced retro-tech composition
    const u = ((i * 0.61803398875) % 1)
    const v = (((i * 1.32471795724) + 0.23) % 1)
    const isUpper = i % 2 === 0
    const tArm = u
    // Anchor along upper arm or lower leg outer edge
    const baseX = isUpper ? (10 + tArm * 86) : (14 + tArm * 90)
    const baseY = isUpper ? (-12 - tArm * 92) : (-16 + tArm * 120)
    const spreadX = 8 + v * 58 * (0.35 + tArm * 0.65)
    const spreadY = (v - 0.5) * 28
    const px = baseX + spreadX
    const py = baseY + spreadY
    const lum = Math.max(0.18, (1 - v * 0.78) * (0.5 + 0.5 * tArm))
    const sz = 2.0 + (1 - v) * 3.8
    const gVal = Math.round(lum * 255)
    ctx.fillStyle = `rgb(${gVal}, ${gVal}, ${gVal})`
    ctx.fillRect(px, py, sz, sz)
  }

  ctx.restore()
}
// 6 Feature Emblems (Scaled strictly for 116x52 watermark header strip)
function drawEmblem01(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 01 // 3.4MS: Lightning speed vector + binary ticks
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.fillStyle = '#FFFFFF'
  ctx.lineWidth = 2.5

  ctx.beginPath()
  ctx.moveTo(cx + 6, cy - 20)
  ctx.lineTo(cx - 12, cy - 2)
  ctx.lineTo(cx - 1, cy - 2)
  ctx.lineTo(cx - 8, cy + 20)
  ctx.lineTo(cx + 14, cy + 2)
  ctx.lineTo(cx + 3, cy + 2)
  ctx.closePath()
  ctx.fill()

  ctx.lineWidth = 1.5
  for (let i = -3; i <= 3; i++) {
    const x = cx + i * 14
    const barH = 5 + Math.abs(i) * 3
    ctx.beginPath()
    ctx.moveTo(x, cy + 18)
    ctx.lineTo(x, cy + 18 - barH)
    ctx.stroke()
  }
}

function drawEmblem02(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 02 // BYOM: Markdown document nodes & bidirectional links
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.fillStyle = '#FFFFFF'

  ctx.strokeRect(cx - 36, cy - 16, 22, 32)
  ctx.fillRect(cx - 31, cy - 9, 12, 2.5)
  ctx.fillRect(cx - 31, cy - 3, 10, 2.5)
  ctx.fillRect(cx - 31, cy + 3, 12, 2.5)

  ctx.strokeRect(cx + 14, cy - 16, 22, 32)
  ctx.fillRect(cx + 19, cy - 9, 12, 2.5)
  ctx.fillRect(cx + 19, cy - 3, 8, 2.5)
  ctx.fillRect(cx + 19, cy + 3, 12, 2.5)

  ctx.lineWidth = 2
  ctx.beginPath()
  ctx.moveTo(cx - 10, cy)
  ctx.lineTo(cx + 10, cy)
  ctx.stroke()
}

function drawEmblem03(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 03 // RRF-60: Tri-modal rings (Venn) with fusion core
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.lineWidth = 2
  const r = 12

  ctx.beginPath()
  ctx.arc(cx, cy - 7, r, 0, Math.PI * 2)
  ctx.stroke()

  ctx.beginPath()
  ctx.arc(cx - 9, cy + 7, r, 0, Math.PI * 2)
  ctx.stroke()

  ctx.beginPath()
  ctx.arc(cx + 9, cy + 7, r, 0, Math.PI * 2)
  ctx.stroke()

  ctx.fillStyle = '#FFFFFF'
  ctx.beginPath()
  ctx.arc(cx, cy + 2, 4, 0, Math.PI * 2)
  ctx.fill()
}

function drawEmblem04(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 04 // <10MS: Living ADR Architectural Shield / Gate Lock
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.fillStyle = '#FFFFFF'
  ctx.lineWidth = 2

  ctx.beginPath()
  const r = 18
  for (let i = 0; i < 6; i++) {
    const ang = (i / 6) * Math.PI * 2 - Math.PI / 2
    const px = cx + Math.cos(ang) * r
    const py = cy + Math.sin(ang) * r
    if (i === 0) ctx.moveTo(px, py)
    else ctx.lineTo(px, py)
  }
  ctx.closePath()
  ctx.stroke()

  ctx.fillRect(cx - 8, cy - 4, 16, 12)
  ctx.beginPath()
  ctx.arc(cx, cy - 4, 5, Math.PI, 0, false)
  ctx.lineWidth = 2.5
  ctx.stroke()
}

function drawEmblem05(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 05 // MCP: FastMCP Stdio Stream Wave & Protocol frames
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.lineWidth = 2

  ctx.beginPath()
  for (let x = cx - 36; x <= cx + 36; x += 3) {
    const y = cy + Math.sin((x - cx) * 0.15) * 11
    if (x === cx - 36) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
  ctx.stroke()

  ctx.setLineDash([3, 3])
  ctx.beginPath()
  ctx.moveTo(cx - 36, cy - 16)
  ctx.lineTo(cx + 36, cy - 16)
  ctx.moveTo(cx - 36, cy + 16)
  ctx.lineTo(cx + 36, cy + 16)
  ctx.stroke()
  ctx.setLineDash([])

  ctx.fillStyle = '#FFFFFF'
  ctx.fillRect(cx - 16, cy - 3, 5, 5)
  ctx.fillRect(cx + 11, cy - 3, 5, 5)
}

function drawEmblem06(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // 06 // A100: 4x4 Systolic Tensor Grid with data vectors
  const cx = w * 0.5
  const cy = h * 0.5
  ctx.strokeStyle = '#FFFFFF'
  ctx.fillStyle = '#FFFFFF'
  ctx.lineWidth = 1.2

  const cellSize = 8
  const startX = cx - 16
  const startY = cy - 16

  for (let r = 0; r < 4; r++) {
    for (let c = 0; c < 4; c++) {
      const px = startX + c * cellSize
      const py = startY + r * cellSize
      ctx.strokeRect(px, py, cellSize - 1.5, cellSize - 1.5)
      if ((r + c) % 2 === 0) {
        ctx.fillRect(px + 1.5, py + 1.5, cellSize - 4.5, cellSize - 4.5)
      }
    }
  }

  ctx.lineWidth = 1.8
  ctx.beginPath()
  ctx.moveTo(cx - 26, cy)
  ctx.lineTo(cx + 26, cy)
  ctx.moveTo(cx, cy - 22)
  ctx.lineTo(cx, cy + 22)
  ctx.stroke()
}

const FEATURE_EMBLEM_DRAWERS = [
  drawEmblem01,
  drawEmblem02,
  drawEmblem03,
  drawEmblem04,
  drawEmblem05,
  drawEmblem06,
]

// --- Animation State & Physics ---
let bgAnimId = 0
let showcaseAnimId = 0
let bgTime = 0
let showcaseTime = 0
let isHeroInViewport = true
let heroObserver: IntersectionObserver | null = null
// Mouse state for showcase
const showcaseMouse = {
  x: -999,
  y: -999,
  isInside: false,
}

// Spring physics state for vertical scanlines + particle lattice in showcase
interface ScanlineCol {
  origX: number
  x: number
  vx: number
  phase: number
}
let scanlineCols: ScanlineCol[] = []

// Free-floating detached memory particles around the K
interface FloatingParticle {
  baseX: number
  baseY: number
  x: number
  y: number
  vx: number
  vy: number
  size: number
  tone: number
  speed: number
  phase: number
}
let floatingParticles: FloatingParticle[] = []

// Offscreen luminance maps cache
let heroLuminanceMap: LuminanceMap | null = null
const featureLuminanceMaps: (LuminanceMap | null)[] = [null, null, null, null, null, null]

// Feature cards cleaner
let featureCleanups: (() => void)[] = []

// Whisper-quiet optical registration marks (Braun / Hasselblad viewfinder style)
function renderOpticalRegistration(
  ctx: CanvasRenderingContext2D,
  pal: PaletteStops,
  w: number,
  h: number
) {
  const pad = 16
  const tick = 6

  ctx.strokeStyle = samplePaletteColor(pal, 0.65, 0.32)
  ctx.lineWidth = 1

  // 4 minimal crosshair corner ticks
  ctx.beginPath()
  // Top-left
  ctx.moveTo(pad, pad + tick)
  ctx.lineTo(pad, pad)
  ctx.lineTo(pad + tick, pad)
  // Top-right
  ctx.moveTo(w - pad - tick, pad)
  ctx.lineTo(w - pad, pad)
  ctx.lineTo(w - pad, pad + tick)
  // Bottom-left
  ctx.moveTo(pad, h - pad - tick)
  ctx.lineTo(pad, h - pad)
  ctx.lineTo(pad + tick, h - pad)
  // Bottom-right
  ctx.moveTo(w - pad - tick, h - pad)
  ctx.lineTo(w - pad, h - pad)
  ctx.lineTo(w - pad, h - pad - tick)
  ctx.stroke()

  // Ultra-subtle micro-typography in top corners (no boxes, no borders)
  ctx.font = '500 8.5px "JetBrains Mono", ui-monospace, monospace'
  ctx.fillStyle = samplePaletteColor(pal, 0.62, 0.45)
  ctx.textBaseline = 'top'
  ctx.textAlign = 'left'
  ctx.fillText('FIG. 01 // K-FIELD', pad + 10, pad + 1)

  ctx.textAlign = 'right'
  ctx.fillText('3.38 MS · RRF-60', w - pad - 10, pad + 1)
}

// --- Background ASCII Loop ---
function initBgCanvas() {
  const canvas = bgCanvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const handleResize = () => {
    if (!canvas) return
    const rect = canvas.parentElement?.getBoundingClientRect()
    const w = rect?.width || window.innerWidth
    const h = rect?.height || window.innerHeight
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    canvas.width = w * dpr
    canvas.height = h * dpr
    canvas.style.width = `${w}px`
    canvas.style.height = `${h}px`
  }

  handleResize()
  window.addEventListener('resize', handleResize)

  const renderBg = () => {
    if (!isHeroInViewport) {
      bgAnimId = requestAnimationFrame(renderBg)
      return
    }
    bgTime += 0.016
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    const w = canvas.width / dpr
    const h = canvas.height / dpr

    ctx.save()
    ctx.scale(dpr, dpr)
    ctx.clearRect(0, 0, w, h)

    const pal = PALETTES[activePalette.value]
    const s = DENSITY
    const currentMask = MASK_VEIL
    // Compute cell dimensions
    const baseCell = 14
    const cellW = baseCell
    const cellH = baseCell * 1.35
    const cols = Math.ceil(w / cellW) + 1
    const rows = Math.ceil(h / cellH) + 1

    ctx.font = `600 ${Math.floor(cellW * 0.9)}px "JetBrains Mono", ui-monospace, monospace`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'

    // @Kubai087 Tone parameters
    const black = 0.22 * s
    const white = 1 - 0.08 * s

    for (let r = 0; r < rows; r++) {
      const y = r * cellH
      const normY = y / Math.max(1, h)

      // 90% Vertical Mask Veil:
      // Top 44%, 42% mid at 20%, bottom 50% scaled by mask
      let maskFactor = 1.0
      if (normY < 0.2) {
        maskFactor = 0.44 * (1 - currentMask * 0.65)
      } else if (normY < 0.5) {
        maskFactor = 0.42 * (1 - currentMask * 0.75)
      } else {
        const u = (normY - 0.5) / 0.5
        maskFactor = (0.35 - u * 0.32) * (1 - currentMask * 0.88)
      }
      maskFactor = Math.max(0.015, maskFactor)

      for (let c = 0; c < cols; c++) {
        const x = c * cellW

        // Topological wave field
        const n1 = Math.sin(c * 0.08 + bgTime * 0.5) * Math.cos(r * 0.09 - bgTime * 0.35)
        const n2 = Math.sin((c + r) * 0.05 + bgTime * 0.25)
        const rawL = (n1 + n2 + 2) / 4

        // Smoothstep tone curve
        const lev = Math.max(0, Math.min(1, (rawL - black) / Math.max(0.001, white - black)))
        const tone = lev * (1 - s) + smoothstep(lev) * s

        const charIdx = Math.floor(tone * (ASCII_CHARS.length - 1))
        const ch = ASCII_CHARS[charIdx]

        if (ch !== ' ') {
          ctx.fillStyle = samplePaletteColor(pal, tone * 0.8, maskFactor * 0.45)
          ctx.fillText(ch, x, y)
        }
      }
    }

    ctx.restore()
    bgAnimId = requestAnimationFrame(renderBg)
  }

  renderBg()

  return () => {
    window.removeEventListener('resize', handleResize)
    cancelAnimationFrame(bgAnimId)
  }
}

// --- Showcase Canvas Loop ---
function initShowcaseCanvas() {
  const canvas = showcaseCanvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const cssW = 480
  const cssH = 340
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  canvas.width = cssW * dpr
  canvas.height = cssH * dpr
  canvas.style.width = '100%'
  canvas.style.height = `${cssH}px`

  // Build offscreen 3D-shaded luminance map for the Sculptural "K"
  heroLuminanceMap = buildLuminanceMap(cssW, cssH, drawHeroEmblem)

  // Initialize 96 ultra-fine vertical columns (@ianneo_ai Cloudflare combing + particle lattice)
  const numCols = 96
  const colSpacing = cssW / numCols
  scanlineCols = []
  for (let i = 0; i < numCols; i++) {
    const orig = i * colSpacing + colSpacing * 0.5
    scanlineCols.push({
      origX: orig,
      x: orig,
      vx: 0,
      phase: (i * 0.37) % (Math.PI * 2),
    })
  }

  // Initialize 110 detached memory particles orbiting and streaming from the K facets
  floatingParticles = []
  for (let i = 0; i < 110; i++) {
    const u = (i * 0.61803398875) % 1
    const v = ((i * 1.32471795724) + 0.17) % 1
    const isUpper = i % 2 === 0
    // Cluster along the K's right diagonal arms and ambient field
    const bx = cssW * 0.48 + (isUpper ? (-10 + u * 135) : (-5 + u * 140)) + (v - 0.3) * 45
    const by = cssH * 0.50 + (isUpper ? (-10 - u * 105) : (10 + u * 105)) + (v - 0.5) * 36
    floatingParticles.push({
      baseX: bx,
      baseY: by,
      x: bx,
      y: by,
      vx: 0,
      vy: 0,
      size: 1.3 + (i % 4) * 0.65,
      tone: 0.35 + (1 - u * 0.65) * 0.65,
      speed: 0.4 + (i % 5) * 0.18,
      phase: i * 0.91,
    })
  }

  // Mouse handlers
  const onMouseMove = (e: MouseEvent) => {
    const rect = canvas.getBoundingClientRect()
    showcaseMouse.x = ((e.clientX - rect.left) / rect.width) * cssW
    showcaseMouse.y = ((e.clientY - rect.top) / rect.height) * cssH
    showcaseMouse.isInside = true
  }

  const onMouseLeave = () => {
    showcaseMouse.isInside = false
    showcaseMouse.x = -999
    showcaseMouse.y = -999
  }

  canvas.addEventListener('mousemove', onMouseMove)
  canvas.addEventListener('mouseleave', onMouseLeave)

  const renderShowcase = () => {
    if (!isHeroInViewport) {
      showcaseAnimId = requestAnimationFrame(renderShowcase)
      return
    }
    showcaseTime += 0.018
    ctx.save()
    ctx.scale(dpr, dpr)
    ctx.clearRect(0, 0, cssW, cssH)

    const pal = PALETTES[activePalette.value]
    const s = DENSITY
    const filter = activeFilter.value
    const lMap = heroLuminanceMap

    if (!lMap) {
      ctx.restore()
      showcaseAnimId = requestAnimationFrame(renderShowcase)
      return
    }

    // --- MODE 1: PARTICLE SCANLINE (Default — Sculptural Particle Lattice + Vertical Combing) ---
    if (filter === 'scanline') {
      const rowPitch = 4.2
      const numRows = Math.ceil(cssH / rowPitch)

      // 1. Update spring physics on vertical optical columns
      for (const col of scanlineCols) {
        if (showcaseMouse.isInside) {
          const dx = col.x - showcaseMouse.x
          const dist = Math.abs(dx)
          if (dist < 76) {
            const push = (1 - dist / 76) * (dx >= 0 ? 11 : -11)
            col.vx += push * 0.22
          }
        }
        col.vx += (col.origX - col.x) * 0.15
        col.vx *= 0.78
        col.x += col.vx
      }

      // 2. Render whisper-faint full-height vertical optical carrier lines (Cloudflare style)
      ctx.lineWidth = 0.5
      for (let i = 0; i < scanlineCols.length; i++) {
        const col = scanlineCols[i]
        const edgeX = Math.abs(col.origX / cssW - 0.5) * 2
        const alpha = 0.055 * (1 - Math.pow(edgeX, 2) * 0.7)
        ctx.strokeStyle = samplePaletteColor(pal, 0.25, alpha)
        ctx.beginPath()
        ctx.moveTo(col.x, 18)
        ctx.lineTo(col.x, cssH - 18)
        ctx.stroke()
      }

      // 3. Render quantized particle segments along the chiseled 3D "K" field
      for (let i = 0; i < scanlineCols.length; i++) {
        const col = scanlineCols[i]
        for (let r = 4; r < numRows - 4; r++) {
          const midY = r * rowPitch + rowPitch * 0.5
          const L = lMap.getL(col.origX, midY)

          if (L > 0.045) {
            // Subtle vertical harmonic shimmer along each fiber
            const wave = Math.sin(showcaseTime * 2.2 + col.phase + r * 0.18) * 0.06
            const tone = Math.max(0, Math.min(1, L + wave))

            // Radial cursor repulsion in 2D for individual particles inside the K
            let px = col.x
            let py = midY
            if (showcaseMouse.isInside) {
              const dx = px - showcaseMouse.x
              const dy = py - showcaseMouse.y
              const dist = Math.sqrt(dx * dx + dy * dy)
              if (dist < 68) {
                const f = Math.pow(1 - dist / 68, 1.6) * 10
                px += (dx / (dist || 1)) * f
                py += (dy / (dist || 1)) * f
              }
            }

            // High-luminance ridges render as crisp vertical micro-bars;
            // mid/penumbra zones render as quantized retro-tech phosphor dots
            const alpha = Math.min(1, 0.22 + tone * 0.82)
            ctx.fillStyle = samplePaletteColor(pal, tone, alpha)

            if (tone > 0.52) {
              const wBar = 1.1 + tone * 2.1
              const hBar = 2.0 + tone * 1.4
              ctx.fillRect(px - wBar * 0.5, py - hBar * 0.5, wBar, hBar)
            } else {
              const sz = 1.1 + tone * 1.8
              ctx.fillRect(px - sz * 0.5, py - sz * 0.5, sz, sz)
            }
          }
        }
      }

      // 4. Render free-floating detached memory particles streaming off the K facets
      for (const pt of floatingParticles) {
        const driftX = Math.sin(showcaseTime * pt.speed + pt.phase) * 7 + Math.cos(showcaseTime * 0.7 + pt.phase) * 4
        const driftY = Math.cos(showcaseTime * pt.speed * 0.85 + pt.phase) * 6
        const targetX = pt.baseX + driftX
        const targetY = pt.baseY + driftY

        if (showcaseMouse.isInside) {
          const dx = pt.x - showcaseMouse.x
          const dy = pt.y - showcaseMouse.y
          const dist = Math.sqrt(dx * dx + dy * dy)
          if (dist < 85) {
            const push = (1 - dist / 85) * 3.2
            pt.vx += (dx / (dist || 1)) * push
            pt.vy += (dy / (dist || 1)) * push
          }
        }

        pt.vx += (targetX - pt.x) * 0.10
        pt.vy += (targetY - pt.y) * 0.10
        pt.vx *= 0.82
        pt.vy *= 0.82
        pt.x += pt.vx
        pt.y += pt.vy

        const pulse = 0.45 + 0.55 * Math.sin(showcaseTime * 2.4 + pt.phase)
        ctx.fillStyle = samplePaletteColor(pal, pt.tone, 0.35 + pulse * 0.55)
        ctx.fillRect(pt.x - pt.size * 0.5, pt.y - pt.size * 0.5, pt.size, pt.size)
      }

      renderOpticalRegistration(ctx, pal, cssW, cssH)
    }

    // --- MODE 2: ASCII (@Kubai087 High-Density Volumetric Typographic Engraving) ---
    else if (filter === 'ascii') {
      const baseCellW = 5.6
      const baseCellH = 6.6
      const cols = Math.floor((cssW - 24) / baseCellW)
      const rows = Math.floor((cssH - 24) / baseCellH)
      const offsetX = (cssW - cols * baseCellW) * 0.5
      const offsetY = (cssH - rows * baseCellH) * 0.5

      ctx.font = '700 7.5px "JetBrains Mono", ui-monospace, monospace'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'

      for (let r = 0; r < rows; r++) {
        const cy = offsetY + r * baseCellH + baseCellH * 0.5
        for (let c = 0; c < cols; c++) {
          const cx = offsetX + c * baseCellW + baseCellW * 0.5

          let drawX = cx
          let drawY = cy
          if (showcaseMouse.isInside) {
            const dx = cx - showcaseMouse.x
            const dy = cy - showcaseMouse.y
            const dist = Math.sqrt(dx * dx + dy * dy)
            if (dist < 78) {
              const push = Math.pow(1 - dist / 78, 1.5) * 9
              drawX += (dx / (dist || 1)) * push
              drawY += (dy / (dist || 1)) * push
            }
          }

          const rawL = lMap.getL(cx, cy)
          if (rawL > 0.04) {
            // Subtle organic shimmer across the ASCII character ramp
            const shimmer = Math.sin(showcaseTime * 2.0 + c * 0.22 - r * 0.16) * 0.04
            const lev = Math.max(0, Math.min(1, (rawL - 0.04) / 0.94 + shimmer))
            const tone = lev * (1 - s) + smoothstep(lev) * s
            const charIdx = Math.max(2, Math.min(ASCII_CHARS.length - 1, Math.floor(tone * (ASCII_CHARS.length - 1))))
            const ch = ASCII_CHARS[charIdx]
            const alpha = 0.28 + 0.72 * tone
            ctx.fillStyle = samplePaletteColor(pal, tone, alpha)
            ctx.fillText(ch, drawX, drawY)
          } else if ((r % 3 === 0) && (c % 3 === 0)) {
            // Whisper-subtle background matrix dots
            ctx.fillStyle = samplePaletteColor(pal, 0.18, 0.09)
            ctx.fillText('·', drawX, drawY)
          }
        }
      }

      renderOpticalRegistration(ctx, pal, cssW, cssH)
    }

    // --- MODE 3: DITHER (@Kubai087 Retro Bayer 4x4 Ordered Lithograph) ---
    else if (filter === 'dither') {
      const pixelSize = 3
      const cols = Math.ceil(cssW / pixelSize)
      const rows = Math.ceil(cssH / pixelSize)

      for (let r = 0; r < rows; r++) {
        const by = r * pixelSize
        const my = r % 4

        for (let c = 0; c < cols; c++) {
          const bx = c * pixelSize
          const mx = c % 4

          let sampleX = bx + pixelSize * 0.5
          let sampleY = by + pixelSize * 0.5
          if (showcaseMouse.isInside) {
            const dx = sampleX - showcaseMouse.x
            const dy = sampleY - showcaseMouse.y
            const dist = Math.sqrt(dx * dx + dy * dy)
            if (dist < 75) {
              const warp = (1 - dist / 75) * 10
              sampleX -= (dx / (dist || 1)) * warp
              sampleY -= (dy / (dist || 1)) * warp
            }
          }

          const L = lMap.getL(sampleX, sampleY)
          if (L <= 0.03) continue

          const threshold = (BAYER_4X4[my][mx] + 0.5) / 16
          const adjustedL = Math.max(0, Math.min(1, L + (threshold - 0.5) * 0.34))
          if (adjustedL <= 0.12) continue

          let stopIdx = 0
          if (adjustedL > 0.72) stopIdx = 3
          else if (adjustedL > 0.46) stopIdx = 2
          else if (adjustedL > 0.22) stopIdx = 1

          ctx.fillStyle = pal.stops[stopIdx]
          ctx.fillRect(bx, by, pixelSize - 0.3, pixelSize - 0.3)
        }
      }

      renderOpticalRegistration(ctx, pal, cssW, cssH)
    }

    ctx.restore()
    showcaseAnimId = requestAnimationFrame(renderShowcase)
  }

  renderShowcase()

  return () => {
    canvas.removeEventListener('mousemove', onMouseMove)
    canvas.removeEventListener('mouseleave', onMouseLeave)
    cancelAnimationFrame(showcaseAnimId)
  }
}

// --- Setup .VPFeature Card Watermarks (116x52 strictly in top-right header) ---
function setupFeatureCards() {
  featureCleanups.forEach((cleanup) => cleanup())
  featureCleanups = []

  const cards = document.querySelectorAll<HTMLElement>('.VPFeature')
  if (!cards || cards.length === 0) return

  cards.forEach((card, idx) => {
    if (idx >= 6) return

    // Position container for absolute watermark
    card.style.position = 'relative'
    card.style.overflow = 'hidden'

    // Prevent duplicate canvases
    const existing = card.querySelector('.vp-feature-ascii-canvas')
    if (existing) existing.remove()

    const canvas = document.createElement('canvas')
    canvas.className = 'vp-feature-ascii-canvas'
    const cssW = 116
    const cssH = 52
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    canvas.width = cssW * dpr
    canvas.height = cssH * dpr
    canvas.style.cssText = `
      position: absolute;
      top: 10px;
      right: 12px;
      width: ${cssW}px;
      height: ${cssH}px;
      pointer-events: none;
      z-index: 1;
      opacity: 0.65;
      transition: opacity 0.3s ease;
    `
    card.appendChild(canvas)

    // Build or reuse luminance map
    if (!featureLuminanceMaps[idx]) {
      featureLuminanceMaps[idx] = buildLuminanceMap(
        cssW,
        cssH,
        FEATURE_EMBLEM_DRAWERS[idx] || drawEmblem01
      )
    }
    const lMap = featureLuminanceMaps[idx]!

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let isHovered = false
    let cardMouseX = -999
    let cardMouseY = -999
    let animId = 0
    let fTime = Math.random() * 10

    // Hover listeners
    const onEnter = () => {
      isHovered = true
      canvas.style.opacity = '1.0'
    }
    const onLeave = () => {
      isHovered = false
      cardMouseX = -999
      cardMouseY = -999
      canvas.style.opacity = '0.65'
    }
    const onMove = (e: MouseEvent) => {
      const rect = card.getBoundingClientRect()
      cardMouseX = e.clientX - (rect.right - cssW - 12)
      cardMouseY = e.clientY - (rect.top + 10)
    }

    card.addEventListener('mouseenter', onEnter)
    card.addEventListener('mouseleave', onLeave)
    card.addEventListener('mousemove', onMove)

    const featureTags = [
      '01 // 3.4MS',
      '02 // BYOM',
      '03 // RRF-60',
      '04 // <10MS',
      '05 // MCP',
      '06 // A100',
    ]

    const numCols = 16
    const colSpacing = cssW / numCols

    const renderCard = () => {
      fTime += 0.025
      ctx.save()
      ctx.scale(dpr, dpr)
      ctx.clearRect(0, 0, cssW, cssH)

      const pal = PALETTES[activePalette.value]
      const segH = 4.5
      const numRows = Math.ceil(cssH / segH)

      for (let c = 0; c < numCols; c++) {
        const origX = c * colSpacing + colSpacing * 0.5
        let curX = origX

        // Mouse scatter physics
        if (isHovered && cardMouseX > -30 && cardMouseX < cssW + 30) {
          const dx = origX - cardMouseX
          if (Math.abs(dx) < 35) {
            curX += (dx >= 0 ? 1 : -1) * (1 - Math.abs(dx) / 35) * 6
          }
        }

        for (let r = 0; r < numRows; r++) {
          const y1 = r * segH
          const y2 = y1 + segH - 1.0
          const L = lMap.getL(origX, y1 + segH * 0.5)

          if (L > 0.08) {
            ctx.lineWidth = 0.8 + L * 2.2
            ctx.strokeStyle = samplePaletteColor(pal, L, isHovered ? 0.95 : 0.6)
            ctx.beginPath()
            ctx.moveTo(curX, y1)
            ctx.lineTo(curX, y2)
            ctx.stroke()
          }
        }
      }

      // Detection box on hover
      if (isHovered) {
        const bx = 6
        const by = 6
        const bw = cssW - 12
        const bh = cssH - 12

        ctx.strokeStyle = samplePaletteColor(pal, 0.74, 0.85)
        ctx.setLineDash([2, 2])
        ctx.strokeRect(bx, by, bw, bh)
        ctx.setLineDash([])

        // 4 corner ticks
        const t = 3.5
        ctx.strokeStyle = samplePaletteColor(pal, 1.0, 1.0)
        ctx.lineWidth = 1.4

        ctx.beginPath()
        ctx.moveTo(bx, by + t)
        ctx.lineTo(bx, by)
        ctx.lineTo(bx + t, by)
        ctx.moveTo(bx + bw - t, by)
        ctx.lineTo(bx + bw, by)
        ctx.lineTo(bx + bw, by + t)
        ctx.moveTo(bx, by + bh - t)
        ctx.lineTo(bx, by + bh)
        ctx.lineTo(bx + t, by + bh)
        ctx.moveTo(bx + bw - t, by + bh)
        ctx.lineTo(bx + bw, by + bh)
        ctx.lineTo(bx + bw, by + bh - t)
        ctx.stroke()

        // Tag
        ctx.font = '700 7.5px "JetBrains Mono", ui-monospace, monospace'
        ctx.fillStyle = samplePaletteColor(pal, 1.0, 0.95)
        ctx.fillText(featureTags[idx] || 'DET', bx + 4, by + 8)
      }

      ctx.restore()
      animId = requestAnimationFrame(renderCard)
    }

    renderCard()

    featureCleanups.push(() => {
      card.removeEventListener('mouseenter', onEnter)
      card.removeEventListener('mouseleave', onLeave)
      card.removeEventListener('mousemove', onMove)
      cancelAnimationFrame(animId)
      canvas.remove()
    })
  })
}

// --- Mount & Route Observers ---
const route = useRoute()
let cleanupBg: (() => void) | undefined
let cleanupShowcase: (() => void) | undefined

function locateHeroContainer() {
  if (typeof window === 'undefined' || typeof document === 'undefined') return
  heroContainer.value = document.querySelector<HTMLElement>('.VPHero .container')
  heroObserver?.disconnect()
  isHeroInViewport = true
  const vpHero = document.querySelector<HTMLElement>('.VPHero')
  if (vpHero && 'IntersectionObserver' in window) {
    heroObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          isHeroInViewport = entry.isIntersecting
        }
      },
      { threshold: 0.01 }
    )
    heroObserver.observe(vpHero)
  }
}

async function initAll() {
  if (typeof window === 'undefined' || typeof document === 'undefined') return
  locateHeroContainer()
  await nextTick()
  if (props.mode === 'all' || props.mode === 'bg') {
    cleanupBg?.()
    cleanupBg = initBgCanvas()
  }
  if (props.mode === 'all' || props.mode === 'showcase') {
    cleanupShowcase?.()
    cleanupShowcase = initShowcaseCanvas()
  }
  setupFeatureCards()
}

onMounted(() => {
  isMounted.value = true
  nextTick(() => {
    initAll()
  })
})

watch(
  () => route.path,
  () => {
    nextTick(() => {
      initAll()
    })
  }
)

onBeforeUnmount(() => {
  heroObserver?.disconnect()
  heroObserver = null
  cleanupBg?.()
  cleanupShowcase?.()
  featureCleanups.forEach((c) => c())
  featureCleanups = []
})
</script>

<template>
  <div class="k0-ascii-hero-engine-wrapper">
    <!-- 1. Ambient Background ASCII Canvas (Behind Hero) -->
    <div
      v-if="mode === 'all' || mode === 'bg'"
      class="ascii-hero-bg-container"
    >
      <canvas
        ref="bgCanvasRef"
        class="ascii-hero-bg-canvas"
      />
    </div>

    <!-- 2. Split-Hero Interactive Showcase Panel (Mounted into .VPHero .container) -->
    <Teleport
      v-if="(mode === 'all' || mode === 'showcase') && isMounted && heroContainer"
      :to="heroContainer"
    >
      <div
        class="ascii-showcase-panel"
        style="order: 2; width: 100%; max-width: 480px; justify-self: end;"
      >
        <!-- Pure Frameless Sculptural Canvas Stage -->
        <div class="showcase-canvas-wrapper">
          <canvas
            ref="showcaseCanvasRef"
            class="showcase-canvas"
          />
        </div>

        <!-- Whisper-Quiet Minimalist Retro-Tech Footer Bar -->
        <div class="showcase-minimal-bar">
          <div class="filter-segmented-group">
            <button
              type="button"
              class="filter-btn"
              :class="{ active: activeFilter === 'scanline' }"
              @click="activeFilter = 'scanline'"
            >
              PARTICLE
            </button>
            <button
              type="button"
              class="filter-btn"
              :class="{ active: activeFilter === 'ascii' }"
              @click="activeFilter = 'ascii'"
            >
              ASCII
            </button>
            <button
              type="button"
              class="filter-btn"
              :class="{ active: activeFilter === 'dither' }"
              @click="activeFilter = 'dither'"
            >
              DITHER
            </button>
          </div>

          <!-- 4-Stop Low-Saturation Swatches -->
          <div class="palette-swatches-group">
            <button
              v-for="palKey in (['meadow', 'mono', 'amber', 'blue'] as PaletteName[])"
              :key="palKey"
              type="button"
              class="swatch-btn"
              :class="[
                `swatch-${palKey}`,
                { active: activePalette === palKey }
              ]"
              :title="`Palette: ${PALETTES[palKey].label}`"
              @click="activePalette = palKey"
            />
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.k0-ascii-hero-engine-wrapper {
  display: contents;
}

/* 1. Ambient Background Behind Hero */
.ascii-hero-bg-container {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 0;
  overflow: hidden;
}

.ascii-hero-bg-canvas {
  display: block;
  width: 100%;
  height: 100%;
}

/* 2. Container and Main Layout Coordination */
:global(.VPHero .container) {
  display: flex !important;
  flex-direction: row !important;
  justify-content: space-between !important;
  align-items: center !important;
  gap: 32px !important;
}

:global(.VPHero .main) {
  order: 1 !important;
  flex: 1 1 520px !important;
  max-width: 600px !important;
}

/* 3. Split Showcase Panel (Strictly Order 2 on Right) */
.ascii-showcase-panel {
  position: relative;
  order: 2 !important;
  width: 100% !important;
  max-width: 480px !important;
  flex: 0 0 480px !important;
  justify-self: end;
  margin-left: auto;
  background: #0B0D0A;
  border: 1px solid rgba(174, 186, 120, 0.15);
  border-radius: 6px;
  box-shadow: 0 20px 50px -16px rgba(0, 0, 0, 0.72);
  z-index: 2;
  overflow: hidden;
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
}

@media (max-width: 960px) {
  :global(.VPHero .container) {
    flex-direction: column !important;
  }
  :global(.VPHero .main) {
    order: 1 !important;
    max-width: 100% !important;
  }
  .ascii-showcase-panel {
    order: 2 !important;
    max-width: 100% !important;
    width: 100% !important;
    flex: 1 1 100% !important;
    margin-left: 0 !important;
    margin-top: 28px !important;
  }
}

/* Canvas Area */
.showcase-canvas-wrapper {
  position: relative;
  width: 100%;
  height: 340px;
  background: radial-gradient(circle at 48% 50%, #11150E 0%, #0A0C09 100%);
  overflow: hidden;
}

.showcase-canvas {
  display: block;
  width: 100%;
  height: 100%;
  cursor: crosshair;
}

/* Whisper-Quiet Minimalist Footer Bar */
.showcase-minimal-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 14px;
  background: #090B08;
  border-top: 1px solid rgba(174, 186, 120, 0.11);
}

.filter-segmented-group {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.filter-btn {
  background: transparent;
  border: none;
  color: #666E58;
  font-family: inherit;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.08em;
  padding: 3px 8px;
  border-radius: 3px;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}

.filter-btn:hover {
  color: #AEBA78;
}

.filter-btn.active {
  background: rgba(103, 125, 69, 0.18);
  color: #E5E1BD;
}

/* Palette Swatches */
.palette-swatches-group {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}

.swatch-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.16);
  cursor: pointer;
  padding: 0;
  outline: none;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}
.swatch-btn:hover {
  transform: scale(1.15);
}

.swatch-btn.active {
  box-shadow:
    0 0 0 1.5px #090B08,
    0 0 0 2.5px #AEBA78;
}

.swatch-meadow {
  background: linear-gradient(135deg, #677D45 0%, #AEBA78 100%);
}

.swatch-mono {
  background: linear-gradient(135deg, #575757 0%, #A6A6A6 100%);
}

.swatch-amber {
  background: linear-gradient(135deg, #A74C15 0%, #EF9A46 100%);
}

.swatch-blue {
  background: linear-gradient(135deg, #326AC5 0%, #A2C2F1 100%);
}
</style>
